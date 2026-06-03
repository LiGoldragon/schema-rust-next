# Architecture

`schema-rust-next` consumes `schema-next::Asschema` and emits Rust source.

## Interfaces

- `RustEmitter` is the code-generation engine.
- `RustModule` is the data model between assembled schema and rendered Rust
  text. It carries scalar aliases, cross-crate imports, generated Rust
  declarations, root enums, and support metadata before anything is rendered.
- `RustCode` is the generated source text.
- `GeneratedFile` names a generated path plus source text.
- `RustModulePath` maps single-colon schema identities to crate-local generated
  module paths. The crate namespace segment is dropped; `lib` becomes
  `src/schema/lib.rs`, and nested modules become files under `src/schema/`.

## Input Contract

The input contract is assembled schema, not authored schema. `Asschema` has
already resolved all macros and sugar; the emitter does not read authored macro
calls, sigils, or structural macro captures. The active test path gets that
`Asschema` as typed data from `schema-next` lowering real `.schema` fixtures,
then proves the emitter can consume the same value after an asschema NOTA
artifact file read and an asschema rkyv artifact file read. That keeps Rust
emission attached to the live assembled data object rather than to hidden
parser state.
`RustEmitter::emit_file_from_artifact`, `emit_file_from_nota_path`, and
`emit_file_from_binary_path` are the explicit artifact handoff methods; the
plain `emit_file(&Asschema)` path remains for callers that already hold the
typed value in-process.
All of those paths now converge through `RustEmitter::emit_module(&Asschema)`.
The rendered source is `RustModule::render()`, so tests can inspect the module
data shape before comparing strings.
Namespace entries arrive as visibility-tagged declarations: `(Public Name
Value)` or `(Private Name Value)`. The emitter must project that boundary into
Rust instead of flattening every type into the same public surface.

The active fixtures use the current enum-body signature shape: square
brackets contain one vector element type, so unit variants are bare symbols
and data-carrying variants are parenthesized records such as
`(Record Entry)`. This emitter only sees the resulting `Asschema` data and
must not grow a second parser for the authored form.

## Constraints

- No dependency on the old signal macro.
- No `macro_rules!` or proc-macro surface in `src/`.
- No authored-schema macro syntax is accepted as an emitter input. Tests lower
  real `.schema` fixtures into typed `Asschema` values before comparing
  generated Rust; no assembled-schema text fixture is accepted.
- Public asschema declarations emit public Rust types and fields. Private
  asschema declarations emit `pub(crate)` types and fields, preserving inline
  PascalCase schema declarations as module-local implementation nouns.
- `TypeDeclaration::Newtype` carries a single contained `TypeReference`, not a
  field map. It emits as a tuple newtype. `TypeDeclaration::Struct` is the
  named-field map shape.
- Generated Rust is source-visible under `src/schema/`; consumers include or
  compile that source rather than hiding the interface in `OUT_DIR`.
- Emission is tested by source fixture comparison and by compiling the fixture
  as Rust code.
- Root declarations emit Signal, Nexus, and SEMA traits. Runtime code
  implements those traits on data-bearing engine objects. Signal triage
  creates generated Nexus envelopes directly, Nexus executes through the
  generated mutable trait, and SEMA splits writes from reads. No generated
  convenience mail wrapper or parallel dispatch trait remains beside the
  working plane traits.
- Signal, Nexus, and SEMA roots are emitted from the same schema shape:
  imports/exports, input, output, and namespace. Emission may attach different
  support traits per plane, but the generated Rust mirrors the same authored
  schema structure.
- Plane namespaces are emitted for the three runtime planes. `signal::Input`,
  `nexus::Input`, `sema::WriteInput`, and `sema::ReadInput` are the public
  shape for plane-local payloads; the current flat backing names are a
  bootstrap detail until schema files split fully by plane.
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
  or introspection subscribers can react without polling. `MessageProcessed`
  carries the same origin route with the processed reply after Nexus receives
  the SEMA or execution outcome.
- Generated objects are the hand-written behavior surfaces. The emitter must
  not compensate for missing runtime nouns by producing free helper functions.
  If dispatch, upgrade, mail acceptance, or SEMA application needs behavior,
  the generated type exposes a trait or method target and the consumer
  implements it on a data-bearing actor or store object.
- The next runner target is generated/programmatic component wiring. The
  emitter should grow a component-runner surface so a daemon binary can reduce
  to a tiny generated call while domain behavior still lives in non-default
  implementations of generated Signal, Nexus, and SEMA engine traits. The
  runner does not move algorithms into `main`; it gives the component a
  schema-defined place to instantiate Signal, Nexus, SEMA, transport, trace,
  and binary configuration surfaces.
- Generated engine traits carry minimal lifecycle hooks. `SignalEngine`,
  `NexusEngine`, and `SemaEngine` each emit default no-op `on_start` and
  `on_stop` methods returning typed `ActorStartFailure` and
  `ActorStopFailure` results. These hooks give the generated runner and
  persona supervision a graceful-start/stop address without introducing full
  actor mailbox or runtime-control traits before those behaviors are needed.
- Schemas that declare `Input`/`Output` roots emit a `SignalEngine` trait.
  The Signal trait only owns boundary triage: Signal input becomes Nexus input,
  and Nexus replies become Signal output. Schemas that declare
  `NexusInput`/`NexusOutput` emit a mutable `NexusEngine` trait for the heavier
  execution and decision plane. Schemas that declare `SemaWriteInput` /
  `SemaWriteOutput` plus `SemaReadInput` / `SemaReadOutput` emit a
  `SemaEngine` trait with `apply(&mut self, ...)` for mutations and
  `observe(&self, ...)` for reads. Tests and runtime code use those generated
  plane traits so Signal, Nexus, and SEMA take and return routed root messages
  for their own planes.
- Cross-plane projections prefer exact operation names before falling back to a
  unique payload type. That lets a realistic interface carry both
  `Lookup(RecordIdentifier)` and `Remove(RecordIdentifier)` without routing the
  read operation to the write plane, while still allowing semantic output
  bridges such as `Recorded(SemaReceipt)` to become
  `RecordAccepted(SemaReceipt)` when the payload type is unique.
- The engine traits also own testing trace hooks. Implementors provide
  `triage_inner`, `reply_inner`, `decide`, `apply_inner`, and `observe_inner`;
  generated default wrappers keep the public method names
  `triage`/`reply`/`execute`/`apply`/`observe` and call default no-op trace
  hooks around the inner behavior. Those hooks activate typed generated
  object names, not strings: Signal receives `SignalObjectName`, Nexus receives
  `NexusObjectName`, and SEMA receives `SemaObjectName`. Interface/header names
  use route enums such as `InputRoute`, `NexusInputRoute`, and
  `SemaReadInputRoute`; actor-boundary names live beside the plane that owns
  them (`SignalObjectName::Started`, `NexusObjectName::Entered`,
  `SemaObjectName::WriteApplied`, `SemaObjectName::Stopped`). The
  generated `ObjectName` enum wraps the per-plane names for `TraceEvent`
  transport. A non-trace consumer gets the no-op defaults without linking a
  parallel instrumentation API.
- Trace remains typed data until the client display boundary. The generated
  `TraceEvent` is the component-specific event noun. Its current emitted shape
  is a transparent tuple newtype over `ObjectName`, so `TraceEvent` serializes
  to the generated object-name NOTA shape instead of a double-wrapped
  one-field struct. The shared
  `triad-runtime` trace client/log/socket objects are generic over that noun.
  The next emitter target is generating the small component adapters that are
  still mechanical today: `TraceEventFrame` for rkyv trace archives,
  `Display for TraceEvent` that renders the generated NOTA value at the
  client edge, and aliases for `TraceLog<TraceEvent>` /
  `TraceClient<TraceEvent>` when a trace surface is emitted. The emitter must
  not generate string-log substrates or a
  trace-on-trace path by default.
- Help/documentation emission comes from typed schema description data. The
  target is a mirror description namespace keyed by fully qualified schema
  symbols, with generated defaults for symbols that have no explicit
  description. Generated help actions or client help output render that typed
  description data at the client edge; they are not hand-written CLI string
  tables.
- Mail identifiers, origin routes, and short headers use the generated scalar
  floor (`Integer`) rather than bespoke primitive widths. This keeps the runtime
  mail support closer to schema-authored nouns while the core mail schema is
  still emitted by the support surface.
- Scalar references are explicit asschema data. `TypeReference::String`,
  `TypeReference::Integer`, `TypeReference::Boolean`, and `TypeReference::Path`
  emit the scalar aliases (`String = std::string::String`, `Integer = u64`,
  `Boolean = bool`). Binary `rkyv` support is emitted for every consumer; NOTA
  codec derives are an optional text-client surface.
  `Plain(Name)` no longer carries scalar special cases; it names an emitted or
  imported schema type.
- Collection references emit standard Rust collections. Authored schemas use
  Schema type-reference vocabulary such as `(Vec Topic)`, `(Map (Topic
  RecordIdentifier))`, and `(Optional Topic)`. Authored datatype declarations
  are strict namespace key/value entries: `Topic String`,
  `Entry { topic Topic }`, and `Kind [Decision Correction]`. Square brackets
  declare enum bodies at enum positions; they are not the `Vec` reference
  syntax. The emitter's
  Rust type projection recurses a `TypeReference`: `Vector` → `Vec<inner>`,
  `Map` → `std::collections::BTreeMap<key, value>` (fully qualified, so no
  `use` and a deterministic key order for rkyv + NOTA), `Optional` →
  `Option<inner>`.
- Generated code can import `nota-next`'s shared codec surface and derive
  `nota_next::NotaDecode` / `nota_next::NotaEncode` for generated nouns, but
  that surface is selected by `RustEmissionOptions`: always enabled,
  feature-gated for text clients, or disabled for binary-only consumers. When
  NOTA is selected, small inherent bridge methods (`from_nota_block`,
  `to_nota`) stay on the owning noun, but the emitter does not hand-write
  per-type codec trait implementations. It does not emit private `NotaSource`,
  `NotaBlock`, or `NotaCollection` helper types. Its NOTA value shapes stay the
  shared codec shapes: a `Vec` is a square-bracket block `[e1 e2 ...]`, a
  `BTreeMap` is a brace block of `key value` pairs `{k1 v1 ...}`, and an
  `Option` is the atom `None` or the paren `(Some inner)`.
- `RustEmissionOptions` carries one field — `pub nota_surface: NotaSurface` —
  which is the only knob today. Callers either construct positionally
  (`RustEmissionOptions { nota_surface: NotaSurface::Disabled }`) or through
  the named constructors (`::binary_only`, `::feature_gated_nota("…")`,
  `::always_enabled_nota`). `RustEmissionOptions::default()` and
  `RustEmitter::default()` both pick `NotaSurface::FeatureGated { feature:
  "nota-text" }` — the recommended shape per the codec opt-in design (rkyv is
  universal, NOTA derives gate behind a cargo feature so binary-only
  consumers don't compile `nota-next`). `NotaSurface::Disabled` removes the
  NOTA surface entirely: no derives, no `use nota_next::*` items, no
  `from_nota_block` / `to_nota` bridges, no root `FromStr` / `Display`
  impls. `NotaSurface::AlwaysEnabled` keeps the older unconditional emission
  for callers (mostly tests) that always want NOTA on.
- NOTA owns those value shapes. Schema owns the type-name keywords that select
  scalar and composite type references in `.schema` files.
- A type used anywhere as a `BTreeMap` key earns the ordering derives
  (`PartialOrd, Ord` plus the archived `#[rkyv(derive(...))]`); value-only and
  non-collection types keep the original derive set. `CollectionScan` decides
  both the collection-codec emission and the map-key derive set, so a
  collection-free schema stays byte-identical to the current scalar-floor
  fixture when collection support changes.
- Integration tests load substantive `.schema` and `.nota` language examples
  from `tests/fixtures/` through `tests/support::FixtureSchema` and
  `FixtureNota`. Inline Rust strings remain for short expected generated-code
  fragments; the actual schema/NOTA input surfaces stay visible as files.
