use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

#[allow(dead_code)]
mod generated {
    include!("fixtures/spirit_generated.rs");
}

#[test]
fn emits_rust_source_as_a_separate_artifact() {
    let source = include_str!("fixtures/spirit-min.schema");
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("spirit", "0.1.0"))
        .expect("schema lowers");
    let generated = RustEmitter.emit_file(&asschema);

    assert_eq!(generated.path, "spirit.rs");
    assert!(generated.code.as_str().contains("pub enum Input"));
    assert!(
        generated
            .code
            .as_str()
            .contains("impl std::str::FromStr for Input")
    );
    assert!(generated.code.as_str().contains("rkyv::Archive"));
    assert!(generated.code.as_str().contains("pub mod short_header"));
}

#[test]
fn compiled_fixture_is_usable_rust() {
    let entry = generated::Entry {
        topics: generated::Topics(generated::Topic(String::from("schema"))),
        kind: generated::Kind::Decision,
        description: generated::Description(String::from("schema drives rust")),
        magnitude: generated::Magnitude::Maximum,
    };
    let input = generated::Input::Record(entry);

    assert!(matches!(input, generated::Input::Record(_)));
    assert_eq!(generated::short_header::INPUT_RECORD, 0x0000_0000_0000_0000);
    assert_eq!(
        generated::short_header::INPUT_OBSERVE,
        0x0001_0000_0000_0000
    );
}

#[test]
fn generated_input_parses_cli_nota_and_emits_nota() {
    let input = "(Record ([schema] Constraint [agent's clarified intent] Maximum))"
        .parse::<generated::Input>()
        .expect("parse generated input");

    match &input {
        generated::Input::Record(entry) => {
            assert_eq!(entry.topics.0.0, "schema");
            assert_eq!(entry.kind, generated::Kind::Constraint);
            assert_eq!(entry.description.0, "agent's clarified intent");
            assert_eq!(entry.magnitude, generated::Magnitude::Maximum);
        }
        generated::Input::Observe(_) => panic!("expected record"),
    }

    assert_eq!(
        input.to_string(),
        "(Record ([schema] Constraint [agent's clarified intent] Maximum))"
    );
}

#[test]
fn generated_signal_input_round_trips_from_nota_to_rkyv_bytes() {
    let input = "(Record ([schema] Constraint [component messages use binary rkyv] Maximum))"
        .parse::<generated::Input>()
        .expect("parse generated input");

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input).expect("archive input");
    let decoded =
        rkyv::from_bytes::<generated::Input, rkyv::rancor::Error>(&bytes).expect("decode input");

    assert_eq!(decoded, input);
}
