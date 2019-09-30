use ert::prelude::*;
use futures::stream::iter_ok;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{prelude::*, runtime::Runtime};

#[test]
fn ids() {
    let mut rt = Runtime::new().unwrap();

    let map = Arc::new(Mutex::new(HashMap::new()));

    let f = iter_ok(0usize..10000)
        .map(move |i| {
            let tag = i % 1000;
            let map2 = map.clone();
            Ok::<_, ()>(())
                .into_future()
                .map(move |_| {
                    let mut map2 = map2.lock().unwrap();
                    let id = map2.entry(tag).or_insert(worker_id());
                    assert_eq!(worker_id(), *id);
                })
                .via_g(tag)
        })
        .buffered(100)
        .for_each(|_| Ok(()));

    rt.block_on(f).unwrap();
}
