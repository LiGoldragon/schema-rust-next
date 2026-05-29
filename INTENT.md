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
schema files still provide Input and Output as the first two roots, but the
emitter consumes ordered root declarations from `Asschema` rather than a
hard-coded pair.*

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

*Rust emission consumes scalar pass-throughs from asschema as data, not as
emitter-side magic. `TypeReference::String`, `TypeReference::Integer`,
`TypeReference::Boolean`, and `TypeReference::Path` emit the scalar aliases and
shared NOTA derives directly;
`Plain(Name)` means a declared schema type or imported namespace name. The
scalar floor uses `String`, `Integer`, and `Boolean`; `Bool` is not a spelling,
and `Text` is a schema-declared newtype when a domain wants that noun.*

*Collection references emit the standard Rust collections plus their NOTA
codecs. The authored schema uses Schema type-reference vocabulary at reference
positions: `(Vec T)` emits `Vec<T>`, `(Map (K V))` emits
`std::collections::BTreeMap<K, V>` (ordered so rkyv and NOTA round-trips are
deterministic), and `(Optional T)` emits `Option<T>`; nested references
recurse. Square brackets remain raw NOTA vector structure and schema field
lists in assembled data; authored schema datatype declarations use pipe
forms, not plain square-bracket declarations. Square brackets are not `Vec`
reference syntax. The emitter writes a
shared `nota-next` codec import and derives `nota_next::NotaDecode` /
`nota_next::NotaEncode` for each generated noun, leaving only small inherent
bridge methods such as `from_nota_block` and `to_nota` on the emitted noun.
It must not hand-write per-type codec implementations. A vector value is still
a square-bracket block, a map
value is a brace of key/value pairs, and an option is `None` / `(Some inner)`,
but those value shapes live in `nota-next` rather than in a per-file generated
runtime. A type used as a map key earns the ordering derives (`PartialOrd, Ord`
on both the type and its archived form); other types keep the original derive
set.*

*NOTA owns raw delimiter structure and serialization shapes. Schema owns all
type-name keywords: scalar names such as `String`, `Integer`, and `Boolean`,
and composite names such as `Vec`, `Optional`, and `Map`. The generated NOTA
codec still serializes Rust `Vec`, `BTreeMap`, and `Option` values into NOTA
value shapes, but the names used in a `.schema` file to reference those types
are Schema vocabulary.*

*The authored schema declaration surface is the name-first `@` syntax:
`Input@[...]` and `Output@[...]` for root enums, `Name@{...}` for structs,
`Name@[...]` for enums, and `name@Type` / `name@(Composite Type)` for fields
or variant payloads. Namespace braces contain self-named declarations, not
duplicated `Name Name@{...}` key/value pairs. Rust emission should not care
which authored surface produced the assembled data: it consumes the
macro-free `Asschema` roots and type declarations.*

*The emitter starts from assembled schema data, not from authored macro syntax.
That assembled data is currently produced in memory from real `.schema`
fixtures; checked-in assembled-schema text fixtures must not remain in active
code or fixtures. Rust emission must not compensate for unresolved schema
sugar.*

*Tests should keep meaningful schema and NOTA inputs in real fixture files
under project paths and load them through a shared support surface. Inline Rust
string literals are acceptable for tiny expected source fragments, but not for
substantive `.schema` or `.nota` language examples.*

Future forge build logic may eventually turn generated Rust into
content-addressed crates directly. That is future design; this repo owns the
current explicit source emission step.
