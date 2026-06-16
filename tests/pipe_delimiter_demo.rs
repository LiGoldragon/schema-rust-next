//! End-to-end demonstration of the schema-language vision in pipe-delimiter
//! syntax: a generic reaction frame declared once with `(| [params] body |)`,
//! a component that binds it `(Work …)` and EXPANDS to concrete enums, payload
//! types of every kind (struct / enum / newtype), a MARKER impl `{| Trait
//! Target |}`, and a `Deref` impl `{| Deref Target [ (deref (reference (field
//! self payload))) ] |}` whose body is emitted from a code-is-data expression
//! tree. The generated Rust is asserted structurally, then `include!`d,
//! compiled, and exercised: the expanded enums round-trip through rkyv and
//! NOTA, the newtype derefs to its inner payload (proving the data-emitted
//! body), and a function bounded on the marker trait accepts the marked type.

use schema_next::{ImportResolver, MacroContext, SchemaEngine, SchemaIdentity};
use schema_rust_next::{RustEmissionOptions, RustEmissionTarget, RustEmitter};

mod support;

use support::FixtureSchema;

fn emit_ledger() -> String {
    let reaction_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pipe-demo/schema");
    let resolver = ImportResolver::new().with_dependency(
        "reaction",
        reaction_dir.to_str().expect("reaction fixture path is utf-8"),
        "0.1.0",
    );
    let source = FixtureSchema::new("pipe-demo/schema/ledger.schema").read();
    let mut context = MacroContext::default();
    let schema = SchemaEngine::default()
        .lower_source_with_resolver(
            &source,
            SchemaIdentity::new("ledger:core", "0.1.0"),
            &mut context,
            &resolver,
        )
        .expect("ledger lowers through pipe-delimiter generics + impls");
    let options = RustEmissionOptions::feature_gated_nota("nota-text")
        .with_target(RustEmissionTarget::NexusRuntime);
    RustEmitter::new(options)
        .emit_code_from_schema(&schema)
        .as_str()
        .to_owned()
}

#[test]
fn generic_declaration_expands_to_concrete_root_enums() {
    let code = emit_ledger();

    // The `(Work …)` / `(Action …)` bindings of the `(| |)`-declared frames
    // expand into CONCRETE enums — not `pub type Input = Work<…>` aliases.
    assert!(!code.contains("pub type Input ="), "no Input alias:\n{code}");
    assert!(!code.contains("pub type Output ="), "no Output alias:\n{code}");
    assert!(code.contains("pub enum Input {"), "concrete Input enum:\n{code}");
    assert!(code.contains("pub enum Output {"), "concrete Output enum:\n{code}");

    for leg in [
        "SignalArrived(SignalInput)",
        "SemaWriteCompleted(SemaWriteOutput)",
        "SemaReadCompleted(SemaReadOutput)",
        "EffectCompleted(EffectOutcome)",
    ] {
        assert!(code.contains(leg), "Input carries leg {leg}:\n{code}");
    }
    for leg in [
        "ReplyToSignal(SignalOutput)",
        "CommandSemaWrite(SemaWriteSet)",
        "CommandSemaRead(SemaReadInput)",
        "CommandEffect(EffectCommand)",
        "Continue(Input)",
    ] {
        assert!(code.contains(leg), "Output carries leg {leg}:\n{code}");
    }

    // Concrete enums flow through the concrete-enum emitters: constructors + From.
    assert!(
        code.contains("impl Input {") && code.contains("pub fn signal_arrived(payload"),
        "Input gains constructors:\n{code}"
    );
    assert!(
        code.contains("impl From<Input> for Output"),
        "Output's Continue leg gains From<Input>:\n{code}"
    );
}

#[test]
fn payload_types_of_each_kind_emit() {
    let code = emit_ledger();

    // struct
    assert!(
        code.contains("pub struct LedgerEntry {"),
        "struct payload emits:\n{code}"
    );
    // enum
    assert!(
        code.contains("pub enum SemaWriteSet {") && code.contains("pub enum EffectCommand {"),
        "enum payloads emit:\n{code}"
    );
    // newtype
    assert!(
        code.contains("pub struct EntryHandle(Statement)")
            || code.contains("pub struct EntryHandle (Statement)"),
        "newtype payload emits:\n{code}"
    );
}

#[test]
fn marker_impl_emits_empty_impl_block() {
    let code = emit_ledger();
    assert!(
        code.contains("impl Auditable for EntryHandle {}"),
        "marker impl emits an empty impl block:\n{code}"
    );
}

#[test]
fn deref_impl_emits_from_code_is_data_body() {
    let code = emit_ledger();
    // The Deref impl block, with the body PROJECTED from the expression tree
    // `(reference (field self payload))` → `&self.0`.
    assert!(
        code.contains("impl std::ops::Deref for EntryHandle"),
        "Deref impl block emits:\n{code}"
    );
    assert!(
        code.contains("type Target = Statement"),
        "Deref Target is the newtype inner type:\n{code}"
    );
    assert!(
        code.contains("fn deref(&self) -> &Self::Target"),
        "Deref method signature emits:\n{code}"
    );
    // The body came from data: `&self.0`.
    let normalized: String = code.split_whitespace().collect();
    assert!(
        normalized.contains("&self.0"),
        "Deref body is the data-emitted &self.0:\n{code}"
    );
}

#[test]
fn write_ledger_fixture() {
    if std::env::var_os("SCHEMA_RUST_NEXT_UPDATE_FIXTURES").is_none() {
        return;
    }
    let code = emit_ledger();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pipe_demo_ledger_generated.rs");
    std::fs::write(path, code).expect("write ledger fixture");
}

/// A local stand-in for the `reaction` frame crate at the module path the
/// emission's `pub use reaction::schema::reaction::{Work, Action}` import lines
/// reference. The roots are expanded into concrete enums, so the generic frame
/// enums are not load-bearing for the root types, but the emitter still
/// re-exports the resolved imports, so the stand-in keeps them defined.
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

/// The local marker trait the `{| Auditable EntryHandle |}` marker impl targets.
/// A self-contained trait — the demonstration needs no external crate.
#[allow(dead_code)]
pub trait Auditable {}

#[allow(dead_code, unused_imports)]
mod ledger_generated {
    use crate::reaction;
    use crate::Auditable;
    include!("fixtures/pipe_demo_ledger_generated.rs");
}

#[test]
fn expanded_input_round_trips_through_rkyv() {
    use ledger_generated::{Input, SignalInput};

    let arrived: Input = Input::signal_arrived("ledger opened".to_owned());
    assert_eq!(
        arrived,
        Input::SignalArrived(SignalInput::new("ledger opened"))
    );
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&arrived).expect("archives");
    let restored =
        rkyv::from_bytes::<Input, rkyv::rancor::Error>(&bytes).expect("rkyv round-trips");
    assert_eq!(arrived, restored);
}

#[test]
fn expanded_output_recursive_continue_round_trips_through_rkyv() {
    use ledger_generated::{Input, Output, SignalInput};

    let inner: Input = Input::SignalArrived(SignalInput::new("continue"));
    let next: Output = Output::r#continue(inner.clone());
    assert_eq!(Output::from(inner), next);
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&next).expect("archives");
    let restored =
        rkyv::from_bytes::<Output, rkyv::rancor::Error>(&bytes).expect("rkyv round-trips");
    assert_eq!(next, restored);
}

#[test]
fn deref_impl_returns_inner_payload() {
    use ledger_generated::{EntryHandle, Statement};

    // Construct the newtype, then deref through the DATA-EMITTED `Deref` body.
    // If the body `&self.0` were wrong, this would not compile or would return
    // the wrong value — so the assertion proves the code-is-data body is right.
    let statement = Statement::new("recorded".to_owned());
    let handle = EntryHandle::new(statement.clone());
    let derefed: &Statement = &handle; // uses Deref
    assert_eq!(derefed, &statement);
    // Reaching the inner type's accessor THROUGH the deref (explicit `*` so the
    // EntryHandle's own inherent `payload` doesn't shadow it): this exercises
    // the data-emitted `deref` body `&self.0`, returning the inner Statement,
    // whose `payload()` is the original String.
    assert_eq!((*handle).payload(), statement.payload());
}

#[test]
fn marker_impl_admits_a_trait_bounded_function() {
    use crate::Auditable;
    use ledger_generated::{EntryHandle, Statement};

    // A function bounded on the marker trait — compile-proof that the marker
    // impl `impl Auditable for EntryHandle {}` is present and effective.
    fn audited<T: Auditable>(value: T) -> T {
        value
    }
    let handle = EntryHandle::new(Statement::new("audited".to_owned()));
    let _back = audited(handle);
}

#[cfg(feature = "nota-text")]
#[test]
fn expanded_enums_round_trip_through_nota() {
    use nota_next::{NotaEncode, NotaSource};
    use ledger_generated::{Input, Output, SignalInput};

    let arrived: Input = Input::signal_arrived("recorded".to_owned());
    let rendered = arrived.to_nota();
    let parsed = NotaSource::new(&rendered)
        .parse::<Input>()
        .expect("Input parses back from NOTA");
    assert_eq!(arrived, parsed);

    let next: Output = Output::Continue(Input::SignalArrived(SignalInput::new("again")));
    let rendered = next.to_nota();
    let parsed = NotaSource::new(&rendered)
        .parse::<Output>()
        .expect("Output parses back from NOTA");
    assert_eq!(next, parsed);
}
