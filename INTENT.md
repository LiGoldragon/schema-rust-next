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

## Continuous manifestation

Per spirit record 944 (Maximum, 2026-05-27): this `INTENT.md` is
maintained continuously. See
`~/primary/skills/repo-intent.md` §"Continuous manifestation discipline".

Future forge build logic may eventually turn generated Rust into
content-addressed crates directly. That is future design; this repo owns the
current explicit source emission step.
