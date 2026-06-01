use schema_rust_next::{NotaSurface, RustEmissionOptions, RustEmitter, RustTypeDeclaration};
use std::path::PathBuf;

mod support;

use support::{FixtureNota, FixtureSchema};

fn generated_fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(file_name)
}

fn assert_generated_fixture(file_name: &str, generated: &str) {
    let path = generated_fixture_path(file_name);
    if std::env::var_os("SCHEMA_RUST_NEXT_UPDATE_FIXTURES").is_some() {
        std::fs::write(&path, generated).expect("write generated fixture");
    }
    let expected = std::fs::read_to_string(path).expect("read generated fixture");
    assert_eq!(generated, expected);
}

#[allow(dead_code)]
mod generated {
    include!("fixtures/spirit_generated.rs");
}

#[allow(dead_code)]
mod collections_generated {
    include!("fixtures/collections_generated.rs");
}

#[test]
fn emits_rust_source_as_a_separate_artifact() {
    let asschema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated = RustEmitter::default().emit_file(&asschema);

    assert_eq!(generated.path, "src/schema/lib.rs");
    assert!(generated.code.as_str().contains("pub enum Input"));
    assert!(
        generated
            .code
            .as_str()
            .contains("impl std::str::FromStr for Input")
    );
    assert!(generated.code.as_str().contains("rkyv::Archive"));
    assert!(generated.code.as_str().contains("pub mod short_header"));
    assert!(generated.code.as_str().contains("pub enum InputRoute"));
    assert!(
        generated
            .code
            .as_str()
            .contains("pub fn encode_signal_frame")
    );
    assert!(!generated.code.as_str().contains("pub trait InputNexus"));
    assert!(!generated.code.as_str().contains("pub trait OutputNexus"));
    assert!(!generated.code.as_str().contains("dispatch_mail_with_nexus"));
    assert!(generated.code.as_str().contains("pub struct MessageSent"));
    assert!(generated.code.as_str().contains("pub struct OriginRoute"));
    assert!(generated.code.as_str().contains("pub mod signal"));
    assert!(generated.code.as_str().contains("pub enum Plane"));
    assert!(
        generated
            .code
            .as_str()
            .contains("Signal(super::Signal<SignalRoot>)")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("pub type Input = super::Input;")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("pub origin_route: OriginRoute")
    );
    assert!(generated.code.as_str().contains("impl OriginRoute"));
    assert!(
        generated
            .code
            .as_str()
            .contains("pub trait UpgradeFrom<Previous>")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("impl From<Entry> for Input")
    );
    assert_generated_fixture("spirit_generated.rs", generated.code.as_str());
}

#[test]
fn emitter_builds_rust_module_data_before_rendering_text() {
    let asschema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let emitter = RustEmitter::default();
    let module = emitter.emit_module(&asschema);

    assert_eq!(module.file_path(), "src/schema/lib.rs");
    assert_eq!(module.root_enums().len(), 2);
    assert_eq!(module.root_enums()[0].name().as_str(), "Input");
    assert_eq!(module.root_enums()[1].name().as_str(), "Output");
    assert!(
        module
            .scalar_aliases()
            .iter()
            .any(|alias| alias.name() == "Integer" && alias.rust_type() == "u64"),
        "scalar floor should be data on RustModule"
    );

    let topic = module
        .declaration_named("Topic")
        .expect("Topic declaration exists");
    assert!(matches!(topic.value(), RustTypeDeclaration::Newtype(_)));

    let entry = module
        .declaration_named("Entry")
        .expect("Entry declaration exists");
    let RustTypeDeclaration::Struct(entry_struct) = entry.value() else {
        panic!("Entry should model as a Rust struct declaration");
    };
    assert_eq!(entry_struct.fields()[0].name().as_str(), "topics");
    assert_eq!(entry_struct.fields()[1].name().as_str(), "kind");

    assert_eq!(module.render(), emitter.emit(&asschema));
}

#[test]
fn emission_can_disable_nota_surface_for_binary_only_consumers() {
    // Binary-only shape — daemons and other binary-only consumers
    // ship zero NOTA derives and zero `nota_next::*` references.
    // The emitted source carries only the `rkyv` + signal-frame
    // surface, so the generated module compiles when the consumer
    // does not depend on `nota-next` at all.
    let asschema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated = RustEmitter::new(RustEmissionOptions {
        nota_surface: NotaSurface::Disabled,
    })
    .emit(&asschema);
    let code = generated.as_str();

    assert!(code.contains("rkyv::Archive"));
    assert!(code.contains("pub fn encode_signal_frame"));
    assert!(code.contains("pub fn decode_signal_frame"));
    assert!(!code.contains("nota_next"));
    assert!(!code.contains("NotaDecode"));
    assert!(!code.contains("NotaEncode"));
    assert!(!code.contains("from_nota_block"));
    assert!(!code.contains("to_nota"));
    assert!(!code.contains("FromStr"));
    assert!(!code.contains("impl std::fmt::Display for Input"));
    assert!(!code.contains("impl std::fmt::Display for Output"));
    // No leftover `#[cfg(feature = ...)]` directives either —
    // `Disabled` removes the whole NOTA surface, it doesn't gate it.
    assert!(!code.contains("#[cfg(feature ="));
    assert!(!code.contains("#[cfg_attr(feature ="));
    // Lock in the exact emitted source so any future drift forces a
    // conscious fixture update.
    assert_generated_fixture("spirit_generated_binary_only.rs", code);
}

#[test]
fn emission_can_gate_nota_surface_behind_text_client_feature() {
    // Feature-gated shape (the default) — every NOTA surface lands
    // behind `#[cfg(feature = "nota-text")]` on impls / `use` items
    // and `#[cfg_attr(...)]` on derives, so a single emitted module
    // serves both text-facing CLIs (with the feature on) and
    // binary-only daemons (with the feature off).
    let asschema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated = RustEmitter::new(RustEmissionOptions {
        nota_surface: NotaSurface::FeatureGated {
            feature: "nota-text".to_owned(),
        },
    })
    .emit(&asschema);
    let code = generated.as_str();

    assert!(code.contains(
        "#[cfg_attr(feature = \"nota-text\", derive(nota_next::NotaDecode, nota_next::NotaEncode))]"
    ));
    assert!(code.contains("#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize"));
    assert!(code.contains("#[cfg(feature = \"nota-text\")]\npub use nota_next::{"));
    assert!(code.contains("#[cfg(feature = \"nota-text\")]\nimpl std::str::FromStr for Input"));
    assert!(code.contains("#[cfg(feature = \"nota-text\")]\nimpl std::fmt::Display for Output"));
    assert!(code.contains("pub fn encode_signal_frame"));
    assert!(code.contains("pub fn decode_signal_frame"));
    // The default shape is identical to this explicit
    // construction — the checked-in `spirit_generated.rs` snapshot
    // is the binding form. Constructed-positional and default-
    // constructed emissions must stay byte-identical.
    let default_generated = RustEmitter::default().emit(&asschema);
    assert_eq!(code, default_generated.as_str());
}

#[test]
fn rust_emission_options_default_is_feature_gated_nota_text() {
    // `RustEmissionOptions::default()` and `RustEmitter::default()`
    // both pick the compatibility-oriented opt-in shape per the
    // codec opt-in design: rkyv is universal, NOTA is gated by the
    // `nota-text` feature.
    let options = RustEmissionOptions::default();
    assert_eq!(
        options.nota_surface,
        NotaSurface::FeatureGated {
            feature: "nota-text".to_owned(),
        },
    );
}

#[test]
fn emitted_path_mirrors_schema_module_identity() {
    let asschema = FixtureSchema::new("spirit-min.schema").lower("spirit-next:signal:public");
    let generated = RustEmitter::default().emit_file(&asschema);

    assert_eq!(generated.path, "src/schema/signal/public.rs");
}

#[test]
fn inline_private_schema_types_emit_crate_local_rust_boundary() {
    let asschema = FixtureSchema::new("inline-private-type.schema").lower("example:inline");
    let generated = RustEmitter::default().emit_file(&asschema);
    let code = generated.code.as_str();

    assert!(code.contains("pub(crate) struct Receipt"));
    assert!(code.contains("pub struct Entry"));
    assert!(code.contains("pub(crate) receipt: Receipt"));
    assert!(code.contains("pub(crate) later: Receipt"));
}

#[test]
fn emits_schema_plane_engine_traits_for_declared_signal_nexus_and_sema_languages() {
    let asschema = FixtureSchema::new("plane-triad.schema").lower("spirit:lib");
    let generated = RustEmitter::default().emit_file(&asschema);

    assert!(generated.code.as_str().contains("pub trait SignalEngine"));
    assert!(generated.code.as_str().contains(
        "fn trace_signal_triaged(&self, _input: &signal::Signal<signal::Input>, _output: &nexus::Nexus<nexus::Input>) {}"
    ));
    assert!(generated.code.as_str().contains(
        "fn triage_inner(&self, input: signal::Signal<signal::Input>) -> nexus::Nexus<nexus::Input>;"
    ));
    assert!(generated.code.as_str().contains(
        "fn triage(&self, input: signal::Signal<signal::Input>) -> nexus::Nexus<nexus::Input> {"
    ));
    assert!(
        generated
            .code
            .as_str()
            .contains("self.trace_signal_replied(&signal_output);")
    );
    assert!(generated.code.as_str().contains("pub trait NexusEngine"));
    assert!(generated.code.as_str().contains("pub mod nexus"));
    assert!(generated.code.as_str().contains("pub mod sema"));
    assert!(generated.code.as_str().contains(
        "fn decide(&mut self, input: nexus::Nexus<nexus::Input>) -> nexus::Nexus<nexus::Output>;"
    ));
    assert!(
        generated
            .code
            .as_str()
            .contains("self.trace_nexus_entered(&input);")
    );
    assert!(generated.code.as_str().contains("pub trait SemaEngine"));
    assert!(generated.code.as_str().contains(
        "fn apply_inner(&mut self, input: sema::Sema<sema::WriteInput>) -> sema::Sema<sema::WriteOutput>;"
    ));
    assert!(generated.code.as_str().contains(
        "fn observe_inner(&self, input: sema::Sema<sema::ReadInput>) -> sema::Sema<sema::ReadOutput>;"
    ));
    assert!(
        generated
            .code
            .as_str()
            .contains("self.trace_sema_write_applied(&trace_input, &output);")
    );
    assert!(!generated.code.as_str().contains("NexusMail<Payload>"));
    assert!(
        generated
            .code
            .as_str()
            .contains("pub fn into_nexus_output(self) -> nexus::Nexus<nexus::Output>")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("Input::Record(payload) => NexusOutput::from(SemaWriteInput::from(payload))")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("Input::Observe(payload) => NexusOutput::from(SemaReadInput::from(payload))")
    );
    assert!(generated.code.as_str().contains(
        "SemaWriteOutput::Recorded(payload) => NexusOutput::from(Output::from(payload))"
    ));
    assert!(
        generated.code.as_str().contains(
            "SemaReadOutput::Observed(payload) => NexusOutput::from(Output::from(payload))"
        )
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("pub fn into_sema_write_input(self) -> sema::Sema<sema::WriteInput>")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("pub fn into_sema_read_input(self) -> sema::Sema<sema::ReadInput>")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("pub fn into_signal_output(self) -> signal::Signal<signal::Output>")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("impl sema::Sema<sema::WriteOutput>")
    );
    assert!(
        generated
            .code
            .as_str()
            .contains("impl sema::Sema<sema::ReadOutput>")
    );
}

#[test]
fn compiled_fixture_is_usable_rust() {
    let entry = generated::Entry {
        topics: generated::Topics(vec![generated::Topic(String::from("schema"))]),
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
    assert_eq!(input.route(), generated::InputRoute::Record);
    assert_eq!(input.short_header(), generated::short_header::INPUT_RECORD);
}

#[test]
fn generated_roots_wrap_into_messages_with_automatic_origin_route() {
    let input = FixtureNota::new("nota/observe-schema-principle.nota")
        .read()
        .parse::<generated::Input>()
        .expect("parse observe input");
    let message: generated::signal::Signal<generated::signal::Input> =
        input.with_origin_route(generated::OriginRoute(19));

    assert_eq!(message.origin_route(), generated::OriginRoute(19));
    let plane = generated::schema::Plane::<generated::signal::Input, (), ()>::Signal(message);
    assert_eq!(plane.origin_route(), generated::OriginRoute(19));
    let generated::schema::Plane::Signal(message) = plane else {
        panic!("expected signal plane");
    };
    assert!(matches!(
        message.root(),
        generated::Input::Observe(generated::Query { .. })
    ));
}

#[test]
fn generated_input_parses_cli_nota_and_emits_nota() {
    let source = FixtureNota::new("nota/record-clarified-intent.nota").read();
    let input = source
        .parse::<generated::Input>()
        .expect("parse generated input");

    match &input {
        generated::Input::Record(entry) => {
            assert_eq!(entry.topics.0[0].0, "schema");
            assert_eq!(entry.kind, generated::Kind::Constraint);
            assert_eq!(entry.description.0, "agent's clarified intent");
            assert_eq!(entry.magnitude, generated::Magnitude::Maximum);
        }
        generated::Input::Observe(_) => panic!("expected record"),
    }

    assert_eq!(input.to_string(), source);
}

#[test]
fn generated_signal_input_round_trips_from_nota_to_rkyv_bytes() {
    let input = FixtureNota::new("nota/record-component-rkyv.nota")
        .read()
        .parse::<generated::Input>()
        .expect("parse generated input");

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input).expect("archive input");
    let decoded =
        rkyv::from_bytes::<generated::Input, rkyv::rancor::Error>(&bytes).expect("decode input");

    assert_eq!(decoded, input);
}

#[test]
fn generated_signal_frame_methods_round_trip_and_triage_route() {
    let input = FixtureNota::new("nota/record-schema-owns-frames.nota")
        .read()
        .parse::<generated::Input>()
        .expect("parse generated input");

    let frame = input.encode_signal_frame().expect("encode signal frame");
    let (route, decoded) =
        generated::Input::decode_signal_frame(&frame).expect("decode signal frame");

    assert_eq!(route, generated::InputRoute::Record);
    assert_eq!(decoded, input);
    assert_eq!(
        generated::Input::route_from_short_header(generated::short_header::INPUT_OBSERVE)
            .expect("observe route"),
        generated::InputRoute::Observe
    );
}

struct MailHook {
    sent_events: Vec<generated::MessageSent>,
    processed_events: Vec<generated::MessageProcessed<generated::Output>>,
}

impl MailHook {
    fn new() -> Self {
        Self {
            sent_events: Vec::new(),
            processed_events: Vec::new(),
        }
    }
}

impl generated::MessageSentHook for MailHook {
    type Error = RuntimeError;

    fn message_sent(&mut self, event: generated::MessageSent) -> Result<(), Self::Error> {
        self.sent_events.push(event);
        Ok(())
    }
}

#[test]
fn generated_signal_roots_emit_typed_message_sent_events() {
    let input = FixtureNota::new("nota/observe-schema-principle.nota")
        .read()
        .parse::<generated::Input>()
        .expect("parse observe input");
    let event = input
        .with_origin_route(generated::OriginRoute(900))
        .message_sent(generated::MessageIdentifier(42));
    let mut hook = MailHook::new();

    event.push_to(&mut hook).expect("message sent event pushes");

    assert_eq!(
        hook.sent_events,
        vec![generated::MessageSent {
            identifier: generated::MessageIdentifier(42),
            origin_route: generated::OriginRoute(900),
            root: generated::MessageRoot::Input,
            short_header: generated::short_header::INPUT_OBSERVE,
        }],
    );
    assert_eq!(event.origin_route(), generated::OriginRoute(900));
    assert_eq!(
        generated::NotaSource::new("900")
            .parse::<generated::OriginRoute>()
            .expect("origin route decodes through shared codec"),
        generated::OriginRoute(900)
    );
    assert_eq!(generated::OriginRoute(900).to_nota(), "900");
    assert_eq!(
        generated::NotaSource::new("42")
            .parse::<generated::MessageIdentifier>()
            .expect("message identifier decodes through shared codec"),
        generated::MessageIdentifier(42)
    );
    assert_eq!(generated::MessageIdentifier(42).to_nota(), "42");
    assert_ne!(
        event.origin_route(),
        generated::OriginRoute(event.identifier.0),
        "origin route is minted separately from the message identifier"
    );
}

#[derive(Debug, PartialEq, Eq)]
enum RuntimeError {
    StateRejected,
}

impl generated::MessageProcessedHook<generated::Output> for MailHook {
    type Error = RuntimeError;

    fn message_processed(
        &mut self,
        event: generated::MessageProcessed<generated::Output>,
    ) -> Result<(), Self::Error> {
        self.processed_events.push(event);
        Ok(())
    }
}

#[test]
fn generated_processed_mail_events_are_typed_without_root_dispatch_traits() {
    assert_eq!(RuntimeError::StateRejected, RuntimeError::StateRejected);
    let mut hook = MailHook::new();
    let reply = generated::Output::RecordsObserved(generated::RecordSet(vec![]));

    let processed = generated::MessageProcessed::new(
        generated::MessageIdentifier(77),
        generated::OriginRoute(701),
        reply,
    );
    processed
        .push_to(&mut hook)
        .expect("processed mail event pushes");

    assert_eq!(
        hook.processed_events,
        vec![generated::MessageProcessed {
            identifier: generated::MessageIdentifier(77),
            origin_route: generated::OriginRoute(701),
            reply: generated::Output::RecordsObserved(generated::RecordSet(vec![])),
        }],
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreviousEntry {
    topic: String,
    description: String,
}

#[derive(Debug, PartialEq, Eq)]
struct UpgradeEvent {
    description: String,
}

impl generated::UpgradeFrom<PreviousEntry> for generated::Entry {
    type Error = RuntimeError;

    fn upgrade_from(previous: PreviousEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            topics: generated::Topics(vec![generated::Topic(previous.topic)]),
            kind: generated::Kind::Clarification,
            description: generated::Description(previous.description),
            magnitude: generated::Magnitude::High,
        })
    }
}

impl UpgradeEvent {
    fn from_previous_entry(previous: PreviousEntry) -> Result<Self, RuntimeError> {
        let entry =
            <generated::Entry as generated::AcceptPrevious<PreviousEntry>>::accept_previous(
                previous,
            )?;
        Ok(Self {
            description: format!("accepted previous Entry as {}", entry.to_nota()),
        })
    }
}

#[test]
fn generated_upgrade_trait_accepts_previous_schema_objects_observably() {
    let event = UpgradeEvent::from_previous_entry(PreviousEntry {
        topic: "schema".to_owned(),
        description: "old client spoke previous entry".to_owned(),
    })
    .expect("previous entry upgrades");

    assert_eq!(
        event,
        UpgradeEvent {
            description: "accepted previous Entry as ([[schema]] Clarification [old client spoke previous entry] High)".to_owned(),
        },
    );
}

#[test]
fn emits_vec_map_and_option_collection_types_with_shared_codec_traits() {
    let asschema = FixtureSchema::new("collections.schema").lower("collections:lib");
    let generated = RustEmitter::default().emit(&asschema);
    let code = generated.as_str();

    // The generated code imports the shared NOTA codec instead of
    // emitting a local collection-support runtime block. Under the
    // default `feature_gated_nota("nota-text")` shape, the `use
    // nota_next::*` and `cfg_attr(...)` derives sit behind the
    // `nota-text` feature.
    assert!(code.contains("#[cfg(feature = \"nota-text\")]\npub use nota_next::{"));
    assert!(!code.contains("pub struct NotaCollection"));
    assert!(code.contains(
        "#[cfg_attr(feature = \"nota-text\", derive(nota_next::NotaDecode, nota_next::NotaEncode))]"
    ));
    assert!(!code.contains("impl NotaDecode for Cluster"));
    assert!(!code.contains("impl NotaEncode for Cluster"));
    // Vec / KeyValue->BTreeMap / Option render at the field positions.
    assert!(code.contains("pub services: Vec<Service>,"));
    assert!(code.contains("pub nodes: std::collections::BTreeMap<NodeName, NodeConfig>,"));
    assert!(code.contains("pub cache: Option<NodeConfig>,"));
    assert!(code.contains("pub healthy: Boolean,"));
    assert!(code.contains("pub config_path: Path,"));
    assert!(code.contains("pub type Path = std::string::String;"));
    // Collection payloads in a root output variant.
    assert!(code.contains("Projected(std::collections::BTreeMap<NodeName, NodeConfig>),"));
    assert!(code.contains("Listed(Vec<NodeName>),"));
    // A map key type earns the ordering derives so BTreeMap compiles;
    // a value-only type keeps the original derive set. Both forms
    // gain a feature-gated NOTA derive above the unconditional rkyv
    // derive under the default emission shape.
    assert!(code.contains(
        "#[rkyv(derive(PartialEq, Eq, PartialOrd, Ord))]\npub struct NodeName(pub String);"
    ));
    assert!(code.contains(concat!(
        "#[cfg_attr(feature = \"nota-text\", derive(nota_next::NotaDecode, nota_next::NotaEncode))]\n",
        "#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]\n",
        "pub struct NodeConfig(pub String);",
    )));
    assert_generated_fixture("collections_generated.rs", code);
}

#[test]
fn collection_free_schema_keeps_checked_generated_source_stable() {
    // The regression safety net: a schema that uses no collection still
    // emits exactly the checked-in fixture, proving the collection work
    // is purely additive.
    let asschema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated = RustEmitter::default().emit(&asschema);

    assert_generated_fixture("spirit_generated.rs", generated.as_str());
}

#[test]
fn generated_collection_struct_round_trips_through_nota() {
    // Author a Cluster carrying all three collection kinds, encode it
    // to NOTA, parse it back, and confirm the value survives.
    let cluster = collections_generated::Cluster {
        services: vec![
            collections_generated::Service("dns".to_owned()),
            collections_generated::Service("mail".to_owned()),
        ],
        nodes: {
            let mut nodes = std::collections::BTreeMap::new();
            nodes.insert(
                collections_generated::NodeName("alpha".to_owned()),
                collections_generated::NodeConfig("primary".to_owned()),
            );
            nodes.insert(
                collections_generated::NodeName("beta".to_owned()),
                collections_generated::NodeConfig("replica".to_owned()),
            );
            nodes
        },
        cache: Some(collections_generated::NodeConfig("warm".to_owned())),
        healthy: true,
        config_path: "/tmp/cluster.nota".to_owned(),
    };

    let encoded = cluster.to_nota();
    let parsed = collections_generated::Cluster::from_nota_block(
        &collections_generated::NotaSource::new(&encoded)
            .parse_root()
            .expect("cluster nota parses"),
    )
    .expect("cluster decodes");

    assert_eq!(parsed, cluster);
    // The empty / None forms also round-trip.
    let empty = collections_generated::Cluster {
        services: Vec::new(),
        nodes: std::collections::BTreeMap::new(),
        cache: None,
        healthy: false,
        config_path: "/tmp/empty.nota".to_owned(),
    };
    let empty_encoded = empty.to_nota();
    let empty_parsed = collections_generated::Cluster::from_nota_block(
        &collections_generated::NotaSource::new(&empty_encoded)
            .parse_root()
            .expect("empty cluster nota parses"),
    )
    .expect("empty cluster decodes");
    assert_eq!(empty_parsed, empty);
}

#[test]
fn generated_collection_payload_root_variant_round_trips_to_nota_and_rkyv() {
    let mut projection = std::collections::BTreeMap::new();
    projection.insert(
        collections_generated::NodeName("alpha".to_owned()),
        collections_generated::NodeConfig("primary".to_owned()),
    );
    let output = collections_generated::Output::Projected(projection);

    // NOTA round-trip through the root enum codec.
    let encoded = output.to_nota();
    let parsed = encoded
        .parse::<collections_generated::Output>()
        .expect("projected output parses");
    assert_eq!(parsed, output);

    // rkyv round-trip — the map key's ordering derives let the archive
    // form compile and compare.
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&output).expect("archive output");
    let decoded = rkyv::from_bytes::<collections_generated::Output, rkyv::rancor::Error>(&bytes)
        .expect("decode output");
    assert_eq!(decoded, output);
}
