# schema-rust-next

`schema-rust-next` emits Rust source code from `schema-next`'s assembled
schema.

This repository is deliberately not a Rust macro crate. The MVP path is:
assembled schema in, Rust source text out, compile and test the emitted source,
then layer macro ergonomics later.

The emitter consumes final assembled-schema data, not authored schema sugar.
That `Asschema` value is produced in memory by `schema-next` from real
`.schema` fixtures. Checked-in assembled-schema text fixtures are no longer
part of this repository's active test surface.

Generated paths mirror crate-local schema modules. An assembled schema identity
such as `spirit-next:lib` emits to `src/schema/lib.rs`; an identity such as
`spirit-next:signal:public` emits to `src/schema/signal/public.rs`. The first
namespace segment is the crate boundary and is not repeated inside the crate's
generated module tree.

The emitted source includes the data types, `nota-next` codec derives, small
inherent NOTA bridge methods, rkyv derives, short-header signal frames, Nexus traits, Nexus
mail lifecycle objects, mail-event hooks, and upgrade/accept traits that
runtime crates implement against.

Composite type references come from typed NOTA datatype objects in the
authored schema: `(Vec Topic)`, `(Map (Topic RecordIdentifier))`, and
`(Optional Topic)`. Authored datatype declarations use name-first `@` forms
such as `Entry@{ topic@Topic }` and `Kind@[Decision Correction]`. Square brackets
are still used by NOTA values; they are not schema datatype declarations and
they are not the schema surface for declaring `Vec`.

Tests keep meaningful schema and NOTA examples in fixture files under
`tests/fixtures/`. Rust tests load those fixtures through the support helpers
instead of hiding the language examples inside Rust string literals.

For development, `cargo run --example emit_schema -- <schema/lib.schema>
<crate:module> [version]` prints the generated Rust source so a consumer can
refresh its checked-in `src/schema/<module>.rs` from local schema-next and
schema-rust-next changes.
