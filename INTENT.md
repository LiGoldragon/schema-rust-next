# Intent

`schema-rust-next` is the Rust emission repository for the schema-derived
stack.

Psyche intent:

*The Rust emission repository for the schema-derived stack is
schema-rust-next. Rust emission is a separate step from Rust macros: the stack
generates Rust code first, and macros are a later or separate consumption
surface.*

## Emission target — src/schema in the consumer crate

Per spirit record 909 (Maximum, 2026-05-27):

*"schema-derived Rust code emits to src/schema/lib.rs and
src/schema/[module].rs in the crate source tree NOT to OUT_DIR/schema;
this matches the literal wording of record 902 and is the load-bearing
choice for visibility and grep-ability - the schema-derived Rust lives
alongside hand-written Rust in the same crate src directory and can be
read by humans and tools without rebuilding."*

Per spirit record 910 (Maximum, 2026-05-27):

*"In the current schema-stack version, schema-generated Rust should
materialize under src/schema/<module>.rs rather than remain only an
OUT_DIR future target."*

Per record 902 (Maximum): *"Rust emission target is another folder in
the crate source called src/schema/ producing src/schema/lib.rs etc -
the schema-derived Rust code lives next to hand-written Rust and gets
re-emitted automatically; development hot-reload via a watch hook on the
schema files."*

The src/schema target is the load-bearing choice for visibility and
grep-ability. Generated content can be committed or gitignored per
workspace policy, but the path is fixed.

## Methods on non-ZST types

Per spirit records 712 (Maximum, 2026-05-26) and 882 (Maximum,
2026-05-27):

*"Every Rust function is a method or associated function on an impl block
of a non-zero-sized data-bearing type, or a trait impl. Free functions
are forbidden except in #[cfg(test)] modules and fn main(). Methods on
zero-sized placeholder types used as a namespace are equally forbidden —
that's a free function in disguise."*

Emitted Rust follows the same rule: macros emit methods inside `impl`
blocks of data-bearing types they emit, never free helpers. Trait impls
are preferred for projection / conversion (`impl From<X> for Y` over
`fn project_x_to_y`). The emitter itself (the hand-written Rust in this
crate) follows the same discipline.

## No proc-macro / macro_rules surface

The emitter generates Rust source text directly. No `macro_rules!` or
proc-macro surface in `src/`. Schema macros are a separate schema-layer
mechanism (record 932) and live in `schema-next`; this crate is the
source-text emission step downstream of `schema-next::Asschema`.

## Emits Rust for three schema types

Per spirit record 964 (Maximum, 2026-05-27):

*"the schema layer has THREE SCHEMA TYPES corresponding to the three
runtime planes: SIGNAL schemas (wire and communication layer); NEXUS
schemas (execution IO and UI layer - previously named Executor in
record 371s runtime triad framing); SEMA schemas (durable state
layer the database); each has its own engine with its own traits but
all three engines share the pattern of running code based on input
message and returning output message with populated data; each
schema declares its own input and output enums per records 933 and
940 uses namespace imports for shared types per record 902 colon-path
and emits Rust types and traits via schema-rust-next."*

`schema-rust-next` is the emission target for **all three schema
types**. Each schema document declares its plane (Signal / Nexus /
Sema) and schema-rust-next emits Rust source carrying the
plane-appropriate input/output enums, payload types, and trait
surface. The emission pattern — `src/schema/lib.rs` + per-module
files — is the same regardless of plane.

The **root type** of a schema is the message type; emitted Rust
attaches encode/decode and Communicate-trait methods to that root.

**File extensions are open** per record 964: candidates include
`.signal.schema` / `.nexus.schema` / `.sema.schema`, OR the variant
as the first record of the schema content. Not yet locked at the
emitter side either.

Per record 965: Nexus schemas drive ANY layer where code runs on
typed input and returns typed output — internal IO, external CLI
calls, AND all UI panels (Mencie is implemented as nexus schemas).

Per record 970 (Maximum, 2026-05-27): the three schema types map
to the daemon's **THREE EXECUTION CENTERS** — Signal (communication),
Nexus (execution + mail keeper + translator), SEMA (state). The
emission target for Nexus schemas includes the mail-keeping surface
(NexusMail<Payload>, MessageIdentifier, lifecycle hooks); the
emission target for Signal schemas includes the on_sent hook surface
(record 963); the emission target for Sema schemas includes the
database-marker reply layer (record 935). Record 970 CONSOLIDATES
records 935 + 963 + 964 + 965 — the emitted Rust per plane reflects
each plane's role in the flow.

## Schema and emitted Rust mirror each other

Per spirit record 952 (High, 2026-05-27): the naming system between
schema-emitted code and Rust source **mirrors each other**. *"You
can use the naming system that way to like a mirror."* The
colon-path namespace in a schema (e.g. `spirit-next:signal:Frame`)
maps directly to the Rust module-and-type path
(`spirit_next::signal::Frame`):

- colon `:` becomes double colon `::`
- kebab-case crate names become snake_case module names
- PascalCase type names stay unchanged

Two consequences for this crate's emission discipline:

1. **Emit names identical to the schema's identifier (modulo Rust's
   case rules).** A schema position named `signal:Frame` emits a
   Rust type at module path `signal::Frame` — the same identifier
   in both views.
2. **The schema and the emitted Rust become navigable from either
   side.** An agent reading the schema can grep the emitted Rust
   for the same name; an agent reading the Rust can find the
   schema definition the same way. The mirror property + the
   side-by-side file placement (per record 909) means the two
   surfaces sit together AND read with the same identifiers — one
   identity in two views.

The mirror is load-bearing for the workspace's introspection
property (per `~/primary/ESSENCE.md` §"What I am building"): the
structure of the system is the documentation of itself, and the
mirror makes both halves of the structure visible.

## Recurring patterns realised in this repo

Per spirit record 988 (Maximum, 2026-05-27) + workspace INTENT.md
§"Recurring architectural patterns": schema-rust-next is the
Rust emission substrate and realises:

- **Pattern B — Three execution centers (Signal + Nexus + SEMA).**
  This crate is the emission target for ALL three schema types;
  each plane's emitted Rust carries plane-appropriate
  input/output enums, payload types, and trait surfaces (records
  964, 970).
- **Pattern C — Methods on schema-generated data types.** See
  §"Methods on non-ZST types" above — the emitter generates
  methods inside `impl` blocks of data-bearing types, never free
  helpers (records 712, 882).
- **Pattern F — Mirror naming.** See §"Schema and emitted Rust
  mirror each other" above — this is the load-bearing emission
  rule that realises pattern F.

## Continuous manifestation

Per spirit record 944 (Maximum, 2026-05-27): this `INTENT.md` is
maintained continuously. See
`~/primary/skills/repo-intent.md` §"Continuous manifestation discipline".

Future forge build logic may eventually turn generated Rust into
content-addressed crates directly. That is future design; this repo owns the
current explicit source emission step.
