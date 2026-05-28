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
- Generated Rust is source-visible under `src/schema/`; consumers include or
  compile that source rather than hiding the interface in `OUT_DIR`.
- Emission is tested by source fixture comparison and by compiling the fixture
  as Rust code.
- Root input/output enums emit Nexus traits. Runtime code implements those
  traits on data-bearing engine objects, and the generated enum dispatches
  in-flight `NexusMail<Payload>` to one method per variant. Nexus names the
  execution-IO plane and mail keeper; it replaces the older executor wording.
- Signal, Nexus, and SEMA roots are emitted from the same schema shape:
  imports/exports, input, output, and namespace. Emission may attach different
  support traits per plane, but the generated Rust mirrors the same authored
  schema structure.
- Single-colon schema namespaces map to generated Rust module paths. The
  schema path `spirit-next:nexus:Mail` becomes a module/type path under
  `src/schema/` without inventing a second naming system.
- Generated schema objects emit `UpgradeFrom<Previous>` and
  `AcceptPrevious<Previous>` trait surfaces. A changed type gets hand-written
  upgrade behavior on the generated noun; unchanged types do not need upgrade
  logic.
- Generated signal roots emit rkyv-derived data types, NOTA text conversion,
  short-header route triage, and binary signal-frame encode/decode methods.
- Generated signal roots emit mail-event nouns. `MessageSent` records the
  message identifier, root schema type, and short header, and pushes through
  `MessageSentHook` so routers, UI layers, or introspection subscribers can
  react without polling. `NexusMail<Payload>` represents mail being processed
  by Nexus, and `MessageProcessed<Reply>` carries the processed reply after
  Nexus receives the SEMA or execution outcome.
- Generated objects are the hand-written behavior surfaces. The emitter must
  not compensate for missing runtime nouns by producing free helper functions.
  If dispatch, upgrade, mail acceptance, or SEMA application needs behavior,
  the generated type exposes a trait or method target and the consumer
  implements it on a data-bearing actor or store object.
- Schemas that declare `NexusInput`/`NexusOutput` emit a `NexusEngine` trait,
  and schemas that declare `SemaInput`/`SemaOutput` emit a `SemaEngine` trait.
  Tests and runtime code use those generated plane traits so Nexus takes and
  returns Nexus schema objects, and SEMA takes and returns SEMA schema objects.
- Mail identifiers and short headers use the generated scalar floor
  (`Integer`) rather than bespoke primitive widths. This keeps the runtime mail
  support closer to schema-authored nouns while the core mail schema is still
  emitted by the support surface.

## Cross-crate import emission

`RustEmitter` gained two methods, both early-returning on empty imports so
existing emission stays byte-identical:

- `emit_imports` writes one `pub use <dep-path> as <Local>;` per resolved
  import, placed before the types that reference them so the alias resolves.
  No `pub struct`/`pub enum` is emitted for an imported type — the dependency
  crate owns the definition.
- `emit_imported_error_bridges` writes one
  `impl From<<dep-module>::NotaDecodeError> for NotaDecodeError` per distinct
  imported module, collapsing the dependency's parse failures into the local
  `Parse` variant so cross-crate NOTA codecs compose.

schema-next resolves the imports (`Asschema::resolved_imports`); this repo
turns each `ResolvedImport` into the `use` alias and the error bridge. Proven
end-to-end in Nix by the `schema-core` repo's `nix flake check`.

## Collection emission (record 1034)

The emitter turns the `TypeReference` collection variants into real Rust:

- `rust_type` recurses: `Vector` → `Vec<inner>`, `Map` → fully-qualified
  `std::collections::BTreeMap<key, value>` (no `use` emitted, deterministic
  ordering), `Optional` → `Option<inner>`. `Plain` keeps the leaf name (after a
  cross-crate import alias, the imported type's local name).
- `parse_expression` / `format_expression` recurse over the same variants and
  emit a NOTA codec. NOTA shapes: a `Vec` is `[e1 e2 ...]`, a `BTreeMap` is a
  brace of `key value` pairs `{k1 v1 ...}`, an `Option` is the atom `None` or
  `(Some inner)`.
- `emit_collection_support` emits a `NotaCollection` runtime type carrying
  `parse_vector` / `parse_map` / `parse_option` (closure-per-element so nested
  collections compose) and the matching `format_*` associated functions. It is
  emitted ONLY when `CollectionScan` finds a collection in the schema, so
  collection-free schemas keep byte-identical emission.
- Map keys need ordering. `CollectionScan::map_key_type_names` finds every type
  used as a `BTreeMap` key (recursively, through nested collections), and
  `data_type_derive` adds `PartialOrd, Ord` plus `#[rkyv(derive(..., Ord))]` to
  exactly those types' derives. Non-key types keep the original derive set.

This unblocks the aggregate roots in horizon and lojix, which are all
collection-bearing. Demonstrated end-to-end on the real Horizon `ClusterProposal`
shape in the `horizon-next` concept repo. `examples/emit_collections_probe.rs`
prints the emission for a small collection-bearing schema; the
`collections_emission` test compiles the captured output and round-trips a
collection value through NOTA and rkyv.

## The Plane surface and the runtime floor

`RustEmitter::emit` emits the runtime floor only for a component module
(`Asschema::signal_plane().is_some()`). The floor is, in order:

- `emit_mail_event_support` — `MessageIdentifier`, `OriginRoute` (the
  auto-created route, records 1038/1039), `MessageRoot`, `MessageSent`,
  `NexusMail<Payload>`, `MessageProcessed<Reply>`, the push hooks, all
  threading `origin_route`.
- `emit_nexus_support` — per-root `XNexus` dispatch traits + the
  `dispatch_mail_with_nexus` method threading the origin route.
- `emit_plane_surface` — the record-1054 `Plane` enum (`Signal(OriginRoute,
  Input)` / `Nexus(OriginRoute, Input)` / `Sema(OriginRoute, Output)`), the
  three trait-ordered engines `SignalEngine` / `NexusEngine` / `SemaEngine`
  (each `Plane -> Plane`), the `PlaneChainError` enum, and `Plane::drive` —
  the running chain (record 1030) that threads Signal → Nexus → Sema and
  echoes the origin route.

A types-only module (`signal_plane() == None`) emits none of this — its types
are imported across the crate boundary by component modules.
