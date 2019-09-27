use ert::prelude::*;
use rand::Rng;
use std::time::{Duration, Instant};
use tokio::{fs::File, prelude::*, timer::Delay};

struct Data {
    file: String,
    tag: usize,
    value: usize,
}

fn delay() -> impl Future<Item = (), Error = ()> {
    let i = rand::thread_rng().gen_range(100, 500);
    Delay::new(Instant::now() + Duration::from_micros(i))
        .map(|_| ())
        .map_err(|_| ())
}

fn read_value_from_file(f: &str) -> impl Future<Item = usize, Error = ()> {
    println!("Read from {}", f);

    File::open(f.to_string())
        .map_err(|_| ())
        .and_then(|mut file| {
            let mut contents = vec![];
            file.read_to_end(&mut contents)
                .map_err(|_| ())
                .and_then(move |_| {
                    Ok(std::str::from_utf8(&contents)
                        .map_err(|_| ())?
                        .parse()
                        .map_err(|_| ())?)
                })
        })
        .or_else(|_| Ok(0))
        .and_then(|v| delay().map(move |_| v))
}

fn write_value_to_file(f: &str, value: usize) -> impl Future<Item = (), Error = ()> {
    println!("Write {} to {}", f, value);

    File::create(f.to_string())
        .and_then(move |mut file| file.poll_write(&value.to_string().as_bytes()))
        .map(|_| ())
        .map_err(|_| ())
}

// tcp incoming stream mock
fn tcp_stream() -> impl Stream<Item = Data, Error = ()> {
    stream::iter_ok(0..1000).map(|i| {
        let i = i % 10;

        Data {
            file: format!("file{}.txt", i),
            tag: i,
            value: 1,
        }
    })
}

fn main() {
    let f = tcp_stream()
        .map(|d| {
            let tag = d.tag;
            Ok(d)
                .into_future()
                .and_then(move |d| read_value_from_file(&d.file).map(move |v| (v, d)))
                .and_then(move |(v, d)| write_value_to_file(&d.file, v + d.value))
                .map_err(|_| ())
                .via_g(tag)
        })
        .buffer_unordered(100)
        .for_each(|_| Ok(()));

    tokio::run(f);
}
