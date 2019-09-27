# ert

A combinator to control future execution order.

```rust
struct Data {
    file: String,
    tag: usize,
    value: usize,
}

fn main() {

    // Stream of `Data` coming over TCP.
    let f = tcp_stream()
        .map(|d| {
            let tag = d.tag;
            Ok(d)
                .into_future()
                .and_then(move |d| read_value_from_file(&d.file).map(move |v| (v, d)))
                .and_then(move |(v, d)| write_value_to_file(&d.file, v + d.value))
                .map_err(|_| ())
                .via_g(tag) // Add execution order constrains
        })
        .buffer_unordered(100)
        .for_each(|_| Ok(()));

    tokio::run(f);
}
```

![order](https://raw.github.com/wiki/YushiOMOTE/ert/assets/order.png)
