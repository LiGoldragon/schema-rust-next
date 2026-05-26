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
    assert_eq!(
        generated.code.as_str(),
        include_str!("fixtures/spirit_generated.rs")
    );
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
