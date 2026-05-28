use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;
use std::cell::Cell;

#[allow(dead_code)]
mod generated {
    include!("fixtures/spirit_generated.rs");
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
fn emits_the_data_carrying_plane_surface_and_three_engine_chain() {
    // Record 1054: the schema root plane surface is a single
    // data-carrying enum named `Plane` whose Signal / Nexus / Sema
    // variants carry the actual plane messages with the auto-created
    // origin route (records 1038/1039) folded onto the root, so runtime
    // code matches directly on the plane (record 1052 names the kind-
    // tag-beside-envelope shape wrong). Records 1028/1030: the three
    // trait-ordered engines drive a real chain, not dead scaffolding.
    let source = "\
{}
(Input ((Record Entry) (Observe Query)))
(Output ((RecordAccepted SemaReceipt) (RecordsObserved ObservedRecords) (Error ErrorReport)))
{
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
  RecordSet [Entry]
  Kind (Decision Principle Correction Clarification Constraint)
  Magnitude (Minimum VeryLow Low Medium High VeryHigh Maximum)
}";
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("spirit:lib", "0.1.0"))
        .expect("schema lowers");
    let generated = RustEmitter::default().emit_file(&asschema);
    let code = generated.code.as_str();

    // The data-carrying Plane enum: variants carry the messages, with
    // the origin route as the leading element (record 1054 + 1038/1039).
    assert!(code.contains("pub enum Plane {"));
    assert!(code.contains("Signal(OriginRoute, Input),"));
    assert!(code.contains("Nexus(OriginRoute, Input),"));
    assert!(code.contains("Sema(OriginRoute, Output),"));

    // The three trait-ordered engines (record 1028).
    assert!(code.contains("pub trait SignalEngine"));
    assert!(code.contains("fn admit(&self, signal: Plane) -> Result<Plane, Self::Error>;"));
    assert!(code.contains("pub trait NexusEngine"));
    assert!(code.contains("fn execute(&self, nexus: Plane) -> Result<Plane, Self::Error>;"));
    assert!(code.contains("pub trait SemaEngine"));
    assert!(code.contains("fn apply(&mut self, sema: Plane) -> Result<Plane, Self::Error>;"));

    // The running chain that drives Signal -> Nexus -> Sema (record 1030).
    assert!(code.contains("pub fn drive<Signal, Nexus, Sema>("));
    assert!(code.contains("let admitted = signal.admit(self)"));
    assert!(code.contains("let executed = nexus.execute(admitted)"));
    assert!(code.contains("let reply = sema.apply(executed)"));
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
    assert_eq!(input.route(), generated::InputRoute::Record);
    assert_eq!(input.short_header(), generated::short_header::INPUT_RECORD);
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

#[test]
fn generated_signal_frame_methods_round_trip_and_triage_route() {
    let input = "(Record ([schema] Constraint [schema owns signal frames] Maximum))"
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
    let event = input.message_sent(generated::MessageIdentifier(42), generated::OriginRoute(7));
    let mut hook = MailHook::new();

    event.push_to(&mut hook).expect("message sent event pushes");

    assert_eq!(
        hook.sent_events,
        vec![generated::MessageSent {
            identifier: generated::MessageIdentifier(42),
            origin_route: generated::OriginRoute(7),
            root: generated::MessageRoot::Input,
            short_header: generated::short_header::INPUT_OBSERVE,
        }],
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
    let input = "(Record ([schema] Principle [schema objects drive behavior] Maximum))"
        .parse::<generated::Input>()
        .expect("parse generated input");
    let nexus = SpiritNexus::new();
    let mut hook = MailHook::new();

    let processed = input
        .dispatch_mail_with_nexus(
            generated::MessageIdentifier(77),
            generated::OriginRoute(13),
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
    // The origin route minted at dispatch threads onto the processed reply.
    assert_eq!(processed.origin_route(), generated::OriginRoute(13));
    assert_eq!(nexus.accepted_records(), 1);
    assert_eq!(
        nexus.last_mail_identifier.get(),
        Some(generated::MessageIdentifier(77))
    );
    assert_eq!(
        hook.processed_events,
        vec![generated::MessageProcessed {
            identifier: generated::MessageIdentifier(77),
            origin_route: generated::OriginRoute(13),
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
            topics: generated::Topics(generated::Topic(previous.topic)),
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
            description: "accepted previous Entry as ([schema] Clarification [old client spoke previous entry] High)".to_owned(),
        },
    );
}
