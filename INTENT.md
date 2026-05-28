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
signal root creates a typed `MessageSent` event with the message identifier,
root schema type, and short header, and the event is pushed through hook methods
so observers can react without polling.*

*Mail lifecycle support should stay on the schema scalar floor. Generated
`MessageIdentifier` and `MessageSent.short_header` use `Integer`, not bespoke
primitive widths, while the mail support surface is being moved toward a shared
schema-authored core.*

Future forge build logic may eventually turn generated Rust into
content-addressed crates directly. That is future design; this repo owns the
current explicit source emission step.

Cross-crate import emission (Spirit record 1009, 2026-05-28):

*An imported type is REFERENCED, never re-declared.* When the assembled schema
carries resolved imports, the emitter writes a `pub use` alias to the
dependency crate's emitted type
(`pub use crate::schema::module::Type as Local;`) instead of emitting a fresh
struct/enum. The dependency crate owns the type's definition and its rkyv/NOTA
impls; the importing module reaches them through the alias. One type identity
crosses the crate boundary — this is the cross-crate realisation of "schema
types are the nouns; don't hand-write a parallel mirror."

Because each emitted module declares its own `NotaDecodeError`, the emitter
also writes `impl From<<dep-module>::NotaDecodeError> for NotaDecodeError` for
each distinct imported module, so a local NOTA codec calling an imported type's
`from_nota_block` composes under the `?` operator.
