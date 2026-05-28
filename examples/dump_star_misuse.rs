//! Throwaway finding-capture (designer report 418).
//!
//! The `*` same-name-payload sugar is recognised only on a BARE variant
//! tag (`Decision*` → `(Decision Decision)`). When `*` is attached to
//! the tag inside an EXPLICIT `(Tag Payload)` pair — `(Restored*
//! Snapshot)` — the parenthesised-variant path lowers the head through
//! `schema_name` and the `*` is swallowed into the variant NAME rather
//! than rejected. This dump captures that real behaviour: the assembled
//! enum variant is literally named `Restored*`, which the emitter then
//! writes verbatim into Rust identifiers (`Restored*(Snapshot)`,
//! `OutputRoute::Restored*`) — code that cannot compile.
//!
//! Run: `cargo run --example dump_star_misuse`

use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

fn main() {
    let source = "\
{}
(Input ())
(Output (Restored* Snapshot))
{
  Entry [Text]
  Snapshot [(entries (@Vec Entry))]
}
";
    println!("----- input .schema (deliberate `*`-in-parens misuse) -----");
    println!("{}", source.trim());
    println!();

    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("misuse:demo", "0.1.0"))
        .expect("schema still lowers — the `*` is NOT rejected");

    println!("----- assembled Output enum (engine output, {{:#?}}) -----");
    println!("{:#?}", asschema.output());
    println!();

    // Show the lines of generated Rust that carry the broken identifier.
    let code = RustEmitter::default().emit(&asschema);
    println!("----- generated Rust lines containing the broken identifier -----");
    for line in code.as_str().lines() {
        if line.contains("Restored") {
            println!("{line}");
        }
    }
}
