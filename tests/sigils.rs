//! End-to-end witness for the converged sigil grammar (records 1072 /
//! 1078 / 1079 / 1080 / 1085 / 1087) through the Rust emitter.
//!
//! The fixture `tests/fixtures/sigils.schema` is written in the sigil
//! grammar: `@`-prefixed macro invocations (`(@Vec Service)`,
//! `(@Option NodeName)`, `(@KeyValue NodeName Service)`) and `*`-suffix
//! same-name variants (`Register*`, `Observe*`, `Registered*`,
//! `Listed*`) at root-enum positions, plus a plain unit variant
//! (`Decommission`).
//!
//! The load-bearing point: both sigils are SURFACE grammar that
//! schema-next lowers into the existing assembled shapes
//! (`TypeReference::Vector` etc. and `EnumVariant { payload: Some(..) }`).
//! The emitter is unchanged — it emits `Vec<Service>`, `BTreeMap`,
//! `Option`, and same-name data variants (`Register(Register)`) exactly
//! as it would for the desugared source. This test proves the surface
//! sigils lower, emit, and round-trip through NOTA + rkyv.

use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

#[allow(dead_code)]
mod generated {
    include!("fixtures/sigils_generated.rs");
}

/// The sigil-grammar fixture lowers and emits byte-for-byte the
/// checked-in generated Rust. Re-emitting the schema must match the
/// fixture, so the lowering of `@`-macros and `*`-variants is stable.
#[test]
fn sigil_schema_emits_byte_identical_to_generated_fixture() {
    let source = include_str!("fixtures/sigils.schema");
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("sigils:lib", "0.1.0"))
        .expect("sigil schema lowers");
    let generated = RustEmitter::default().emit(&asschema);

    assert_eq!(
        generated.as_str(),
        include_str!("fixtures/sigils_generated.rs"),
        "the sigil grammar lowers to the same assembled shapes the emitter already handles",
    );
}

/// The `*`-suffix variants emitted same-name data variants: `Register*`
/// became `Register(Register)`, carrying the namespace `Register`
/// struct (which itself holds two `@`-macro collection fields). The
/// plain `Decommission` stayed a unit variant. Round-trips through
/// NOTA.
#[test]
fn same_name_variant_carrying_collection_payload_round_trips_through_nota() {
    let register = generated::Register {
        services: vec![
            generated::Service("dns".to_owned()),
            generated::Service("mail".to_owned()),
        ],
        replicas: Some(generated::NodeName("alpha".to_owned())),
    };
    let input = generated::Input::Register(register);

    let encoded = input.to_nota();
    let parsed = encoded
        .parse::<generated::Input>()
        .expect("register input parses");
    assert_eq!(parsed, input);

    // The unit variant round-trips too.
    let decommission = "Decommission"
        .parse::<generated::Input>()
        .expect("decommission parses");
    assert_eq!(decommission, generated::Input::Decommission);
    assert_eq!(decommission.to_nota(), "Decommission");
}

/// The `@KeyValue` macro became a `BTreeMap` same-name variant payload:
/// `Registered*` → `Registered(Registered)` where `Registered` wraps a
/// `BTreeMap<NodeName, Service>`. Round-trips through NOTA and rkyv (the
/// map key earns its ordering derives so the archive form compiles).
#[test]
fn same_name_map_variant_round_trips_through_nota_and_rkyv() {
    let mut placements = std::collections::BTreeMap::new();
    placements.insert(
        generated::NodeName("alpha".to_owned()),
        generated::Service("dns".to_owned()),
    );
    placements.insert(
        generated::NodeName("beta".to_owned()),
        generated::Service("mail".to_owned()),
    );
    let output = generated::Output::Registered(generated::Registered(placements));

    // NOTA round-trip through the root-enum codec.
    let encoded = output.to_nota();
    let parsed = encoded
        .parse::<generated::Output>()
        .expect("registered output parses");
    assert_eq!(parsed, output);

    // rkyv round-trip.
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&output).expect("archive output");
    let decoded =
        rkyv::from_bytes::<generated::Output, rkyv::rancor::Error>(&bytes).expect("decode output");
    assert_eq!(decoded, output);
}

/// The `@Vec` same-name variant (`Listed*` → `Listed(Listed)` wrapping
/// a `Vec<NodeName>`) crosses the rkyv signal frame and triages back to
/// its route — the full wire path for a sigil-grammar root variant.
#[test]
fn same_name_vector_variant_crosses_signal_frame_and_triages_route() {
    let listed = generated::Output::Listed(generated::Listed(vec![
        generated::NodeName("alpha".to_owned()),
        generated::NodeName("beta".to_owned()),
    ]));

    let frame = listed.encode_signal_frame().expect("encode signal frame");
    let (route, decoded) =
        generated::Output::decode_signal_frame(&frame).expect("decode signal frame");

    assert_eq!(route, generated::OutputRoute::Listed);
    assert_eq!(decoded, listed);
}

/// The `@Option` macro field round-trips both its `Some` and `None`
/// forms — proving the macro lowered to a genuine `Option<NodeName>`
/// the runtime codec handles.
#[test]
fn option_macro_field_round_trips_some_and_none() {
    let with_replica = generated::Input::Register(generated::Register {
        services: vec![generated::Service("dns".to_owned())],
        replicas: Some(generated::NodeName("alpha".to_owned())),
    });
    let without_replica = generated::Input::Register(generated::Register {
        services: Vec::new(),
        replicas: None,
    });

    for value in [with_replica, without_replica] {
        let parsed = value
            .to_nota()
            .parse::<generated::Input>()
            .expect("register input parses");
        assert_eq!(parsed, value);
    }
}
