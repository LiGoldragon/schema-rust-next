//! End-to-end proof of SHAPE-DERIVED capability resolution (`d3r2`): a method
//! body that COMPOSES calls is DATA, and every call resolves against its
//! RECEIVER'S SCHEMA SHAPE — not a global method-name allowlist. The real
//! component method `ConfigurationPath::as_str` — currently hand-written at
//! `signal-spirit/src/lib.rs:26` as `self.payload().as_str()` — is declared as
//! the NOTA expression tree `(call (call self payload) as_str)`, parsed through
//! the real `SchemaEngine`, projected by `Expression::to_rust(resolver)`, and
//! the projected body is compiled and RUN to prove behavior.
//!
//! The composition node is `Expression::MethodCall(receiver, method, args)`
//! (`schema-next/.../schema.rs`). It is NOT an expression compiler: the call
//! head must resolve against the RECEIVER'S SHAPE-IMPLIED capability set
//! (`ReceiverShape::capabilities` via `CapabilityResolver`). `payload` resolves
//! on `ConfigurationPath` because it IS a newtype; the SAME name is REJECTED on
//! a struct because a struct shape implies only field projections. The first
//! call that resolves to no capability of its receiver's shape is the
//! business-logic boundary — rejected with a typed `SchemaError`, never a wrong
//! body, never a panic.

use schema_next::{
    CapabilityResolver, Declaration, Expression, ImplBody, MethodDeclaration, Name, ReceiverShape,
    Schema, SchemaError, TypeDeclaration, TypeReference,
};
use schema_rust_next::{RustEmissionOptions, RustEmissionTarget, RustEmitter};

mod support;

use support::FixtureSchema;

/// Lower the composition-demo schema through the real engine.
fn demo_schema() -> Schema {
    FixtureSchema::new("composition-demo/schema/configuration.schema").lower("composition:demo")
}

/// Emit the composition-demo schema to Rust source through the real emitter. The
/// `ConfigurationPath` newtype flows through the standard newtype emitter
/// (`new` / `payload` / `into_payload`), the `Magnitude` struct flows through
/// the standard struct emitter (`new` + per-field accessors), the `Reply` enum
/// flows through the variant-constructor emitter, and the `Composed` impl emits
/// an inherent `impl ConfigurationPath { pub fn as_str(&self) -> &str { … } }`
/// whose body is the shape-resolved composition.
fn emit_configuration() -> String {
    let options = RustEmissionOptions::feature_gated_nota("nota-text")
        .with_target(RustEmissionTarget::NexusRuntime);
    RustEmitter::new(options)
        .emit_code_from_schema(&demo_schema())
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
fn standard_impls_emit_from_data_for_every_shape() {
    let code = emit_configuration();
    // NEWTYPE shape: emitted as a tuple struct over its inner type with the
    // standard new/payload/into_payload + From<Inner> set — none declared.
    assert!(
        code.contains("pub struct ConfigurationPath(String)")
            || code.contains("pub struct ConfigurationPath (String)"),
        "newtype payload emits:\n{code}"
    );
    assert!(
        code.contains("impl ConfigurationPath {"),
        "newtype inherent impl block emits:\n{code}"
    );
    let normalized: String = code.split_whitespace().collect();
    assert!(
        normalized.contains("pubfnnew") && normalized.contains("pubfnpayload"),
        "standard newtype accessors emit:\n{code}"
    );
    assert!(
        normalized.contains("implFrom<String>forConfigurationPath"),
        "standard newtype From<Inner> emits:\n{code}"
    );

    // STRUCT shape: the two-field `Magnitude` emits the standard struct impl —
    // an all-fields `new` plus a per-field borrow accessor — none declared.
    assert!(
        code.contains("impl Magnitude {"),
        "struct inherent impl block emits:\n{code}"
    );
    assert!(
        normalized.contains("pubfnvalue(&self)->&Integer"),
        "per-field struct accessor emits:\n{code}"
    );
    assert!(
        normalized.contains("pubfnscale(&self)->&Integer"),
        "second per-field struct accessor emits:\n{code}"
    );
    assert!(
        normalized.contains("pubfnnew(value:Integer,scale:Integer)->Self"),
        "all-fields struct constructor emits:\n{code}"
    );

    // ENUM shape: the `Reply` enum emits its per-variant constructor for the
    // payload-carrying variant — proving enum-shape resolution drives emission.
    // The existing constructor emitter unwraps a newtype payload to its inner,
    // so `Rejected ConfigurationPath` constructs from the inner `String`.
    assert!(
        normalized.contains("pubfnrejected(payload:String)->Self"),
        "enum variant constructor emits:\n{code}"
    );
    assert!(
        normalized.contains("Self::Rejected(ConfigurationPath::new(payload))"),
        "the variant constructor wraps the inner payload:\n{code}"
    );
}

/// The single method carried as data by the `Composed ConfigurationPath` impl.
fn composed_as_str_method() -> MethodDeclaration {
    let schema = demo_schema();
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
    // INNER call `payload` on `self`. This is depth-2 composition.
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
fn composed_body_projects_through_shape_derived_capabilities() {
    // The whole composition contribution: the data tree projects to the EXACT
    // Rust source of the hand-written body at signal-spirit/src/lib.rs:27 — but
    // now EVERY call is resolved by walking the type graph. `self.payload()`
    // typeofs `ConfigurationPath` -> Newtype{String} -> `payload` resolves with
    // result `String`; `String` is a Builtin leaf -> `as_str` resolves via the
    // tiny named per-leaf exception.
    let schema = demo_schema();
    let self_type = Name::new("ConfigurationPath");
    let resolver = CapabilityResolver::new(schema.namespace(), &self_type);
    let method = composed_as_str_method();
    let rust = method
        .body()
        .to_rust(&resolver)
        .expect("the composed body resolves against the receiver's shape");
    assert_eq!(rust, "self.payload().as_str()");
}

#[test]
fn type_propagates_through_a_depth_two_composition() {
    // typeof(self.payload()) is the newtype inner type String — proving the
    // OUTER call's receiver type is COMPUTED, not assumed. This is what lets
    // `as_str` resolve as a String-leaf capability rather than a name guess.
    let schema = demo_schema();
    let self_type = Name::new("ConfigurationPath");
    let resolver = CapabilityResolver::new(schema.namespace(), &self_type);
    let inner_payload_call = Expression::MethodCall(
        Box::new(Expression::SelfReceiver),
        Name::new("payload"),
        Vec::new(),
    );
    assert_eq!(
        resolver
            .type_of(&inner_payload_call)
            .expect("payload typeofs"),
        TypeReference::String,
    );
}

/// The local stand-in for the generated module: the emitter-produced
/// `ConfigurationPath` newtype (a tuple struct over `String`) with its standard
/// inherent accessors, plus the inherent `as_str` whose body is the
/// SHAPE-RESOLVED composition. If the projected body were wrong this would not
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
    // `as_str` is the SHAPE-RESOLVED body `self.payload().as_str()`. The
    // returned `&str` borrows through both calls.
    assert_eq!(path.as_str(), "/etc/spirit/config.nota");
    assert_eq!(path.as_str(), path.payload().as_str());
}

/// The `Magnitude` struct name as declared in the demo schema.
fn magnitude_type() -> Name {
    Name::new("Magnitude")
}

#[test]
fn payload_on_a_struct_is_rejected_as_shape_mismatch() {
    // `payload` was in the OLD name allowlist, so the old resolver wrongly
    // accepted it on ANY receiver. The shape resolver REJECTS it on a struct
    // because `Struct::capabilities()` holds only field projections — the case
    // the name-allowlist got wrong.
    let schema = demo_schema();
    let self_type = magnitude_type();
    let resolver = CapabilityResolver::new(schema.namespace(), &self_type);
    let body = Expression::MethodCall(
        Box::new(Expression::SelfReceiver),
        Name::new("payload"),
        Vec::new(),
    );
    match body
        .to_rust(&resolver)
        .expect_err("payload is not a struct capability")
    {
        SchemaError::UnresolvedCapability {
            method,
            receiver_shape,
            ..
        } => {
            assert_eq!(method, "payload");
            assert_eq!(receiver_shape, "struct");
        }
        other => panic!("expected UnresolvedCapability, got {other:?}"),
    }
}

#[test]
fn unknown_struct_field_is_rejected() {
    // `(field self nonexistent)` on a struct that declares no such field.
    let schema = demo_schema();
    let self_type = magnitude_type();
    let resolver = CapabilityResolver::new(schema.namespace(), &self_type);
    let body = Expression::Field(Box::new(Expression::SelfReceiver), Name::new("nonexistent"));
    assert!(
        matches!(
            resolver.type_of(&body).unwrap_err(),
            SchemaError::UnknownFieldProjection { ref field, .. } if field == "nonexistent"
        ),
        "an undeclared field projection is a typed rejection"
    );
}

#[test]
fn field_resolves_on_struct_and_constructor_resolves_on_enum() {
    let schema = demo_schema();
    let self_type = magnitude_type();
    let resolver = CapabilityResolver::new(schema.namespace(), &self_type);
    // A real declared field typeofs to its declared reference.
    assert_eq!(
        resolver
            .type_of(&Expression::Field(
                Box::new(Expression::SelfReceiver),
                Name::new("value"),
            ))
            .expect("value field typeofs"),
        TypeReference::Integer,
    );
    // The enum variant constructor resolves on the enum shape by its emitted
    // call spelling (the snake_case constructor name `rejected`).
    let enum_shape = enum_shape_of(&schema, "Reply");
    assert!(
        enum_shape.resolve(&Name::new("rejected")).is_some(),
        "the Rejected variant constructor resolves on the Reply enum shape"
    );
    // A non-existent variant does not resolve.
    assert!(
        enum_shape.resolve(&Name::new("missing")).is_none(),
        "an undeclared variant does not resolve on the enum shape"
    );
}

/// Build the [`ReceiverShape`] of a named declared enum, straight from the
/// schema type graph — the same shape the resolver reads.
fn enum_shape_of(schema: &Schema, name: &str) -> ReceiverShape {
    let declaration: &Declaration = schema
        .namespace()
        .iter()
        .find(|declaration| declaration.name().as_str() == name)
        .expect("the demo schema declares the enum");
    let TypeDeclaration::Enum(enumeration) = declaration.value() else {
        panic!("{name} is an enum");
    };
    ReceiverShape::Enum {
        variants: enumeration.variants.clone(),
    }
}
