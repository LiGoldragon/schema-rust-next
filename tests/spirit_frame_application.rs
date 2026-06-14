//! Proof for spirit's frame application (designer slice, Part 2a).
//!
//! `spirit-nexus.schema` imports the shared `Work`/`Action` frame and applies
//! it at the root positions. Emission produces spirit's payload enums plus the
//! frame-application aliases `pub type Input = Work<…>` and
//! `pub type Output = Action<…>` — NOT a concrete per-component enum body and
//! NO `into_next_step` shim. The generated module is `include!`d below over a
//! local stand-in `reaction` frame module, compiled, and a spirit `Work`
//! value (the `Input` alias) round-trips through rkyv.

use schema_next::{ImportResolver, MacroContext, SchemaEngine, SchemaIdentity};
use schema_rust_next::{RustEmissionOptions, RustEmissionTarget, RustEmitter};

mod support;

use support::FixtureSchema;

fn emit_spirit_nexus() -> String {
    let reaction_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/reaction/schema");
    let resolver = ImportResolver::new().with_dependency(
        "reaction",
        reaction_dir
            .to_str()
            .expect("reaction fixture path is utf-8"),
        "0.1.0",
    );
    let source = FixtureSchema::new("reaction/schema/spirit-nexus.schema").read();
    let mut context = MacroContext::default();
    let schema = SchemaEngine::default()
        .lower_source_with_resolver(
            &source,
            SchemaIdentity::new("spirit:nexus", "0.1.0"),
            &mut context,
            &resolver,
        )
        .expect("spirit-nexus lowers through import + root-application");
    let options = RustEmissionOptions::feature_gated_nota("nota-text")
        .with_target(RustEmissionTarget::NexusRuntime);
    RustEmitter::new(options)
        .emit_code_from_schema(&schema)
        .as_str()
        .to_owned()
}

#[test]
fn spirit_emits_frame_application_aliases_not_a_concrete_body() {
    let code = emit_spirit_nexus();

    // The imported frame heads are pulled in as the generic Work / Action.
    assert!(
        code.contains("pub use reaction::schema::reaction::Work as Work;"),
        "spirit imports the generic Work frame head:\n{code}"
    );
    assert!(
        code.contains("pub use reaction::schema::reaction::Action as Action;"),
        "spirit imports the generic Action frame head:\n{code}"
    );

    // The root positions become frame-application type aliases, binding
    // spirit's own payload vocabulary at each leg.
    assert!(
        code.contains(
            "pub type Input = Work<SignalInput, SemaWriteOutput, SemaReadOutput, EffectOutcome>;"
        ),
        "spirit Input root is the applied Work frame:\n{code}"
    );
    assert!(
        code.contains("pub type Output = Action<"),
        "spirit Output root is the applied Action frame:\n{code}"
    );

    // Spirit's payload enums emit.
    assert!(code.contains("pub enum SemaWriteSet"));
    assert!(code.contains("pub enum EffectCommand"));
    assert!(code.contains("pub enum EffectOutcome"));

    // NO concrete per-component frame enum body, NO into_next_step shim.
    assert!(
        !code.contains("pub enum NexusWork") && !code.contains("pub enum NexusAction"),
        "no concrete NexusWork / NexusAction enum body:\n{code}"
    );
    assert!(
        !code.contains("into_next_step"),
        "no into_next_step shim — the From<Action> for NextStep projection replaces it:\n{code}"
    );
}

#[test]
fn write_spirit_nexus_fixture() {
    if std::env::var_os("SCHEMA_RUST_NEXT_UPDATE_FIXTURES").is_none() {
        return;
    }
    let code = emit_spirit_nexus();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spirit_nexus_generated.rs");
    std::fs::write(path, code).expect("write spirit nexus fixture");
}

/// A local stand-in for the `reaction` frame crate at the exact module path
/// the spirit emission imports (`reaction::schema::reaction::{Work, Action}`).
/// It carries the proven generic frame enums with the proven derive stack, so
/// the included spirit module's `pub use reaction::schema::reaction::Work as
/// Work` resolves and the applied aliases type-check.
#[allow(dead_code, unused_imports)]
mod reaction {
    pub mod schema {
        pub mod reaction {
            #[cfg_attr(
                feature = "nota-text",
                derive(nota_next::NotaDecode, nota_next::NotaEncode)
            )]
            #[derive(
                rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq,
            )]
            pub enum Work<Event, Write, Read, Effect> {
                SignalArrived(Event),
                SemaWriteCompleted(Write),
                SemaReadCompleted(Read),
                EffectCompleted(Effect),
            }

            #[cfg_attr(
                feature = "nota-text",
                derive(nota_next::NotaDecode, nota_next::NotaEncode)
            )]
            #[derive(
                rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq,
            )]
            pub enum Action<Reply, Write, Read, Effect, Continuation> {
                ReplyToSignal(Reply),
                CommandSemaWrite(Write),
                CommandSemaRead(Read),
                CommandEffect(Effect),
                Continue(Continuation),
            }
        }
    }
}

#[allow(dead_code, unused_imports)]
mod spirit_nexus_generated {
    use crate::reaction;
    include!("fixtures/spirit_nexus_generated.rs");
}

#[test]
fn generated_spirit_work_round_trips_through_rkyv() {
    use spirit_nexus_generated::{Input, SemaWriteOutput, SignalInput};

    // `Input` is spirit's applied Work frame:
    // Work<SignalInput, SemaWriteOutput, SemaReadOutput, EffectOutcome>.
    let arrived: Input = Input::SignalArrived(SignalInput::new("intent recorded"));
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&arrived).expect("spirit work archives");
    let restored = rkyv::from_bytes::<Input, rkyv::rancor::Error>(&bytes)
        .expect("spirit work round-trips from rkyv bytes");
    assert_eq!(arrived, restored);

    let write_done: Input = Input::SemaWriteCompleted(SemaWriteOutput::new(true));
    let bytes =
        rkyv::to_bytes::<rkyv::rancor::Error>(&write_done).expect("write-completed archives");
    let restored = rkyv::from_bytes::<Input, rkyv::rancor::Error>(&bytes)
        .expect("write-completed round-trips");
    assert_eq!(write_done, restored);
}
