use crate::utils::set_worker_id;

use futures::{
    future::lazy,
    prelude::*,
    sync::{mpsc, oneshot},
};
use log::*;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::{Arc, RwLock},
};
use tokio::runtime::{Runtime, TaskExecutor};

type BoxedFuture = Box<dyn Future<Item = (), Error = ()> + Send + 'static>;

lazy_static::lazy_static! {
    static ref GLOBAL_ROUTER: RwLock<Option<Router>> = { RwLock::new(None) };
}

///
/// Router to provide sequential execution of certain set of tasks.
///
#[derive(Clone)]
pub struct Router {
    tx: Arc<Vec<mpsc::UnboundedSender<BoxedFuture>>>,
}

pub struct Via<T, E>(oneshot::Receiver<Result<T, E>>);

impl<T, E> Via<T, E> {
    fn new<F, R>(tx: &mpsc::UnboundedSender<BoxedFuture>, f: F) -> Self
    where
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce() -> R,
        R: IntoFuture<Item = T, Error = E>,
        R::Future: Send + 'static,
    {
        let (otx, orx) = oneshot::channel();

        let fut = Box::new(f().into_future().then(move |r| {
            let _ = otx.send(r);
            Ok(())
        }));

        if tx.unbounded_send(fut).is_err() {
            warn!("Couldn't send future to router; the future will never be resolved");
        }

        Self(orx)
    }
}

impl<T, E> Future for Via<T, E> {
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.0.poll() {
            Ok(Async::Ready(res)) => res.map(|ok| Async::Ready(ok)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(_) => {
                warn!("Router isn't running; this future will never be resolved");
                Ok(Async::NotReady)
            }
        }
    }
}

impl Router {
    ///
    /// Create a new router.
    ///
    /// The created router object can be passed to `.via()` combinator.
    ///
    /// ```
    /// use ert::prelude::*;
    /// use futures::prelude::*;
    ///
    /// let mut rt = tokio::runtime::Runtime::new().unwrap();
    ///
    /// let router = Router::new(rt.executor(), 1000);
    ///
    /// let f = futures::stream::iter_ok::<_, ()>(0..1000)
    ///           .map(move |i| {
    ///               futures::future::ok(i)
    ///                   .via(router.clone(), i)
    ///           })
    ///           .buffered(100)
    ///           .for_each(|_| Ok(()));
    ///
    /// rt.block_on(f);
    /// ```
    ///
    pub fn new(e: TaskExecutor, workers: usize) -> Self {
        if workers == 0 {
            panic!("Invalid number of workers: {}", workers);
        }

        let tx = (0..workers)
            .map(|wid| {
                let (tx, rx) = mpsc::unbounded();
                let _ = e.spawn(lazy(move || {
                    set_worker_id(wid);
                    rx.for_each(|t| t)
                }));
                tx
            })
            .collect();
        let tx = Arc::new(tx);

        Self { tx }
    }

    ///
    /// Create a new router with a background thread which executes the router.
    ///
    /// This is to hide the boilerplate to create the tokio runtime.
    /// The created router is set as a global router, which can be used by `.via_g()` combinator.
    /// Hence, users don't have to pass around handles to a specific router instance.
    ///
    /// ```
    /// use ert::prelude::*;
    /// use futures::prelude::*;
    ///
    /// Router::run_on_thread(1000);
    ///
    /// let f = futures::stream::iter_ok::<_, ()>(0..1000)
    ///           .map(move |i| {
    ///               futures::future::ok(i)
    ///                   .via_g(i)
    ///           })
    ///           .buffered(100)
    ///           .for_each(|_| Ok(()));
    ///
    /// tokio::run(f);
    /// ```
    ///
    pub fn run_on_thread(workers: usize) -> Self {
        let rt = Runtime::new().unwrap();

        let router = Self::new(rt.executor(), workers);

        std::thread::spawn(|| {
            rt.shutdown_on_idle().wait().unwrap();
        });

        router
    }

    ///
    /// Set a router as a global router.
    ///
    /// The created router can be used by `.via_g()` combinator.
    /// Hence, users don't have to pass around handles to a specific router instance.
    ///
    /// ```
    /// use ert::prelude::*;
    /// use futures::prelude::*;
    ///
    /// let mut rt = tokio::runtime::Runtime::new().unwrap();
    ///
    /// let router = Router::new(rt.executor(), 1000);
    /// router.set_as_global();
    ///
    /// let f = futures::stream::iter_ok::<_, ()>(0..1000)
    ///           .map(move |i| {
    ///               futures::future::ok(i)
    ///                   .via_g(i)
    ///           })
    ///           .buffered(100)
    ///           .for_each(|_| Ok(()));
    ///
    /// rt.block_on(f);
    /// ```
    ///
    pub fn set_as_global(self) {
        *GLOBAL_ROUTER.write().unwrap() = Some(self);
    }

    ///
    /// Helper method to access to a global router.
    ///
    /// ```
    /// use ert::Router;
    ///
    /// let rt = tokio::runtime::Runtime::new().unwrap();
    /// let router = Router::new(rt.executor(), 1000);
    /// router.set_as_global();
    ///
    /// Router::with_global(|global_router| {
    ///    // Do something
    /// });
    /// ```
    ///
    pub fn with_global<F, R>(f: F) -> R
    where
        F: FnOnce(Option<&Router>) -> R,
    {
        f(GLOBAL_ROUTER.read().unwrap().as_ref())
    }

    ///
    /// Send a logic to a worker in a router.
    ///
    /// ```
    /// use ert::prelude::*;
    /// use futures::prelude::*;
    ///
    /// let mut rt = tokio::runtime::Runtime::new().unwrap();
    ///
    /// let router = Router::new(rt.executor(), 1000);
    ///
    /// let f = futures::stream::iter_ok::<_, ()>(0..1000)
    ///           .map(move |i| {
    ///               router.via(i, || Ok(()))
    ///           })
    ///           .buffered(100)
    ///           .for_each(|_| Ok(()));
    ///
    /// rt.block_on(f);
    /// ```
    ///
    pub fn via<K, F, T, E, R>(&self, key: K, f: F) -> Via<T, E>
    where
        K: Hash,
        T: Send + 'static,
        E: Send + 'static,
        F: FnOnce() -> R,
        R: IntoFuture<Item = T, Error = E>,
        R::Future: Send + 'static,
    {
        let h = {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            hasher.finish() as usize
        };

        Via::new(&self.tx[h % self.tx.len()], f)
    }
}
