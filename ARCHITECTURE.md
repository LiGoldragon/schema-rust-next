# Architecture

`schema-rust-next` emits Rust interface source from typed schema data. Its
source-facing input is `schema-next::SchemaSource`, the schema-in-Rust value
produced when authored `.schema` deserializes into Rust datatypes that fully
define the schema and serialize through rkyv. Its semantic emission input is
`schema-next::Schema`; there is no older assembled-schema artifact or path API
beside that typed source/schema pipeline.

## Interfaces

- `RustEmitter` is the code-generation engine.
- `RustSchemaLowering` is the entry trait implemented for `schema-next::Schema`.
  The deserialized semantic schema object owns the lowering call; the emitter
  supplies policy such as target, NOTA surface, and generator name.
- `RustSchemaSourceLowering` is the trait implemented for
  `schema-next::SchemaSource`. It lowers typed source through `SchemaEngine`
  and then through `RustSchemaLowering`.
- `LowerToRust<Target>` is the recursive projection trait implemented for the
  schema subobjects that own each piece of the Rust model: imports,
  declarations, type declarations, aliases, newtypes, structs, fields, enums,
  variants, and support metadata. The top-level lowering traits are facades
  over these per-noun projections, not a place where schema meaning is
  reassembled centrally.
- The Rust renderer uses `proc_macro2::TokenStream` and `quote` through
  context-carrying token wrappers over Rust-model nouns. Plain `ToTokens` has no
  context parameter, so generation-wide switches such as the NOTA feature gate
  and private-type visibility flow through `RustRenderContext` wrappers instead
  of being copied into every noun.
- The string-to-token migration is complete. Every emitted section — the
  declaration surface (aliases, newtypes, structs, fields, enums, variants),
  the NOTA bridges, the enum payload `From` impls and variant constructors, the
  signal-frame binary codec, the route enums/impls, the short-header module, the
  cross-plane `into_*` projections, the runtime role-trait impls, the upgrade
  trait surface, and the runtime/plane/runner support — renders itself through a
  `ToTokens` wrapper noun (e.g. `RustScalarAliasTokens`, `NewtypeInherentImplTokens`,
  `NotaInherentBridgeTokens`, `SignalFrameImplTokens`, `RouteEnumTokens`,
  `RouteImplTokens`, `ShortHeaderModuleTokens`, `EnumVariantConstructorsTokens`).
  These are routed through one `prettyplease` pass at `emit_item_tokens`. No
  emitter code builds Rust source with `format!`/`self.line`; the only direct
  text written is the leading `// @generated` header comment, which cannot pass
  through `prettyplease` (it drops non-doc comments).
- Cross-object emission logic is named rather than smeared across a god-struct.
  `ShortHeader` owns the per-root-per-variant route-triage constant (name +
  packed value), shared by the `short_header` module and the frame codec so the
  constant names always agree. `RouteName` owns the `<Name>Route` identifier.
  `ScreamingName` owns the SCREAMING_SNAKE constant projection. The split-plane
  Nexus projection builds its match arms through `split_signal_arrived_arms` and
  `split_output_arms`, which carry the exact-then-fallback variant-matching
  logic as named arm builders.
- Runtime plane naming has a three-tier boundary. `Plane` owns only
  plane-intrinsic names such as the module, wrapper, export aliases, engine
  trait, and trace enum names. `RuntimePlaneSet` / `RustEmissionTarget` own
  which planes a target emits. Schema-presence and construct-presence checks
  stay on the emitter nouns that inspect declarations and roots. Do not move
  target selection or schema logic into `Plane`.
- `RustModule` is the data model between semantic schema data and rendered
  Rust text. It carries scalar aliases, cross-crate imports, generated Rust
  declarations, root enums, and support metadata before anything is rendered.
  `RustModule::render` drives `RustModuleRenderer`, which owns the emission
  context and the schema-analysis predicates and accumulates the pretty-printed
  source as each section's `ToTokens` noun renders itself. The renderer builds
  no Rust as strings.
- `RustCode` is the generated source text.
- `GeneratedFile` names a generated path plus source text.
- `RustModulePath` maps single-colon schema identities to crate-local generated
  module paths. The crate namespace segment is dropped; `lib` becomes
  `src/schema/lib.rs`, and nested modules become files under `src/schema/`.
- `build::GenerationDriver` is the shared build-script orchestrator for
  source-visible generated schema modules. It owns the per-crate
  load/lower/emit/freshness sequence so component `build.rs` files do not
  hand-roll it.
- `build::GenerationPlan` names the crate package, target modules, and
  dependency schema directories. `build::ModuleEmission` selects the
  Rust-emission target for each schema module.
- `daemon_emit::DaemonModule` is the `triad_main!` emitter (designer report
  542): a per-component, source-visible `src/schema/daemon.rs`. It is OFF by
  default and turns ON only when a component declares a
  `daemon_emit::NexusDaemonShape` carrying the OS process name and working
  listener tier. The actor-native slice emits the uniform daemon skeleton:
  the `ComponentDaemon` hook trait (the 1488 escape hatches),
  `DaemonCommand` argv parsing, the `GeneratedDaemonRuntime`
  async decode->execute->encode spine, `ActorConnectionRuntime` over
  `triad_runtime::ActorSingleListenerDaemon`, `DaemonError`, and the
  `ExitReport`-based `DaemonEntry::run_to_exit_code`. The old synchronous
  `{Single,Multi}ListenerDaemon` selection and raw `UnixStream` meta hook are
  gone from daemon emission. If a shape asks for a meta listener tier or the
  schema declares streams, the emitter emits an explicit compile error until
  those concerns return as typed actor-native tiers. Each emitted section
  renders itself as a `ToTokens` noun through `RustModuleRenderer`, matching
  the rest of the crate — the daemon emitter builds no Rust as strings.

## Source Input And Semantic Schema

The public source-facing contract is typed schema source data decoded through
structural macro node codecs: authored `.schema` deserializes into
schema-defining Rust datatypes, those datatypes are rkyv-serializable, and this
emitter lowers that schema-in-Rust value into Rust interface code.
`RustEmitter::emit_file_from_schema_source` and
`emit_module_from_schema_source` are the build-driver path. They ask
`SchemaEngine` to lower source into `Schema`, then render `RustModule`.
Callers that already hold the semantic value use `Schema`'s
`RustSchemaLowering` trait methods (`lower_to_rust_file`,
`lower_to_rust_code`, `lower_to_rust_module`). Those methods build a
`RustLoweringContext` and recursively ask schema subobjects to lower
themselves into Rust-model nouns. No generated component path reads or writes a
separate assembled-schema text file.

The rendered source is `RustModule::render()`, so tests can inspect the module
data shape before comparing generated source text. Namespace entries arrive as
visibility-tagged declarations: `(Public Name Value)` or `(Private Name
Value)`. The emitter must project that boundary into Rust instead of
flattening every type into the same public surface.

The active fixtures use the current enum-body signature shape: square brackets
contain one vector element type, so unit variants are bare symbols and
data-carrying variants are parenthesized records such as `(Record Entry)`.
This emitter sees the typed source/schema data from `schema-next` and must not
grow a second parser for the authored form.

## Constraints

- No dependency on the old signal macro.
- No `macro_rules!` or proc-macro surface in `src/`.
- No authored-schema macro syntax is accepted as an emitter input. Tests lower
  real `.schema` fixtures into typed `Schema` values before comparing
  generated Rust; no assembled-schema text fixture is accepted.
- Public schema declarations emit public Rust types and fields. Private schema
  declarations emit `pub(crate)` types and fields, preserving inline
  PascalCase schema declarations as module-local implementation nouns.
- `TypeDeclaration::Alias` and `TypeDeclaration::Newtype` are distinct.
  Bare source bindings such as `Topic String`, `Topics (Vec Topic)`, or
  `Rejected SignalRejection` emit as Rust `type` aliases. A brace-body
  declaration with exactly one field emits as a tuple newtype.
  `TypeDeclaration::Struct` is the named-field map shape.
- Generated Rust is source-visible under `src/schema/`; consumers include or
  compile that source rather than hiding the interface in `OUT_DIR` or behind a
  compiler macro expansion. The emitter builds every section as `proc_macro2`
  tokens through `quote!` and pretty-prints them into checked source files. The
  string-based runtime emitter named as a migration target in Spirit record
  `0bw0` is gone: emission is token-based end to end (see the Interfaces section
  above).
- Emission is tested by source fixture comparison and by compiling the fixture
  as Rust code.
- `RustEmissionTarget::WireContract` emits the external signal or meta-signal
  wire surface: schema nouns, derives, NOTA/rkyv codecs, and short-header
  route constants. It does not emit runtime envelopes, engine traits, mail
  support, trace support, or signal-frame encode/decode helpers.
- `RustEmissionTarget::SignalRuntime` emits daemon-local Signal runtime
  support over signal roots: the Signal envelope, origin route, mail lifecycle
  nouns, Signal trace object names, and `SignalEngine`. The engine bridge uses
  associated Nexus input/output types so the Signal schema does not import
  daemon-internal Nexus vocabulary.
- `RustEmissionTarget::NexusRuntime` emits daemon-side Nexus support only:
  Nexus envelope, Nexus route/trace vocabulary, and `NexusEngine`.
- `RustEmissionTarget::SemaRuntime` emits daemon-side SEMA support only: SEMA
  envelope, SEMA route/trace vocabulary, and `SemaEngine`. SEMA write and read
  halves emit independently, so a read-only SEMA schema still gets
  `observe`. Split SEMA schemas may use plane-local operation root names
  (`WriteInput`, `WriteOutput`, `ReadInput`, `ReadOutput`) instead of the old
  all-in-one backing names (`SemaWriteInput`, `SemaWriteOutput`,
  `SemaReadInput`, `SemaReadOutput`); the generated public namespace remains
  `sema::WriteInput`, `sema::WriteOutput`, `sema::ReadInput`, and
  `sema::ReadOutput`.
- Runtime plane schemas live as schema files inside the daemon crate, such as
  `cloud/schema/nexus.schema` and `cloud/schema/sema.schema`, and may import
  contract roots when daemon logic needs the external wire vocabulary. Runtime
  code implements generated Nexus and SEMA traits on data-bearing engine
  objects.
- `RustEmissionTarget::ComponentRuntime` is the compatibility/bootstrap target
  for unsplit all-in-one schemas. It emits the old combined Signal/Nexus/SEMA
  runtime support, including the generic plane enum and cross-plane
  projections. New daemon schemas use the per-plane targets instead.
- Build scripts use the shared driver rather than local emit loops. A signal or
  meta-signal contract crate uses `GenerationPlan::wire_contract`, which emits
  `schema/lib.schema` through `RustEmissionTarget::WireContract`. A daemon
  crate uses `GenerationPlan::daemon_runtime`, which emits `schema/nexus.schema`
  through `RustEmissionTarget::NexusRuntime` and `schema/sema.schema` through
  `RustEmissionTarget::SemaRuntime`; daemon crates that carry a local Signal
  runtime module add `ModuleEmission::signal_runtime_module("signal")`
  explicitly. The shared runtime module builders use the same feature-gated
  `nota-text` surface as contracts: normal binary daemon builds keep
  `nota-next` absent, while all-feature trace/testing builds can round-trip
  generated runtime support nouns such as `NexusObjectName` and
  `SemaObjectName`. An unsplit bootstrap schema uses
  `GenerationPlan::component_runtime_compatibility`, keeping
  `RustEmissionTarget::ComponentRuntime` explicit until the schema is split.
- Cross-crate imports in daemon runtime schemas come from Cargo-exposed
  dependency schema directories. Dependency crates publish their `schema/`
  directory as build metadata, and consumers register those paths as
  `build::DependencySchema` entries before lowering runtime modules.
- `build::CargoSchemaMetadata` owns both sides of that Cargo seam. Contract
  crate build scripts call `emit_schema_directory` after a successful
  freshness check; daemon build scripts read the corresponding
  `DEP_<LINKS>_SCHEMA_DIR` value through `DependencySchema::from_cargo_metadata`.
- Driver freshness is source-visible and committed. Authored `.schema` input
  is parsed into `SchemaSourceArtifact` and round-tripped through generated
  schema text plus rkyv archive bytes as internal codec witnesses, but it is
  not treated as generated output. Generated Rust files are checked against the
  working tree; a component-specific update environment variable rewrites them
  when the schema intentionally changes. The shared component driver does not
  write or freshness-check any intermediate schema artifact.
- Signal, Nexus, and SEMA roots are emitted from the same schema shape:
  imports/exports, input, output, and namespace. Emission may attach different
  support traits per plane, but the generated Rust mirrors the same authored
  schema structure.
- Plane namespaces are emitted only for the selected runtime plane.
  `nexus::Work`, `nexus::Action`, `sema::WriteInput`, and `sema::ReadInput`
  are the public shape for plane-local payloads; the current flat backing
  names are a bootstrap detail until schema files split fully by plane.
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
- Stream-aware signal schemas additionally emit the direct `signal-frame`
  streaming surface when semantic `Schema::streams()` is non-empty and the
  stream event type is the payload of `Output.Event`. The emitted aliases mirror
  the wire kernel (`Frame = signal_frame::StreamingFrame<Input, Output, Event>`,
  `FrameBody`, `Request`, `ReplyEnvelope`, `RequestBuilder`) and the generated
  methods construct real `signal_frame::StreamingFrameBody::Request`,
  `Reply`, and `SubscriptionEvent` frames. This path reads schema-next stream
  metadata; it does not infer streaming from names alone and it does not route
  through the retired `signal_channel!` macro.
- Bootstrap all-in-one runtime emission emits mail-event nouns.
  `signal::Signal<Root>`, `nexus::Nexus<Root>`, and `sema::Sema<Root>` are the
  automatic envelopes for root objects in each plane; each has an
  `origin_route` field plus the root object.
  `schema::Plane::{Signal,Nexus,Sema}` is the data-carrying match surface for
  code that needs to branch across planes. Per-plane runtime targets emit only
  their own envelope and do not emit the generic three-plane enum.
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
- Generated construction is also an object method surface. Tuple newtypes emit
  `new`, `payload`, `into_payload`, and `From<Payload>`. Aliases emit no
  inherent impls because they have no distinct Rust type identity. Enums emit
  variant-named associated constructors (`Input::record(entry)`,
  `SemaWriteOutput::recorded(receipt)`, `Output::rejected(rejection)`) so
  component code does not hand-write nested wrapper constructors. When a
  variant stores a generated newtype wrapper, the constructor accepts the
  wrapper's inner payload and creates the wrapper internally; when a variant
  stores an alias, the constructor accepts the alias target directly.
- The next runner target is generated/programmatic component wiring. The
  emitter should grow a component-runner surface so a daemon binary can reduce
  to a tiny generated call while domain behavior still lives in non-default
  implementations of generated Signal, Nexus, and SEMA engine traits. The
  runner does not move algorithms into `main`; it gives the component a
  schema-defined place to instantiate Signal, Nexus, SEMA, transport, trace,
  and binary configuration surfaces.
- Nexus runner glue is generated when the Nexus action/work vocabulary has an
  exhaustive runner shape: `ReplyToSignal` plus any of `CommandSemaWrite`,
  `CommandSemaRead`, `CommandEffect`, and `Continue`, with the matching
  completion work variants present for storage and effects. The generated code
  emits a total `NexusAction` to `triad_runtime::NextStep` projection, a
  data-bearing `NexusRunnerAdapter`, typed hooks on `NexusEngine` for storage,
  effects, and budget exhaustion, and a runner-backed `execute` wrapper that
  keeps the trace hooks at one entered/decided pair per external request.
  Unknown action variants reject runner emission rather than falling through a
  wildcard. The shared loop itself stays in `triad-runtime`.
- Generated engine traits carry minimal lifecycle hooks. `NexusEngine` and
  `SemaEngine` each emit default no-op `on_start` and `on_stop` methods
  returning typed `ActorStartFailure` and `ActorStopFailure` results. The
  bootstrap all-in-one `ComponentRuntime` target still emits the historical
  `SignalEngine` trait while unsplit schemas exist. These hooks give the
  generated runner and persona supervision a graceful-start/stop address
  without introducing full actor mailbox or runtime-control traits before those
  behaviors are needed.
- The `NexusRuntime` target emits a mutable `NexusEngine` trait for the heavier
  execution and decision plane. The `SemaRuntime` target emits
  `SemaEngine`. Schemas that declare a `SemaWriteInput` / `SemaWriteOutput`
  pair emit the mutable `SemaEngine::apply` path; schemas that declare a
  `SemaReadInput` / `SemaReadOutput` pair emit the shared-reference
  `SemaEngine::observe` path. Tests and runtime code use those generated plane
  traits so Nexus and SEMA take and return routed root messages for their own
  planes.
- Cross-plane projections prefer exact operation names before falling back to a
  unique payload type. That lets a realistic interface carry both
  `Lookup(RecordIdentifier)` and `Remove(RecordIdentifier)` without routing the
  read operation to the write plane, while still allowing semantic output
  bridges such as `Recorded(SemaReceipt)` to become
  `RecordAccepted(SemaReceipt)` when the payload type is unique.
- The engine traits also own testing trace hooks. Per-plane implementors
  provide `decide`, `apply_inner`, and `observe_inner`; the bootstrap
  all-in-one target also provides `triage_inner` and `reply_inner`. Generated
  default wrappers keep the public method names `execute`/`apply`/`observe`
  (plus `triage`/`reply` in the bootstrap target) and call default no-op trace
  hooks around the inner behavior. Those hooks activate typed generated object
  names, not strings: Nexus receives `NexusObjectName`, and SEMA receives
  `SemaObjectName`; the bootstrap target also emits `SignalObjectName`.
  Interface/header names use route enums such as `NexusInputRoute` and
  `SemaReadInputRoute`; actor-boundary names live beside the plane that owns
  them (`NexusObjectName::Entered`, `SemaObjectName::WriteApplied`,
  `SemaObjectName::Stopped`). The generated `ObjectName` enum wraps the emitted
  per-plane names for `TraceEvent` transport. A non-trace consumer gets the
  no-op defaults without linking a parallel instrumentation API.
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
- Scalar references are explicit schema data. `TypeReference::String`,
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
- `RustEmissionOptions` carries `nota_surface` and `target`. The named
  constructors (`::binary_only`, `::feature_gated_nota("...")`,
  `::always_enabled_nota`) set the compatibility target
  `RustEmissionTarget::ComponentRuntime`; callers use `with_target` to select
  `RustEmissionTarget::WireContract` for external signal and meta-signal
  contract generation, `RustEmissionTarget::NexusRuntime` for daemon Nexus
  schemas, and `RustEmissionTarget::SemaRuntime` for daemon SEMA schemas.
  `RustEmissionOptions::default()` and `RustEmitter::default()` both pick
  `NotaSurface::FeatureGated { feature: "nota-text" }` plus
  `ComponentRuntime`. `NotaSurface::Disabled` removes the
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
