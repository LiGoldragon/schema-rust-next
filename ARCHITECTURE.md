# Architecture

`schema-rust-next` consumes `schema-next::Asschema` and emits Rust source.

## Interfaces

- `RustEmitter` is the code-generation engine.
- `RustCode` is the generated source text.
- `GeneratedFile` names a generated path plus source text.
- `RustModulePath` maps single-colon schema identities to crate-local generated
  module paths. The crate namespace segment is dropped; `lib` becomes
  `src/schema/lib.rs`, and nested modules become files under `src/schema/`.

## Input Contract

The input contract is assembled schema, not authored schema. `Asschema` has
already resolved all macros and sugar; the emitter does not read authored
`Vec`/`Option`/`KeyValue` macro calls, `$Name`, or structural macro captures. The final
collection/reference variants are `Plain`, `Vector`, `Optional`, and `Map`;
`Map` carries one vector payload, for example
`(Map [(Plain Topic) (Plain RecordIdentifier)])`.

## Constraints

- No dependency on the old signal macro.
- No `macro_rules!` or proc-macro surface in `src/`.
- No authored-schema macro syntax is accepted as an emitter input. Tests parse
  checked `.asschema` fixtures through `Asschema::from_nota` and reject macro
  markers or loose old `Map` payloads before comparing generated Rust.
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
- Plane namespaces are emitted for the three runtime planes. `signal::Input`,
  `nexus::Input`, and `sema::Input` are the public shape for plane-local
  payloads; the current flat backing names (`Input`, `NexusInput`,
  `SemaInput`) are a bootstrap detail until the schema files split fully by
  plane.
- Single-colon schema namespaces map to generated Rust module paths. The
  schema path `spirit-next:nexus:Mail` becomes a module/type path under
  `src/schema/` without inventing a second naming system.
- Cross-crate schema imports are emitted as Rust aliases, not local
  re-declarations. If `schema-next` resolves `DatabaseMarker` from
  `marker-core:mail:DatabaseMarker`, this emitter writes a `pub use
  marker_core::schema::mail::DatabaseMarker as DatabaseMarker;` line and local
  fields or variants reference that alias. The dependency crate owns the
  imported type's rkyv/NOTA implementations; the consumer only bridges imported
  decode errors into its own generated error type.
- Generated schema objects emit `UpgradeFrom<Previous>` and
  `AcceptPrevious<Previous>` trait surfaces. A changed type gets hand-written
  upgrade behavior on the generated noun; unchanged types do not need upgrade
  logic.
- Generated signal roots emit rkyv-derived data types, NOTA text conversion,
  short-header route triage, and binary signal-frame encode/decode methods.
- Generated signal roots emit mail-event nouns. `signal::Signal<Root>`,
  `nexus::Nexus<Root>`, and `sema::Sema<Root>` are the automatic envelopes for
  root objects in each plane; each has an `origin_route` field plus the root
  object. `schema::Plane::{Signal,Nexus,Sema}` is the data-carrying match
  surface for code that needs to branch across planes.
  `MessageSent` records the message identifier, origin route, root schema type,
  and short header, and pushes through `MessageSentHook` so routers, UI layers,
  or introspection subscribers can react without polling. `NexusMail<Payload>`
  represents mail being processed by Nexus and carries the same origin route;
  `MessageProcessed` carries it again with the processed reply after Nexus
  receives the SEMA or execution outcome.
- Generated objects are the hand-written behavior surfaces. The emitter must
  not compensate for missing runtime nouns by producing free helper functions.
  If dispatch, upgrade, mail acceptance, or SEMA application needs behavior,
  the generated type exposes a trait or method target and the consumer
  implements it on a data-bearing actor or store object.
- Schemas that declare `NexusInput`/`NexusOutput` emit a `NexusEngine` trait,
  and schemas that declare `SemaInput`/`SemaOutput` emit a `SemaEngine` trait.
  Tests and runtime code use those generated plane traits so Nexus takes and
  returns routed Nexus root messages, and SEMA takes and returns routed SEMA
  root messages.
- Mail identifiers, origin routes, and short headers use the generated scalar
  floor (`Integer`) rather than bespoke primitive widths. This keeps the runtime
  mail support closer to schema-authored nouns while the core mail schema is
  still emitted by the support surface.
- Collection references emit standard Rust collections. `rust_type` recurses a
  `TypeReference`: `Vector` → `Vec<inner>`, `Map` (the `KeyValue` keyword) →
  `std::collections::BTreeMap<key, value>` (fully qualified, so no `use` and a
  deterministic key order for rkyv + NOTA), `Optional` → `Option<inner>`. The
  `parse_expression` / `format_expression` recursions mirror the type:
  `parse_vector` / `parse_map` / `parse_option` decode and `format_vector` /
  `format_map` / `format_option` encode, each taking a per-element closure so
  nesting composes. A plain non-scalar element passes its `from_nota_block`
  associated function directly (no redundant closure); a map / vector / option
  value already held by reference is passed without a needless borrow.
- A `NotaCollection` runtime codec block is emitted only when the schema uses a
  collection. Its NOTA shapes: a `Vec` is a square-bracket block `[e1 e2 ...]`,
  a `BTreeMap` is a brace block of `key value` pairs `{k1 v1 ...}`, an `Option`
  is the atom `None` or the paren `(Some inner)`.
- A type used anywhere as a `BTreeMap` key earns the ordering derives
  (`PartialOrd, Ord` plus the archived `#[rkyv(derive(...))]`); value-only and
  non-collection types keep the original derive set. `CollectionScan` decides
  both the collection-codec emission and the map-key derive set, so a
  collection-free schema emits byte-identical Rust to the pre-collection
  emitter — the regression safety net the legacy fixture comparison enforces.
