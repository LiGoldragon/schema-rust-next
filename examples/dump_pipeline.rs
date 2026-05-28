//! Throwaway end-to-end dump (designer report 418).
//!
//! Lowers each meaty input schema with the real `SchemaEngine`, prints
//! the assembled `Asschema` with `{:#?}`, then runs the real
//! `RustEmitter` and prints the generated Rust. Nothing here is
//! hand-authored output — every block printed is produced by running the
//! engine + emitter. The report pastes this stdout verbatim.
//!
//! Run: `cargo run --example dump_pipeline`

use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

struct PipelineDump {
    title: &'static str,
    identity: &'static str,
    source: &'static str,
}

impl PipelineDump {
    fn run(&self) {
        println!("################################################################");
        println!("# SCHEMA: {}", self.title);
        println!("# identity: {}", self.identity);
        println!("################################################################");
        println!();
        println!("----- STAGE 1: input .schema -----");
        println!("{}", self.source.trim());
        println!();

        let asschema = SchemaEngine::default()
            .lower_source(self.source, SchemaIdentity::new(self.identity, "0.1.0"))
            .expect("schema lowers");

        println!("----- STAGE 2: assembled Asschema (engine output, {{:#?}}) -----");
        println!("{asschema:#?}");
        println!();

        let code = RustEmitter::default().emit(&asschema);
        println!("----- STAGE 3: generated Rust (emitter output) -----");
        println!("{}", code.as_str());
        println!();
    }
}

fn main() {
    let dumps = [
        PipelineDump {
            title: "spirit intent record store",
            identity: "spirit:intent",
            source: SPIRIT_INTENT,
        },
        PipelineDump {
            title: "horizon cluster proposal (collections-heavy)",
            identity: "horizon:cluster",
            source: HORIZON_CLUSTER,
        },
        PipelineDump {
            title: "reactive component (multi-op, *-variants)",
            identity: "reactor:component",
            source: REACTOR_COMPONENT,
        },
    ];
    for dump in &dumps {
        dump.run();
    }
}

/// A spirit intent record store. Exercises: newtypes over scalars, a
/// struct with an `@Option` field and an `@Vec` field, an enum with
/// `*`-suffix same-name-payload variants (`Decision*` etc.), and a
/// root Input/Output signal plane carrying payloads.
const SPIRIT_INTENT: &str = "\
{}
(Input (Record Entry) (Supersede SupersedeRequest) Snapshot)
(Output (Recorded RecordIdentifier) (Restored Snapshot))
{
  RecordIdentifier [Integer]
  Topic [Text]
  Author [Text]
  Body [Text]
  Magnitude (Maximum High Medium Low)
  Decision [Text]
  Principle [Text]
  Correction [Text]
  Kind (Decision* Principle* Correction*)
  Entry [(topic Topic) (author Author) (body Body) (magnitude Magnitude) (kind Kind) (supersedes (@Option RecordIdentifier))]
  SupersedeRequest [(target RecordIdentifier) (replacement Entry)]
  Snapshot [(entries (@Vec Entry))]
}
";

/// A horizon cluster proposal. Collections-heavy: `@Vec`, `@Option`,
/// `@KeyValue`, plus a nested `@KeyValue NodeName (@Vec Service)` and a
/// `@Vec (@Option Endpoint)`. Root enums use the `*`-suffix sugar on
/// some variants and explicit payloads on others.
const HORIZON_CLUSTER: &str = "\
{}
(Input (Propose Proposal) Drain* (Observe Query))
(Output (Accepted Placement) (Listed (@Vec NodeName)))
{
  NodeName [Text]
  Service [Text]
  Endpoint [Text]
  Replicas [Integer]
  Query [Text]
  Drain [NodeName]
  ServicePlan [(service Service) (replicas Replicas) (endpoints (@Vec (@Option Endpoint)))]
  Proposal [(name NodeName) (services (@Vec ServicePlan)) (cache (@Option Endpoint))]
  Placement [(assignments (@KeyValue NodeName (@Vec Service))) (configs (@KeyValue NodeName Endpoint))]
}
";

/// A reactive multi-operation component. Several root-enum operations,
/// the `*`-suffix sugar used heavily (`Parse*`, `Render*`, `Evaluate*`),
/// a stage enum mixing unit / `*` / explicit variants, and a struct
/// carrying both a vector and a map.
const REACTOR_COMPONENT: &str = "\
{}
(Input (Parse Parse) (Render Render) (Evaluate Evaluate) Reset)
(Output (Parsed Tree) (Rendered Frame) (Evaluated Value))
{
  Source [Text]
  Frame [Text]
  Value [Text]
  Parse [(source Source)]
  Render [(tree Tree) (width Integer)]
  Evaluate [(tree Tree)]
  Node [Text]
  Tree [(nodes (@Vec Node)) (attributes (@KeyValue Node Value))]
  Stage (Reset Parse* Render* Evaluate* (Wrap Tree))
}
";
