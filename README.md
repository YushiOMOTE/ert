# ert

A combinator to control future execution order.

[![Latest version](https://img.shields.io/crates/v/ert.svg)](https://crates.io/crates/ert)
[![Documentation](https://docs.rs/ert/badge.svg)](https://docs.rs/ert)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Actions Status](https://github.com/YushiOMOTE/ert/workflows/Rust/badge.svg)](https://github.com/YushiOMOTE/ert/actions)


```rust
struct Data {
    file: String,
    tag: usize,
    value: usize,
}

#[tokio::main]
async fn main() {
    // Stream of `Data` coming over TCP.
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
```

![order](https://raw.github.com/wiki/YushiOMOTE/ert/assets/order.png)
