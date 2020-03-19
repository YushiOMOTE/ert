mod helper;

use self::helper::Checker;
use ert::prelude::*;
use futures::prelude::*;

#[tokio::test]
async fn global() {
    Router::new(100).set_as_global();

    let c = Checker::new();

    let c1 = c.clone();
    let futs: Vec<_> = (0u64..100)
        .map(move |i| {
            let c1 = c1.clone();
            async move {
                c1.add(i);
            }
            .via_g(i)
        })
        .collect();

    let c2 = c.clone();
    let futs2: Vec<_> = (0u64..10000)
        .map(move |i| {
            let i = i % 100;
            let c2 = c2.clone();
            async move {
                c2.check(i);
            }
            .via_g(i)
        })
        .collect();

    let f = futures::future::join_all(futs).then(|_| futures::future::join_all(futs2));

    f.await;
}
