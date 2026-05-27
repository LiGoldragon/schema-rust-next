# Architecture

`schema-rust-next` consumes `schema-next::Asschema` and emits Rust source.

## Interfaces

- `RustEmitter` is the code-generation engine.
- `RustCode` is the generated source text.
- `GeneratedFile` names a generated path plus source text.
- `RustModulePath` maps single-colon schema identities to crate-local generated
  module paths. The crate namespace segment is dropped; `lib` becomes
  `src/schema/lib.rs`, and nested modules become files under `src/schema/`.

## Emits for three schema types — Signal / Nexus / Sema

Per spirit record 964 (Maximum, 2026-05-27): the schema layer has
three schema types corresponding to three runtime planes. This crate
emits Rust for all three:

| Schema type | Runtime plane | Emitted Rust shape |
|---|---|---|
| `Signal` | Wire and communication layer | Input/Output enums + encode/decode + Communicate trait methods on root |
| `Nexus` | Execution layer — IO, external calls, all UI | Input/Output enums + handler trait + return-type encoding |
| `Sema` | Durable state layer (the database) | Record types + storage/migration trait surface |

Each plane gets its own engine + traits in the consuming runtime
crate; this emitter produces the type vocabulary common to all
three patterns (input-message / run-code / output-message) and the
plane-specific trait surface.

The **root type** of each schema is the message type; the emitter
attaches the plane-appropriate methods to that root.

File extensions are open per record 964: `.signal.schema` /
`.nexus.schema` / `.sema.schema`, OR the variant as the first record
of the schema content. The emitter routes by whichever the schema
declares.

Per record 965 (Maximum, 2026-05-27): Nexus schemas cover internal
IO, external CLI calls (e.g. cloud-to-Cloudflare), and ALL user
interfaces (Mencie panels each have their own nexus schema).

## Constraints

- No dependency on the old signal macro.
- No `macro_rules!` or proc-macro surface in `src/`.
- Generated Rust is source-visible under `src/schema/`; consumers include
  or compile that source rather than hiding the interface in `OUT_DIR`.
  This is the load-bearing emission target locked by spirit records 909
  and 910 (Maximum, 2026-05-27) per the literal wording of record 902.
- Emission is tested by source fixture comparison and by compiling the fixture
  as Rust code.
- Emitted functions live in `impl` blocks of the emitted struct/enum
  types they belong to. The emitter does not produce free helpers
  (spirit records 712, 882). Trait-impl projections (`impl From<X>`)
  are preferred for conversion code over named functions.
- The crate name segment of a colon-qualified schema identity drops out
  of the emitted Rust module path; emitted modules live under the
  consumer crate's `src/schema/` and are addressed by the local part
  of the qualified name.
