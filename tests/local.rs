mod helper;

use self::helper::Checker;
use ert::prelude::*;
use futures::prelude::*;

#[tokio::test]
async fn local() {
    let r = Router::new(100);

    let c = Checker::new();
    let c1 = c.clone();
    let r1 = r.clone();
    let futs: Vec<_> = (0u64..100)
        .map(move |i| {
            let r2 = r1.clone();
            let c1 = c1.clone();
            async move {
                c1.add(i);
            }
            .via(r2.clone(), i)
        })
        .collect();

    let r1 = r.clone();
    let c2 = c.clone();
    let futs2: Vec<_> = (0u64..10000)
        .map(move |i| {
            let i = i % 100;
            let r2 = r1.clone();
            let c2 = c2.clone();
            async move {
                c2.check(i);
            }
            .via(r2.clone(), i)
        })
        .collect();

    let f = futures::future::join_all(futs).then(|_| futures::future::join_all(futs2));

    f.await;
}
