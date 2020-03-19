use ert::prelude::*;
use rand::Rng;
use std::time::{Duration, Instant};
use futures::prelude::*;
use futures::stream;
use tokio::{fs::File, prelude::*, time::delay_for};

struct Data {
    file: String,
    tag: usize,
    value: usize,
}

async fn delay() {
    let i = rand::thread_rng().gen_range(100, 500);
    delay_for(Duration::from_micros(i)).await
}

async fn read_value_from_file(f: &str) -> usize {
    println!("Read from {}", f);

    let v = async move {
        let mut f = File::open(f.to_string()).await.ok()?;
        let mut contents = vec![];
        f.read_to_end(&mut contents).await.ok()?;

        let s = std::str::from_utf8(&contents).ok()?;
        let v = s.parse().ok()?;

        Some(v)
    };

    let v = v.await.unwrap_or(0);

    delay().await;

    v
}

async fn write_value_to_file(f: &str, value: usize) {
    println!("Write {} to {}", f, value);
    let mut f = File::create(f.to_string()).await.unwrap();
    f.write_all(&value.to_string().as_bytes()).await.unwrap();
    f.flush().await.unwrap();
}

// tcp incoming stream mock
fn tcp_stream() -> impl Stream<Item = Data> {
    stream::iter(0..1000).map(|i| {
        let i = i % 10;

        Data {
            file: format!("file{}.txt", i),
            tag: i,
            value: 1,
        }
    })
}

#[tokio::main]
async fn main() {
    let f = tcp_stream()
        .map(|d| {
            let tag = d.tag;
            let f = async move {
                let v = read_value_from_file(&d.file).await;
                write_value_to_file(&d.file, v + d.value).await;
            };
            f.via_g(tag)
        })
        .buffer_unordered(100)
        .for_each(|_| async {});

    f.await;
}
