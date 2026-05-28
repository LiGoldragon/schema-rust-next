use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;
use std::cell::Cell;

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
    let source = include_str!("fixtures/spirit-min.schema");
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("spirit:lib", "0.1.0"))
        .expect("schema lowers");
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
    assert!(generated.code.as_str().contains("pub trait InputNexus"));
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
    assert_eq!(
        generated.code.as_str(),
        include_str!("fixtures/spirit_generated.rs")
    );
}

#[test]
fn emitted_path_mirrors_schema_module_identity() {
    let source = include_str!("fixtures/spirit-min.schema");
    let asschema = SchemaEngine::default()
        .lower_source(
            source,
            SchemaIdentity::new("spirit-next:signal:public", "0.1.0"),
        )
        .expect("schema lowers");
    let generated = RustEmitter::default().emit_file(&asschema);

    assert_eq!(generated.path, "src/schema/signal/public.rs");
}

#[test]
fn emits_schema_plane_engine_traits_for_declared_nexus_and_sema_languages() {
    let source = "\
((Record Entry) (Observe Query))
((RecordAccepted SemaReceipt) (RecordsObserved ObservedRecords) (Error ErrorReport))
{
  NexusInput ((Signal Input) (Sema SemaOutput))
  NexusOutput ((Sema SemaInput) (Signal Output))
  SemaInput ((Record Entry) (Observe Query))
  SemaOutput ((Recorded SemaReceipt) (Observed ObservedRecords) (Missed ErrorReport))
  Topic [Text]
  Description [Text]
  ErrorMessage [Text]
  RecordIdentifier [Integer]
  CommitSequence [Integer]
  StateDigest [Integer]
  DatabaseMarker [CommitSequence StateDigest]
  SemaReceipt [RecordIdentifier DatabaseMarker]
  ObservedRecords [RecordSet DatabaseMarker]
  ErrorReport [ErrorMessage DatabaseMarker]
  Entry [Topic Kind Description Magnitude]
  Query [Topic Kind]
  RecordSet [(Vec Entry)]
  Kind (Decision Principle Correction Clarification Constraint)
  Magnitude (Minimum VeryLow Low Medium High VeryHigh Maximum)
}";
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("spirit:lib", "0.1.0"))
        .expect("schema lowers");
    let generated = RustEmitter::default().emit_file(&asschema);

    assert!(generated.code.as_str().contains("pub trait NexusEngine"));
    assert!(generated.code.as_str().contains("pub mod nexus"));
    assert!(generated.code.as_str().contains("pub mod sema"));
    assert!(generated.code.as_str().contains(
        "fn execute(&self, input: nexus::Nexus<nexus::Input>) -> nexus::Nexus<nexus::Output>;"
    ));
    assert!(generated.code.as_str().contains("pub trait SemaEngine"));
    assert!(generated.code.as_str().contains(
        "fn apply(&mut self, input: sema::Sema<sema::Input>) -> sema::Sema<sema::Output>;"
    ));
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
    let input = "(Observe ([schema] Principle))"
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
    let input = "(Record ([[schema]] Constraint [agent's clarified intent] Maximum))"
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

    assert_eq!(
        input.to_string(),
        "(Record ([[schema]] Constraint [agent's clarified intent] Maximum))"
    );
}

#[test]
fn generated_signal_input_round_trips_from_nota_to_rkyv_bytes() {
    let input = "(Record ([[schema]] Constraint [component messages use binary rkyv] Maximum))"
        .parse::<generated::Input>()
        .expect("parse generated input");

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&input).expect("archive input");
    let decoded =
        rkyv::from_bytes::<generated::Input, rkyv::rancor::Error>(&bytes).expect("decode input");

    assert_eq!(decoded, input);
}

#[test]
fn generated_signal_frame_methods_round_trip_and_triage_route() {
    let input = "(Record ([[schema]] Constraint [schema owns signal frames] Maximum))"
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
    processed_events: Vec<generated::MessageProcessed<RuntimeReply>>,
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

impl generated::MessageProcessedHook<RuntimeReply> for MailHook {
    type Error = RuntimeError;

    fn message_processed(
        &mut self,
        event: generated::MessageProcessed<RuntimeReply>,
    ) -> Result<(), Self::Error> {
        self.processed_events.push(event);
        Ok(())
    }
}

#[test]
fn generated_signal_roots_emit_typed_message_sent_events() {
    let input = "(Observe ([schema] Principle))"
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
    assert_ne!(
        event.origin_route(),
        generated::OriginRoute(event.identifier.0),
        "origin route is minted separately from the message identifier"
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeReply {
    Recorded(String),
    Observed(String),
}

#[derive(Debug, PartialEq, Eq)]
enum RuntimeError {
    StateRejected,
}

struct SpiritNexus {
    accepted_records: Cell<usize>,
    last_mail_identifier: Cell<Option<generated::MessageIdentifier>>,
}

impl SpiritNexus {
    fn new() -> Self {
        Self {
            accepted_records: Cell::new(0),
            last_mail_identifier: Cell::new(None),
        }
    }

    fn accepted_records(&self) -> usize {
        self.accepted_records.get()
    }
}

impl generated::InputNexus for SpiritNexus {
    type Reply = RuntimeReply;
    type Error = RuntimeError;

    fn record(
        &self,
        mail: generated::NexusMail<generated::Entry>,
    ) -> Result<Self::Reply, Self::Error> {
        self.last_mail_identifier.set(Some(mail.identifier()));
        self.accepted_records.set(self.accepted_records.get() + 1);
        let payload = mail.into_payload();
        Ok(RuntimeReply::Recorded(payload.description.0))
    }

    fn observe(
        &self,
        mail: generated::NexusMail<generated::Query>,
    ) -> Result<Self::Reply, Self::Error> {
        self.last_mail_identifier.set(Some(mail.identifier()));
        let payload = mail.into_payload();
        Ok(RuntimeReply::Observed(payload.topic.0))
    }
}

#[test]
fn generated_input_dispatches_mail_through_schema_emitted_nexus_trait_methods() {
    assert_eq!(RuntimeError::StateRejected, RuntimeError::StateRejected);
    let input = "(Record ([[schema]] Principle [schema objects drive behavior] Maximum))"
        .parse::<generated::Input>()
        .expect("parse generated input");
    let nexus = SpiritNexus::new();
    let mut hook = MailHook::new();

    let processed = input
        .dispatch_mail_with_nexus(
            generated::MessageIdentifier(77),
            generated::OriginRoute(701),
            &nexus,
        )
        .expect("input dispatches through generated nexus trait");
    processed
        .push_to(&mut hook)
        .expect("processed mail event pushes");

    assert_eq!(
        processed.reply,
        RuntimeReply::Recorded("schema objects drive behavior".to_owned())
    );
    assert_eq!(nexus.accepted_records(), 1);
    assert_eq!(
        nexus.last_mail_identifier.get(),
        Some(generated::MessageIdentifier(77))
    );
    assert_eq!(
        hook.processed_events,
        vec![generated::MessageProcessed {
            identifier: generated::MessageIdentifier(77),
            origin_route: generated::OriginRoute(701),
            reply: RuntimeReply::Recorded("schema objects drive behavior".to_owned()),
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
fn emits_vec_map_and_option_collection_types_with_runtime_codec() {
    let source = include_str!("fixtures/collections.schema");
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("collections:lib", "0.1.0"))
        .expect("schema lowers");
    let generated = RustEmitter::default().emit(&asschema);
    let code = generated.as_str();

    // The collection-support runtime block is emitted because the
    // schema uses collections.
    assert!(code.contains("pub struct NotaCollection"));
    // Vec / KeyValue->BTreeMap / Option render at the field positions.
    assert!(code.contains("pub services: Vec<Service>,"));
    assert!(code.contains("pub nodes: std::collections::BTreeMap<NodeName, NodeConfig>,"));
    assert!(code.contains("pub cache: Option<NodeConfig>,"));
    // Collection payloads in a root output variant.
    assert!(code.contains("Projected(std::collections::BTreeMap<NodeName, NodeConfig>),"));
    assert!(code.contains("Listed(Vec<NodeName>),"));
    // A map key type earns the ordering derives so BTreeMap compiles;
    // a value-only type keeps the original derive set.
    assert!(code.contains(
        "#[rkyv(derive(PartialEq, Eq, PartialOrd, Ord))]\npub struct NodeName(pub Text);"
    ));
    assert!(code.contains(
        "#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]\npub struct NodeConfig(pub Text);"
    ));
}

#[test]
fn collection_free_schema_emits_byte_identical_to_legacy_fixture() {
    // The regression safety net: a schema that uses no collection still
    // emits exactly the checked-in fixture, proving the collection work
    // is purely additive.
    let source = include_str!("fixtures/spirit-min.schema");
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("spirit:lib", "0.1.0"))
        .expect("schema lowers");
    let generated = RustEmitter::default().emit(&asschema);

    assert_eq!(
        generated.as_str(),
        include_str!("fixtures/spirit_generated.rs")
    );
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
