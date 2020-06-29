use futures::{future::BoxFuture, prelude::*};
use log::*;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::DerefMut,
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};
use tokio::{
    runtime::Builder,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    task_local,
};

type Sender = mpsc::UnboundedSender<BoxFuture<'static, ()>>;
type Receiver = mpsc::UnboundedReceiver<BoxFuture<'static, ()>>;

task_local! {
    pub static ID: u64;
}

/// Helper to return the current worker id
pub fn current() -> u64 {
    ID.get()
}

lazy_static::lazy_static! {
    static ref GLOBAL_ROUTER: RwLock<Option<Router>> = { RwLock::new(None) };
}

#[derive(Clone)]
pub struct Router {
    tx: Arc<Vec<Sender>>,
}

pub struct Via<T>(oneshot::Receiver<T>);

impl<T> Via<T> {
    fn new<F, R>(tx: &Sender, f: F) -> Self
    where
        T: Send + 'static,
        F: FnOnce() -> R,
        R: Future<Output = T> + Send + 'static,
    {
        let (otx, orx) = oneshot::channel();

        let fut = f()
            .then(move |r| async move {
                let _ = otx.send(r);
            })
            .boxed();

        if tx.send(fut).is_err() {
            warn!("Couldn't send future to router; the future will never be resolved");
        }

        Self(orx)
    }
}

impl<T> Future for Via<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match self.0.poll_unpin(cx) {
            Poll::Ready(Ok(output)) => Poll::Ready(output),
            Poll::Ready(Err(_)) | Poll::Pending => {
                // Oneshot cancelled, meaning this feature never gets resolved
                Poll::Pending
            }
        }
    }
}

fn open(workers: usize) -> (Vec<Sender>, Vec<Receiver>) {
    (0..workers).map(|_| mpsc::unbounded_channel()).unzip()
}

fn run(rxs: Vec<Receiver>) -> Vec<JoinHandle<()>> {
    rxs.into_iter()
        .enumerate()
        .map(|(i, rx)| {
            tokio::spawn(rx.for_each(move |t| ID.scope(i as u64, async move { t.await })))
        })
        .collect()
}

impl Router {
    pub fn new(workers: usize) -> Self {
        if workers == 0 {
            panic!("Invalid number of workers: {}", workers);
        }

        let (txs, rxs) = open(workers);

        run(rxs);

        Self { tx: Arc::new(txs) }
    }

    pub fn run_on_thread(workers: usize) -> Self {
        let (txs, rxs) = open(workers);

        std::thread::spawn(move || {
            let mut rt = Builder::new()
                .threaded_scheduler()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                if let Err(e) = futures::future::try_join_all(run(rxs)).await {
                    error!("Couldn't join router worker thread successfully: {}", e);
                }
            });
        });

        Self { tx: Arc::new(txs) }
    }

    pub fn set_as_global(self) {
        *GLOBAL_ROUTER.write().unwrap() = Some(self);
    }

    pub fn with_global<F, R>(f: F) -> R
    where
        F: FnOnce(Option<&Router>) -> R,
    {
        f(GLOBAL_ROUTER.read().unwrap().as_ref())
    }

    pub fn with_global_mut<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Option<Router>) -> R,
    {
        f(GLOBAL_ROUTER.write().unwrap().deref_mut())
    }

    pub fn via<K, F, T, R>(&self, key: K, f: F) -> Via<T>
    where
        K: Hash,
        T: Send + 'static,
        F: FnOnce() -> R,
        R: Future<Output = T> + Send + 'static,
    {
        let h = {
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            hasher.finish() as usize
        };

        Via::new(&self.tx[h % self.tx.len()], f)
    }
}
