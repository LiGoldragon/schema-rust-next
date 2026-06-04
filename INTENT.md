# Intent

`schema-rust-next` is the Rust emission repository for the schema-derived
stack.

Psyche intent:

*The Rust emission repository for the schema-derived stack is
schema-rust-next. Rust emission is a separate step from Rust macros: the stack
generates Rust code first, and macros are a later or separate consumption
surface.*

*Generated Rust code is emitted into the consumer crate source tree under
`src/schema/`, not hidden in `OUT_DIR`. Source-visible generated interfaces are
reviewable and can become committed or freshness-checked build artifacts.*

*Schema-generated objects are the Rust nouns that carry behavior. Actor input
and output roots become enums; runtime engines implement generated Nexus
traits with one method per reaction variant, and those methods live on
data-bearing objects, not free helper functions. Nexus is the execution-IO
schema plane for internal effects, external calls, and UI surfaces.*

*Signal, Nexus, and SEMA schemas share the same authored shape:
imports/exports, roots, and namespace. Rust emission mirrors that shape into
source-visible modules under `src/schema/`, using single-colon schema paths as
the source naming convention and Rust modules as the emitted form. The current
schema files still provide Input and Output as the first two roots. In
assembled schema those roots are direct product fields, not a homogeneous
vector of wrappers; the emitter consumes `Asschema::input_and_output()` as the
two direct root enum definitions.*

*Plane payload names are scoped by emitted namespaces. The generated public
surface should read `signal::Input`, `nexus::Input`,
`sema::WriteInput`, and `sema::ReadInput` inside their respective planes
rather than forcing redundant plane ancestry at every use site. The backing
flat names may exist during bootstrap, but trait signatures and examples
should use the plane namespace.*

*Plane identity is matchable data, not a separate kind tag. When runtime code
needs to branch across Signal, Nexus, and SEMA, the emitted surface is
`schema::Plane::{Signal,Nexus,Sema}` carrying the actual plane message
envelopes. A thin unit discriminator beside the message would duplicate
authority and is not the load-bearing plane surface.*

*Cross-crate schema imports preserve type ownership. A consumer schema that
imports `crate:module:Type` emits a local Rust alias to the dependency crate's
generated type instead of re-declaring the type locally. The imported crate owns
the type definition and its rkyv/NOTA implementations; the consumer only uses
the alias in its local signal/Nexus/SEMA objects and bridges imported decode
errors into the local generated error type.*

*Nexus is also the decision and mail-keeper plane. Signal triage produces a
generated `nexus::Nexus<nexus::Input>` envelope directly; while Nexus executes
through `NexusEngine::execute`, the origin route is the return address carried
through SEMA and back into Signal. Nexus receives SEMA or execution replies and
emits `MessageProcessed<Reply>` before the runtime translates the reply back to
the Signal output surface. Old generated convenience mail wrappers do not stay
beside this working trait path.*

*The async mail path is object flow. Generated `MessageSent`, `NexusInput`,
`NexusOutput`, `SemaWriteInput`, `SemaWriteOutput`, `SemaReadInput`,
`SemaReadOutput`, and `MessageProcessed<Reply>` are the objects the runtime
acts on. The emitter should create trait and method targets for those objects,
not procedural helper functions around them.*

*Schema-plane tests use schema-plane traits. Bootstrap all-in-one runtime
emission may still provide the historical `SignalEngine` trait while that
schema shape is being split. The canonical per-plane runtime shape is narrower:
a Nexus runtime target emits a mutable `NexusEngine` target for execution-plane
object flow and heavier decision-making, and a SEMA runtime target emits
`SemaEngine`. When a schema declares a `SemaWriteInput` / `SemaWriteOutput`
pair, the emitter provides the mutable `SemaEngine::apply` path; when it
declares a `SemaReadInput` / `SemaReadOutput` pair, the emitter provides the
shared-reference `SemaEngine::observe` path. Runtime tests must invoke those
generated trait surfaces rather than primitive or test-local commands.*

*Contract and runtime emission are distinct targets. Signal and meta-signal
contract schemas are external wire surfaces; in
`RustEmissionTarget::WireContract` this emitter emits schema nouns, derives,
NOTA/rkyv codecs, short headers, and signal-frame encode/decode only. Runtime
and daemon schemas live as schema files inside the daemon crate, such as
`cloud/schema/nexus.schema` and `cloud/schema/sema.schema`.
`RustEmissionTarget::NexusRuntime` emits Nexus runtime support only, and
`RustEmissionTarget::SemaRuntime` emits SEMA runtime support only. Runtime
schemas import contract roots where the daemon needs the external wire
vocabulary. `RustEmissionTarget::ComponentRuntime` remains only as the
compatibility/bootstrap target for unsplit all-in-one schemas.*

*The emitter should move toward a generated component-runner surface for the
triad engine. A daemon `main` should eventually be a tiny call into generated
or macro-created startup code; the schema-emitted substrate defines the
programmatic Signal/Nexus/SEMA wiring. Hand-written component code supplies
non-default algorithms by implementing the generated engine traits on
data-bearing actors or stores. Heavy topic-discovery decisions belong to Nexus
implementations; durable indexes and score tables belong to SEMA; Signal stays
the communication and reply boundary.*

*The engine traits carry the minimum lifecycle address needed by the runtime
and persona supervision. `NexusEngine` and `SemaEngine` emit default no-op
`on_start` and `on_stop` methods returning typed `ActorStartFailure` and
`ActorStopFailure` results. The bootstrap all-in-one `ComponentRuntime` target
still emits the historical `SignalEngine` trait while unsplit schemas exist.
Full actor mailbox, backpressure, runtime-control, and inner-engine promotion
remain future work; the lifecycle hooks are the small addressable surface that
composes with that future without changing the engine method substrate.*

*Testing trace hooks belong to those generated engine traits. Per-plane runtime
targets provide default no-op trace hook methods and default wrapper methods
for `execute`, `apply`, and `observe`; the bootstrap all-in-one target also
provides `triage` and `reply`. Runtime actors implement the inner behavior
methods and may override one activation hook per emitted plane. Each hook
receives that plane's generated object-name enum: `NexusObjectName` or
`SemaObjectName`; bootstrap all-in-one emission also carries
`SignalObjectName`. The shared `ObjectName` enum wraps the emitted plane names
for `TraceEvent` transport. Trace identity is generated from the schema header
through route enums such as `NexusInputRoute` and `SemaReadInputRoute`, plus
generated actor-boundary names such as `NexusObjectName::Entered` and
`SemaObjectName::Stopped`. Trace events do not carry cloned input/output
payload snapshots. When a trace event's only payload is the activated object
name, the emitted `TraceEvent` is a transparent newtype over `ObjectName`, so
NOTA displays the object-name value directly instead of adding a one-field
record wrapper. A consumer should not carry parallel local Nexus/SEMA trace
traits or stringly trace vocabularies beside the generated actor/interface
contract.*

*Tracing is a schema-defined typed interface and stays typed until the client
display boundary. The emitter should eventually generate the component-specific
`TraceEventFrame` adapter for binary trace archives and a NOTA display adapter
for the generated `TraceEvent`, plus any aliases needed to use the shared
`triad-runtime` trace client. Component CLIs should not hand-write trace
listener/display logic; they use generic client support over emitted `Input`,
`Output`, and `TraceEvent` nouns. Trace is not enabled on the trace interface
itself in the current design.*

*Help and documentation are schema data, not CLI string tables. The target
shape is a mirror description namespace over the fully qualified schema symbol
namespace, with generated defaults when no explicit description exists. Help
actions or client help output project from that typed description data.*

*Schema version changes drive upgrade surfaces. If a data type has not changed,
no upgrade code is emitted for it. If it has changed, the generated noun exposes
an upgrade/accept trait that hand-written runtime code implements, including
observable acceptance of old-version messages.*

*Signal messages participate in a universal mail mechanism. Sending a generated
root message creates a typed `MessageSent` event with the message identifier,
origin route, root schema type, and short header, and the event is pushed
through hook methods so observers can react without polling. The origin route is
an automatically-created field on the root Signal, Nexus, and SEMA message
objects; authored component schemas do not have to spell it out, but the
generated message object always carries it as part of the message while it moves
through the runtime chain. The origin route is minted distinctly from the
message identifier.*

*Mail lifecycle support should stay on the schema scalar floor. Generated
`MessageIdentifier`, `OriginRoute`, and `MessageSent.short_header` use
`Integer`, not bespoke primitive widths, while the mail support surface is being
moved toward a shared schema-authored core.*

*Rust emission consumes scalar pass-throughs from asschema as data, not as
emitter-side magic. `TypeReference::String`, `TypeReference::Integer`,
`TypeReference::Boolean`, and `TypeReference::Path` emit the scalar aliases.
`Plain(Name)` means a declared schema type or imported namespace name. The
scalar floor uses `String`, `Integer`, and `Boolean`; `Bool` is not a spelling,
and `Text` is a schema-declared newtype when a domain wants that noun. Binary
`rkyv` support is universal; NOTA encode/decode support is an optional emitted
surface for text-facing clients, not a daemon-default surface.*

*Collection references emit the standard Rust collections plus their NOTA
codecs. The authored schema uses Schema type-reference vocabulary at reference
positions: `(Vec T)` emits `Vec<T>`, `(Map (K V))` emits
`std::collections::BTreeMap<K, V>` (ordered so rkyv and NOTA round-trips are
deterministic), and `(Optional T)` emits `Option<T>`; nested references
recurse. Square brackets remain raw NOTA vector structure and, when paired
with `@`, enum declaration bodies; authored schema datatype declarations use
name-first `@` forms, not plain square-bracket declarations. Square brackets
are not `Vec` reference syntax. The emitter can write a shared `nota-next`
codec import and derive `nota_next::NotaDecode` / `nota_next::NotaEncode` for
generated nouns, but that surface is selected explicitly: always enabled,
feature-gated for text clients, or omitted for binary-only daemon consumers.
When NOTA is selected, only small inherent bridge methods such as
`from_nota_block` and `to_nota` live on the emitted noun. It must not hand-write
per-type codec implementations. A vector value is still a square-bracket block,
a map value is a brace of key/value pairs, and an option is `None` / `(Some
inner)`, but those value shapes live in `nota-next` rather than in a per-file
generated runtime. A type used as a map key earns the ordering derives (`PartialOrd, Ord`
on both the type and its archived form); other types keep the original derive
set.*

*The codec opt-in is configured through `RustEmissionOptions` passed to
`RustEmitter::new`. `nota_surface` selects the NOTA text projection:
`RustEmissionOptions::default()` and `RustEmitter::default()` use
`NotaSurface::FeatureGated { feature: "nota-text" }`, so emitted source carries
the NOTA derives, `use nota_next::*` items, inherent bridges, and root
`FromStr` / `Display` impls behind `#[cfg_attr(feature = "nota-text", …)]` /
`#[cfg(feature = "nota-text")]`. A text-facing client crate enables
`nota-text`; a binary-only daemon crate builds generated contract dependencies
with `default-features = false` and carries no `nota_next` in its dependency
closure. `NotaSurface::Disabled` omits the NOTA surface entirely. `target`
selects whether the generated file is a `WireContract`, `NexusRuntime`, or
`SemaRuntime`; the default target remains `ComponentRuntime` for compatibility
while existing unsplit runtime schemas are being retired.*

*NOTA owns raw delimiter structure and serialization shapes. Schema owns all
type-name keywords: scalar names such as `String`, `Integer`, and `Boolean`,
and composite names such as `Vec`, `Optional`, and `Map`. The generated NOTA
codec still serializes Rust `Vec`, `BTreeMap`, and `Option` values into NOTA
value shapes, but the names used in a `.schema` file to reference those types
are Schema vocabulary.*

*Authored enum-body spelling belongs to schema-next, not this emitter. Rust
emission consumes the macro-free `Asschema` roots and type declarations, so it
must not grow a parser for any authored spelling. When source sugar creates
exported bare bindings such as `Rejected SignalRejection` or `Record Entry`,
the assembled schema carries `TypeDeclaration::Alias` and the emitter writes a
Rust `type` alias. The root enum keeps the exported noun name, but Rust callers
pass the underlying payload directly.*

*Asschema aliases and newtypes are separate data shapes. A bare binding lowers
to `TypeDeclaration::Alias` and emits as `pub type Topic = String;` or
`pub type Rejected = SignalRejection;`. A source declaration with a brace body
and exactly one field lowers to `TypeDeclaration::Newtype`; the emitter
projects that shape directly to an ergonomic Rust tuple newtype. Multi-field
structs keep named fields.*

*Generated Rust should not force consumers to hand-write wrapper stacks. Tuple
newtypes emit `new`, `payload`, and `into_payload` methods plus `From<Payload>`;
enums emit variant-named associated constructors such as `Input::record(entry)`
and `Output::rejected(signal_rejection)`. If an enum variant stores an exported
newtype wrapper, the constructor accepts the wrapper's inner payload and builds
the wrapper internally. The wrapper noun remains real in the type system, but
ordinary code names the operation once. If the variant stores an alias, there
is no wrapper to construct and the constructor accepts the alias target
directly.*

*The emitter starts from assembled schema data, not from authored macro syntax.
That assembled data is live: it can be written as NOTA, read back, written as
rkyv bytes, and read back before emission. Checked-in assembled-schema text
fixtures must not remain in active code or fixtures. Rust emission must not
compensate for unresolved schema sugar.*

*Rust emission can consume the assembled schema as an explicit artifact. The
emitter still accepts `&Asschema` for in-process callers, but it also accepts
`AsschemaArtifact` and `.asschema` / `.asschema.rkyv` file paths, so build
pipelines can prove the handoff is serialized data before generated Rust
appears.*

*Rust emission is data before it is text. The emitter maps `Asschema` into a
typed `RustModule` object carrying scalar aliases, cross-crate imports, Rust
declarations, root enums, and support metadata; rendering `RustModule` produces
`RustCode`. Tests assert the module data shape directly so the core mapping is
not hidden inside string-writer side effects.*

*Asschema declaration visibility is a code-generation boundary. A public
declaration is exported Rust API. A private declaration is a module-local Rust
noun, emitted as `pub(crate)`, so inline PascalCase schema types can support a
containing public type without becoming part of the importable schema library
surface.*

*Tests should keep meaningful schema and NOTA inputs in real fixture files
under project paths and load them through a shared support surface. Inline Rust
string literals are acceptable for tiny expected source fragments, but not for
substantive `.schema` or `.nota` language examples.*

Future forge build logic may eventually turn generated Rust into
content-addressed crates directly. That is future design; this repo owns the
current explicit source emission step.
