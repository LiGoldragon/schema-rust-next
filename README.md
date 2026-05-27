# schema-rust-next

`schema-rust-next` emits Rust source code from `schema-next`'s assembled
schema.

This repository is deliberately not a Rust macro crate. The MVP path is:
assembled schema in, Rust source text out, compile and test the emitted source,
then layer macro ergonomics later.

Generated paths mirror crate-local schema modules. An assembled schema identity
such as `spirit-next:lib` emits to `schema/lib.rs`; an identity such as
`spirit-next:signal:public` emits to `schema/signal/public.rs`. The first
namespace segment is the crate boundary and is not repeated inside the crate's
generated module tree.
