# INTENT — schema-rust-next

`schema-rust-next` emits Rust interface source from typed schema data and
powers the shared build-driver orchestrator for generated schema modules.

Load-bearing constraints:

*Rust emission is a separate step from Rust macros.* Schema generates Rust
code first; macros are a later or separate consumption surface. Generated Rust
code is emitted into the consumer crate source tree under `src/schema/`, not
hidden in `OUT_DIR`. Source-visible generated interfaces are reviewable and
can become committed or freshness-checked build artifacts.

*Schema-generated objects are the Rust nouns that carry behavior.* Actor input
and output roots become enums; runtime engines implement generated Nexus
traits with one method per reaction variant on data-bearing objects, not free
helper functions.

*Streaming wire support is emitted from semantic stream metadata.* A schema
that declares `Schema::streams()` and whose stream event type matches
`Output.Event` emits direct `signal-frame` streaming aliases and frame builders:
`Frame`, `FrameBody`, `Request`, `ReplyEnvelope`, `RequestBuilder`,
`Input::into_frame`, `Output::into_reply_frame`, and
`EventPayload::into_subscription_frame`. A bare `Output.Event` name without a
stream declaration is not enough.

*Daemon emission is async task-backed at the listener boundary.* The generated
daemon module emits async listeners over
`triad_runtime::AsyncSingleListenerDaemon` for working-only daemons and
`triad_runtime::AsyncMultiListenerDaemon` for working + meta daemons. Working
traffic uses `AsyncConnectionRuntime` and the async length-prefixed frame IO
spine; working + meta traffic uses `AsyncMultiConnectionRuntime` with a
generated listener-identity enum. The retired synchronous
`{Single,Multi}ListenerDaemon` and raw `UnixStream` daemon surfaces are not
compatibility paths. Declared-stream daemon emission stays async task-backed: the
runtime consumes `AcceptedConnection`, preserves the kernel-vouched
`ConnectionContext`, splits the Tokio stream, retains an owned writer half for
subscription pushes, and uses `triad_runtime`'s typed subscription registry and
publisher for event delivery.
Daemon emission applies both the ordinary working socket mode
(`DaemonConfiguration::socket_mode`) and the meta socket mode through
runtime-owned listener sockets. Components that need a temporary
relation-adapter can opt into `WorkingListenerTier::component_decoded()`: the
generated daemon still owns argv, async task-backed binding, request admission,
peer credentials, lifecycle, and exit handling, while the component owns only
the accepted working connection's relation-specific frame decode/encode.

*Nexus runtime emission is async task-backed at the engine boundary.* Generated
`NexusEngine::execute` returns an awaitable future, generated SEMA/effect
runner hooks are awaitable, and the generated adapter awaits
`triad_runtime::Runner::drive`. Component crates do not isolate generated
Nexus execution with blocking pools; slow storage and external effects belong
behind async generated hooks.

*Cross-crate schema imports preserve type ownership.* A consumer schema that
imports `crate:module:Type` emits a local Rust alias to the dependency crate's
generated type instead of re-declaring. The imported crate owns the type
definition; the consumer only uses the alias.

*Plane payload names are scoped by emitted namespaces.* Generated public
surface reads `signal::Input`, `nexus::Input`, `sema::WriteInput`, and
`sema::ReadInput` inside their respective planes, not redundant plane ancestry
at every use site.

*Collection references emit standard Rust collections with deterministic
rkyv/NOTA round-trips.* `(Vec T)` emits `Vec<T>`, `(Map (K V))` emits
`std::collections::BTreeMap<K, V>` (ordered), and `(Optional T)` emits
`Option<T>`.

*The shared generation driver consumes `SchemaSource` at the component build
boundary.* Per-crate `GenerationDriver` owns the load/lower/emit/freshness
sequence so component `build.rs` files do not hand-roll it. The driver
validates the canonical `.schema` text projection and rkyv source archive,
then emits Rust through the semantic `Schema` value. It does not materialize
or freshness-check an intermediate schema artifact, and it does not preserve
older assembled-schema artifact or path APIs beside the source/schema
pipeline.

*Rust lowering is a trait surface on the typed schema objects and their
subobjects.* `schema-next` owns schema semantics and must not depend on Rust
emission, so the trait lives here and is implemented for `schema_next::Schema`,
`schema_next::SchemaSource`, and the schema nouns that project into Rust-model
nouns: declarations, imports, type declarations, aliases, newtypes, structs,
fields, enums, variants, and support metadata. `RustEmitter` supplies emission
policy; the deserialized schema structure owns the lowering calls recursively
instead of handing the whole tree to a centralized adapter.

*Generated Rust syntax is built as Rust tokens, then written as visible source.*
The emitter uses Rust's macro/codegen substrate (`proc_macro2` and `quote`) for
syntax construction rather than treating Rust as ad hoc formatted strings. The
source-visible boundary still stands: generated modules are pretty-printed into
`src/schema/*.rs` and freshness-checked by the build driver rather than hidden
behind compiler macro expansion.
Each top-level generated item carries `#[rustfmt::skip]`: the generator's
`prettyplease` projection is the stable artifact boundary, and
`cargo fmt --check` validates handwritten Rust without rewriting checked-in
generated artifacts away from freshness.
The string-emission migration is complete: every emitted section — the
declaration surface AND the runtime/plane/runner/codec/projection support AND
the upgrade-migration modules (`migration.rs`) — renders itself through a
`ToTokens` wrapper noun routed through a single `prettyplease` pass. There is
no `self.line`/`format!`-built Rust source left in either emitter. The former
`RustWriter` string god-struct is gone, replaced by `RustModuleRenderer`: a
render driver that owns the emission context and the schema-analysis predicates
(which sections emit) but builds no Rust as strings. The only direct text it
writes is the leading `// @generated` header comment, which cannot pass through
`prettyplease` because `prettyplease` drops non-doc comments. (Closes the
migration debt named in Spirit records `0bw0` and `o7a3`, both High certainty.)
Per Spirit records `4np2` (VeryHigh), `de8i` / `e6v5` (High), and `vez8`
(Maximum): the hand-rolled string emitter is replaced ENTIRELY; lowering is
methods / `ToTokens` on the schema and Rust-model nouns; and cross-object logic
is named (e.g. `ShortHeader`, `RouteName`, the `split_*_arms` projection
builders) rather than smeared across a god-struct.

*Context stays contextual while nouns own intrinsic shape.* Intrinsic properties
such as declaration visibility and field names belong on the Rust-model nouns.
Generation-wide options such as the NOTA text feature gate or the selected
runtime target are contextual and must not be duplicated into every noun.
Context-carrying token wrappers are the correct bridge: each wrapper implements
`ToTokens` for the noun in a `RustRenderContext`, while the noun itself stays a
clean model of its own syntax.

*NOTA text projection is opted into per-emission target.* Generated binaries
always carry rkyv support. `nota_next::NotaDecode` and
`nota_next::NotaEncode` are feature-gated (`nota-text`) or omitted for
binary-only daemon consumers. A binary-only daemon crate builds dependencies
with `default-features = false` and carries no `nota_next` in its dependency
closure.

*Schema aliases and newtypes are separate data shapes.* Bare bindings lower to
`TypeDeclaration::Alias` and emit as Rust `type` aliases. Brace-body
declarations with exactly one field lower to `TypeDeclaration::Newtype` and
emit as tuple newtypes.

*Authored schema macro syntax is not an emitter input.* Tests lower real
`.schema` fixtures into typed `Schema` values before comparing generated Rust.
No assembled-schema text fixture is accepted as a normal input.

This repository owns the Rust code-generation step and the shared build-driver
orchestrator. It does not define schema semantics.
