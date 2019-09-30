use crate::router::{Router, Via};
use futures::prelude::*;
use std::hash::Hash;

///
/// Future combinator to setup sequential execution.
///
pub trait RunVia<T, E> {
    ///
    /// Sequentially evaluate a future in a worker which is associated with `key`,
    /// using `router`.
    ///
    fn via<K: Hash + 'static>(self, router: Router, key: K) -> Via<T, E>;

    ///
    /// Sequentially evaluate a future in a worker which is associated with `key`
    /// in a global router.
    ///
    fn via_g<K: Hash + 'static>(self, key: K) -> Via<T, E>;
}

impl<U> RunVia<U::Item, U::Error> for U
where
    U: Future + Send + 'static,
    U::Item: Send + 'static,
    U::Error: Send + 'static,
{
    fn via<K>(self, router: Router, key: K) -> Via<U::Item, U::Error>
    where
        K: Hash + 'static,
    {
        router.via(key, move || self)
    }

    fn via_g<K>(self, key: K) -> Via<U::Item, U::Error>
    where
        K: Hash + 'static,
    {
        let res = Router::with_global(move |r| match r {
            Some(r) => Ok(r.via(key, move || self)),
            None => Err((self, key)),
        });

        match res {
            Ok(via) => via,
            Err((s, key)) => {
                Router::run_on_thread(1024).set_as_global();
                Router::with_global(|r| r.unwrap().via(key, move || s))
            }
        }
    }
}
