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
imports/exports, input, output, and namespace. Rust emission mirrors that
shape into source-visible modules under `src/schema/`, using single-colon
schema paths as the source naming convention and Rust modules as the emitted
form.*

*Plane payload names are scoped by emitted namespaces. The generated public
surface should read `signal::Input`, `nexus::Input`, and `sema::Input`
inside their respective planes rather than forcing redundant names like
`SemaInput` at every use site. The backing flat names may exist during the
bootstrap, but trait signatures and examples should use the plane namespace.*

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

*Nexus is also the mail keeper. When Signal input enters Nexus, it is wrapped
as `NexusMail<Payload>` with a message identifier; while Nexus owns that value,
the mail is being processed. Nexus receives SEMA or execution replies and emits
`MessageProcessed<Reply>` before the runtime translates the reply back to the
Signal output surface.*

*The async mail path is object flow. Generated `MessageSent`,
`NexusMail<Payload>`, `NexusInput`, `NexusOutput`, `SemaInput`, `SemaOutput`,
and `MessageProcessed<Reply>` are the objects the runtime acts on. The emitter
should create trait and method targets for those objects, not procedural helper
functions around them.*

*Schema-plane tests use schema-plane traits. When a schema declares
`SemaInput` and `SemaOutput`, the emitter provides the `SemaEngine` trait so
the store/state engine takes a SEMA schema object and returns a SEMA schema
object. When a schema declares `NexusInput` and `NexusOutput`, the emitter
provides a `NexusEngine` target for execution-plane object flow. Runtime tests
must invoke those generated trait surfaces rather than primitive or test-local
commands.*

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

*Collection references emit the standard Rust collections plus their NOTA
codecs. A `Vec T` reference emits `Vec<T>`, a `KeyValue K V` reference emits
`std::collections::BTreeMap<K, V>` (ordered so rkyv and NOTA round-trips are
deterministic), and an `Option T` reference emits `Option<T>`; nested
references recurse. The emitter writes a `NotaCollection` runtime codec — a
vector is a square-bracket block, a map is a brace of key/value pairs, an
option is `None` / `(Some inner)` — and the per-field parse/format
expressions recurse through it. A type used as a map key earns the ordering
derives (`PartialOrd, Ord` on both the type and its archived form); other
types keep the original derive set. The collection codec and the ordering
derives are emitted only when the schema actually uses a collection, so a
collection-free schema emits byte-identical Rust to the pre-collection
emitter.*

*The emitter starts from assembled schema data, not from authored macro syntax.
That assembled data is currently produced in memory from real `.schema`
fixtures; the old checked-in `.asschema` vector-record syntax is obsolete and
must not remain in active code or fixtures. Rust emission must not compensate
for unresolved schema sugar.*

Future forge build logic may eventually turn generated Rust into
content-addressed crates directly. That is future design; this repo owns the
current explicit source emission step.
