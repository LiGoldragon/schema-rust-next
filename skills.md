# Skills — schema-rust-next

Read the workspace Rust and schema skills before editing this repo:

- `skills/rust-discipline.md`
- `skills/rust/methods.md`
- `skills/rust/errors.md`
- `skills/rust/storage-and-wire.md`
- `skills/rust/crate-layout.md`
- `skills/abstractions.md`
- `skills/actor-systems.md`

This repo emits Rust nouns from assembled schema data. Generated Signal,
Nexus, and SEMA traits are the runtime method surface; component crates supply
non-default algorithms by implementing those traits on data-bearing runtime
objects. Do not add parser-side shortcuts or hand-written helper APIs beside
the generated trait path.

Use `RustEmissionTarget::WireContract` for signal and meta-signal contract
repos: that target emits wire nouns and codecs only. Use
`RustEmissionTarget::ComponentRuntime` for daemon-crate schema files, including
per-plane runtime schemas such as `schema/nexus.schema` and `schema/sema.schema`
that import contract roots where needed.
