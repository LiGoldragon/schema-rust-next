# Architecture

`schema-rust-next` consumes `schema-next::Asschema` and emits Rust source.

## Interfaces

- `RustEmitter` is the code-generation engine.
- `RustCode` is the generated source text.
- `GeneratedFile` names a generated path plus source text.
- `RustModulePath` maps single-colon schema identities to crate-local generated
  module paths. The crate namespace segment is dropped; `lib` becomes
  `src/schema/lib.rs`, and nested modules become files under `src/schema/`.

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
