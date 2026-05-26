# schema-rust-next

`schema-rust-next` emits Rust source code from `schema-next`'s assembled
schema.

This repository is deliberately not a Rust macro crate. The MVP path is:
assembled schema in, Rust source text out, compile and test the emitted source,
then layer macro ergonomics later.
