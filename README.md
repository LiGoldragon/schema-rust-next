# schema-rust-next

`schema-rust-next` emits Rust source code from `schema-next`'s assembled
schema.

This repository is deliberately not a Rust macro crate. The MVP path is:
assembled schema in, Rust source text out, compile and test the emitted source,
then layer macro ergonomics later.

Generated paths mirror crate-local schema modules. An assembled schema identity
such as `spirit-next:lib` emits to `src/schema/lib.rs`; an identity such as
`spirit-next:signal:public` emits to `src/schema/signal/public.rs`. The first
namespace segment is the crate boundary and is not repeated inside the crate's
generated module tree.

The emitted source includes the data types, NOTA conversion methods, rkyv
derives, short-header signal frames, Nexus traits, Nexus mail lifecycle
objects, mail-event hooks, and upgrade/accept traits that runtime crates
implement against.

Schema imports lower into Rust aliases across generated `src/schema/` modules.
For example, `Proposal horizon-concept:proposal:ClusterProposal` emits a
`pub use crate::schema::proposal::ClusterProposal as Proposal;` bridge plus
the NOTA decode error conversion needed for generated parsers to call across
module boundaries.

For development, `cargo run --example emit_schema -- <schema/lib.schema>
<crate:module> [version]` prints the generated Rust source so a consumer can
refresh its checked-in `src/schema/<module>.rs` from local schema-next and
schema-rust-next changes.

`cargo run --example horizon_concept -- target/horizon-schema-concept` writes a
small Horizon-domain pipeline: authored schema, lowered `Asschema`, and
generated Rust for the proposal, view, and importing lib modules.
