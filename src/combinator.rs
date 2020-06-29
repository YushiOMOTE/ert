use crate::router::{Router, Via};
use futures::prelude::*;
use std::hash::Hash;

pub trait RunVia<T> {
    fn via<K: Hash + 'static>(self, router: Router, key: K) -> Via<T>;

    fn via_g<K: Hash + 'static>(self, key: K) -> Via<T>;
}

impl<U> RunVia<U::Output> for U
where
    U: Future + Send + 'static,
    U::Output: Send + 'static,
{
    fn via<K>(self, router: Router, key: K) -> Via<U::Output>
    where
        K: Hash + 'static,
    {
        router.via(key, move || self)
    }

    fn via_g<K>(self, key: K) -> Via<U::Output>
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
                Router::with_global_mut(|r| {
                    if r.is_none() {
                        *r = Some(Router::run_on_thread(1024));
                    }
                });
                Router::with_global(|r| r.unwrap().via(key, move || s))
            }
        }
    }
}
