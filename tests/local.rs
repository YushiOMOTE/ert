use ert::prelude::*;
use futures::task::current;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::{prelude::*, runtime::Runtime};

#[test]
fn local() {
    let mut rt = Runtime::new().unwrap();
    let r = Router::new(rt.executor(), 100);

    let map = Arc::new(Mutex::new(HashMap::new()));

    let r1 = r.clone();
    let map1 = map.clone();
    let futs: Vec<_> = (0u64..100)
        .map(move |i| {
            let r2 = r1.clone();
            let map2 = map1.clone();
            Ok::<_, ()>(())
                .into_future()
                .map(move |_| {
                    map2.lock().unwrap().insert(i, current());
                })
                .via(r2.clone(), i)
        })
        .collect();

    let r1 = r.clone();
    let futs2: Vec<_> = (0u64..10000)
        .map(move |i| {
            let i = i % 100;
            let r2 = r1.clone();
            let map2 = map.clone();
            Ok::<_, ()>(())
                .into_future()
                .map(move |_| {
                    assert_eq!(map2.lock().unwrap().get(&i).unwrap().is_current(), true);
                })
                .via(r2.clone(), i)
        })
        .collect();

    let f = futures::future::join_all(futs).and_then(|_| futures::future::join_all(futs2));

    rt.block_on(f).unwrap();
}
