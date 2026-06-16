//! End-to-end proof of the composition closure (`d3r2`): a method body that
//! COMPOSES calls to shape-implied primitives is DATA, not hand-written. The
//! real component method `ConfigurationPath::as_str` — currently hand-written at
//! `signal-spirit/src/lib.rs:26` as `self.payload().as_str()` — is declared as
//! the NOTA expression tree `(call (call (self) payload) as_str)`, parsed
//! through the real `SchemaEngine`, projected by `Expression::to_rust()`, and
//! the projected body is compiled and RUN to prove behavior.
//!
//! The composition node is `Expression::MethodCall(receiver, method, args)`
//! (`schema-next/.../schema.rs`). It is NOT an expression compiler: the call
//! head must resolve against the closed shape-implied primitive alphabet
//! (`ComposablePrimitive`: `payload`, `into_payload`, `as_str`). The first call
//! that resolves to no primitive is the business-logic boundary — rejected with
//! the typed `SchemaError::UnresolvedComposition`, never a wrong body, never a
//! panic. The negative test feeds the real business-logic method `keywords`
//! (`signal-spirit/src/lib.rs:647`) and asserts the rejection fires.

use schema_next::{Expression, ImplBody, MethodDeclaration, Name, SchemaError};
use schema_rust_next::{RustEmissionOptions, RustEmissionTarget, RustEmitter};

mod support;

use support::FixtureSchema;

/// Emit the composition-demo schema to Rust source through the real emitter. The
/// `ConfigurationPath` newtype flows through the standard newtype emitter
/// (`new` / `payload` / `into_payload`), and the `Composed` impl emits an
/// inherent `impl ConfigurationPath { pub fn as_str(&self) -> &str { … } }`
/// whose body is the projected composition.
fn emit_configuration() -> String {
    let schema = FixtureSchema::new("composition-demo/schema/configuration.schema")
        .lower("composition:demo");
    let options = RustEmissionOptions::feature_gated_nota("nota-text")
        .with_target(RustEmissionTarget::NexusRuntime);
    RustEmitter::new(options)
        .emit_code_from_schema(&schema)
        .as_str()
        .to_owned()
}

#[test]
fn write_configuration_fixture() {
    if std::env::var_os("SCHEMA_RUST_NEXT_UPDATE_FIXTURES").is_none() {
        return;
    }
    let code = emit_configuration();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/composition_demo_generated.rs");
    std::fs::write(path, code).expect("write configuration fixture");
}

#[test]
fn composed_inherent_impl_emits_from_data() {
    let code = emit_configuration();
    // The newtype emits as a tuple struct over its inner type.
    assert!(
        code.contains("pub struct ConfigurationPath(String)")
            || code.contains("pub struct ConfigurationPath (String)"),
        "newtype payload emits:\n{code}"
    );
    // The composed method emits as an INHERENT impl with the fixed accessor
    // signature, and the body is the data-projected composition.
    assert!(
        code.contains("impl ConfigurationPath {"),
        "inherent impl block emits:\n{code}"
    );
    assert!(
        code.contains("pub fn as_str(&self) -> &str"),
        "as_str signature emits:\n{code}"
    );
    let normalized: String = code.split_whitespace().collect();
    assert!(
        normalized.contains("self.payload().as_str()"),
        "as_str body is the data-emitted composition:\n{code}"
    );
}

/// Lower the composition-demo schema through the real engine and return the
/// single method carried as data by the `Composed ConfigurationPath` impl. This
/// is the actual NOTA → `Expression` path — the body travels as schema data.
fn composed_as_str_method() -> MethodDeclaration {
    let schema = FixtureSchema::new("composition-demo/schema/configuration.schema")
        .lower("composition:demo");
    let impl_declaration = schema
        .impls()
        .iter()
        .find(|declaration| declaration.target().as_str() == "ConfigurationPath")
        .expect("the demo schema declares an impl on ConfigurationPath");
    let ImplBody::Methods(methods) = impl_declaration.body() else {
        panic!("the composed impl carries methods, not a marker");
    };
    methods
        .iter()
        .find(|method| method.name().as_str() == "as_str")
        .cloned()
        .expect("the composed impl declares as_str")
}

#[test]
fn composed_body_parses_from_nota_as_a_method_call_tree() {
    // The body is the composition node nested over itself: the OUTER call is
    // `as_str` on the receiver `self.payload()`, whose receiver is in turn the
    // INNER call `payload` on `self`. This is depth-2 composition — strictly
    // more than the single-projection Deref body the prior slice could carry.
    let method = composed_as_str_method();
    let Expression::MethodCall(outer_receiver, outer_method, outer_arguments) = method.body() else {
        panic!("the as_str body is a method-call composition node");
    };
    assert_eq!(outer_method.as_str(), "as_str");
    assert!(
        outer_arguments.is_empty(),
        "as_str takes no arguments: {outer_arguments:?}"
    );
    let Expression::MethodCall(inner_receiver, inner_method, inner_arguments) =
        outer_receiver.as_ref()
    else {
        panic!("the as_str receiver is itself a method-call (self.payload())");
    };
    assert_eq!(inner_method.as_str(), "payload");
    assert!(inner_arguments.is_empty(), "payload takes no arguments");
    assert!(
        matches!(inner_receiver.as_ref(), Expression::SelfReceiver),
        "the innermost receiver is self"
    );
}

#[test]
fn composed_body_projects_to_exact_rust() {
    // The whole composition contribution: the data tree projects to the EXACT
    // Rust source of the hand-written body at signal-spirit/src/lib.rs:27.
    let method = composed_as_str_method();
    let rust = method
        .body()
        .to_rust()
        .expect("the composed body resolves to shape-implied primitives");
    assert_eq!(rust, "self.payload().as_str()");
}

/// The local stand-in for the generated module: the `ConfigurationPath` newtype
/// (a tuple struct over `String`, exactly as the emitter produces it) with its
/// inherent `payload` accessor, plus the inherent `as_str` whose body is the
/// DATA-EMITTED composition. If the projected body were wrong this would not
/// compile or would return the wrong value, so the behavioral assertion below
/// proves the code-is-data composition is correct.
#[allow(dead_code)]
mod configuration_generated {
    include!("fixtures/composition_demo_generated.rs");
}

#[test]
fn composed_body_compiles_and_runs() {
    use configuration_generated::ConfigurationPath;

    let path = ConfigurationPath::new("/etc/spirit/config.nota".to_owned());
    // `as_str` is the DATA-EMITTED body `self.payload().as_str()` — it composes
    // the newtype `payload` accessor with the inner `String::as_str` leaf. The
    // returned `&str` borrows through both calls.
    assert_eq!(path.as_str(), "/etc/spirit/config.nota");
    // The composed accessor returns the same bytes the inner payload holds.
    assert_eq!(path.as_str(), path.payload().as_str());
}

#[test]
fn business_logic_call_is_rejected_with_a_typed_error() {
    // `keywords` is a real business-logic method on the same component
    // (signal-spirit/src/lib.rs:647) — it is NOT a shape-implied primitive, so a
    // body that calls it must be REJECTED, not emitted. We build the body the
    // same way the reader would (`(call (self) keywords)`) and assert the typed
    // boundary error fires — never a panic, never a silently-wrong body.
    let business_logic_body = Expression::MethodCall(
        Box::new(Expression::SelfReceiver),
        Name::new("keywords"),
        Vec::new(),
    );
    let rejection = business_logic_body
        .to_rust()
        .expect_err("a call to a non-primitive must be rejected");
    match rejection {
        SchemaError::UnresolvedComposition { method, receiver } => {
            assert_eq!(method, "keywords");
            assert_eq!(receiver, "self");
        }
        other => panic!("expected UnresolvedComposition, got {other:?}"),
    }
}

#[test]
fn first_unresolved_call_in_a_composition_rejects_the_whole_body() {
    // The sharp edge fires at the FIRST unresolved call even when nested under a
    // resolvable one: `self.payload().keywords()` resolves `payload` but then
    // hits the business-logic `keywords` — the whole body is rejected.
    let mixed_body = Expression::MethodCall(
        Box::new(Expression::MethodCall(
            Box::new(Expression::SelfReceiver),
            Name::new("payload"),
            Vec::new(),
        )),
        Name::new("keywords"),
        Vec::new(),
    );
    let rejection = mixed_body
        .to_rust()
        .expect_err("the outer business-logic call rejects the whole body");
    assert!(
        matches!(
            rejection,
            SchemaError::UnresolvedComposition { ref method, .. } if method == "keywords"
        ),
        "the first unresolved call (keywords) is the boundary: {rejection:?}"
    );
}
