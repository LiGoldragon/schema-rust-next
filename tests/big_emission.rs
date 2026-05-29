use std::path::{Path, PathBuf};

use schema_next::{
    Asschema, EnumDeclaration, ImportResolver, MacroContext, SchemaEngine, SchemaIdentity,
    TypeDeclaration,
};
use schema_rust_next::RustEmitter;

mod support;

use support::FixtureNota;

#[allow(dead_code)]
mod spirit_large_generated {
    include!("fixtures/big-schemas/spirit-reactive-large.generated.rs");
}

struct BigRustFixture<'fixture> {
    name: &'fixture str,
    identity: &'fixture str,
    source_path: PathBuf,
    rust_path: PathBuf,
    resolver: Option<ImportResolver>,
}

impl<'fixture> BigRustFixture<'fixture> {
    fn local(name: &'fixture str, identity: &'fixture str) -> Self {
        Self {
            name,
            identity,
            source_path: fixture_path(name, "schema"),
            rust_path: fixture_path(name, "generated.rs"),
            resolver: None,
        }
    }

    fn imported(name: &'fixture str, identity: &'fixture str) -> Self {
        let schema_dir = manifest_dir()
            .join("tests")
            .join("fixtures")
            .join("marker-core")
            .join("schema");
        Self {
            name,
            identity,
            source_path: fixture_path(name, "schema"),
            rust_path: fixture_path(name, "generated.rs"),
            resolver: Some(ImportResolver::new().with_dependency(
                "marker-core",
                schema_dir,
                "0.1.0",
            )),
        }
    }

    fn lower(&self) -> (Asschema, MacroContext) {
        let source = std::fs::read_to_string(&self.source_path).expect("read schema fixture");
        let engine = SchemaEngine::default();
        let mut context = MacroContext::default();
        let asschema = match &self.resolver {
            Some(resolver) => engine
                .lower_source_with_resolver(
                    &source,
                    SchemaIdentity::new(self.identity, "0.1.0"),
                    &mut context,
                    resolver,
                )
                .expect("schema with imports lowers"),
            None => engine
                .lower_source_with_context(
                    &source,
                    SchemaIdentity::new(self.identity, "0.1.0"),
                    &mut context,
                )
                .expect("schema lowers"),
        };
        (asschema, context)
    }

    fn generate_rust(&self) -> String {
        let (asschema, _) = self.lower();
        RustEmitter::default().emit(&asschema).as_str().to_owned()
    }

    fn assert_lowers_to_typed_asschema_data(&self) {
        let (asschema, context) = self.lower();
        self.assert_asschema_data_shape(&asschema, &context);
    }

    fn assert_asschema_data_shape(&self, asschema: &Asschema, context: &MacroContext) {
        assert_eq!(asschema.identity().component().as_str(), self.identity);
        assert_eq!(asschema.identity().version(), "0.1.0");
        assert!(
            !asschema.namespace().is_empty(),
            "{} must lower into typed namespace data",
            self.name
        );
        assert!(
            !asschema.input().variants.is_empty(),
            "{} must lower typed input variants",
            self.name
        );
        assert!(
            !asschema.output().variants.is_empty(),
            "{} must lower typed output variants",
            self.name
        );
        assert!(
            context.macros_applied().iter().any(|name| {
                name.contains("Struct") || name.contains("Enum") || name == "RootNamespace"
            }),
            "{} must exercise schema macro lowering",
            self.name
        );
        assert!(
            context.positions_seen().iter().any(|position| {
                position.as_str() == "RootNamespace" || position.as_str() == "NamespaceDeclaration"
            }),
            "{} must record structural macro positions",
            self.name
        );

        match self.name {
            "spirit-reactive-large" => {
                Self::assert_has_type(asschema.namespace(), "Entry");
                Self::assert_has_type(asschema.namespace(), "RecordSet");
                Self::assert_has_variant(asschema.input(), "Record");
                Self::assert_has_variant(asschema.output(), "Recorded");
            }
            "triad-reactive-large" => {
                Self::assert_has_type(asschema.namespace(), "SignalRequest");
                Self::assert_has_type(asschema.namespace(), "NexusRequest");
                Self::assert_has_type(asschema.namespace(), "SemaRequest");
                Self::assert_has_variant(asschema.input(), "SignalIn");
                Self::assert_has_variant(asschema.output(), "SignalOut");
            }
            "imported-mail-consumer" => {
                assert!(!asschema.imports().is_empty());
                assert!(!asschema.resolved_imports().is_empty());
                Self::assert_has_variant(asschema.output(), "Marked");
            }
            _ => panic!("unhandled big fixture {}", self.name),
        }
    }

    fn assert_has_type(declarations: &[TypeDeclaration], name: &str) {
        let found = declarations.iter().any(|declaration| match declaration {
            TypeDeclaration::Struct(declaration) | TypeDeclaration::Newtype(declaration) => {
                declaration.name.as_str() == name
            }
            TypeDeclaration::Enum(declaration) => declaration.name.as_str() == name,
        });
        assert!(found, "missing namespace type {name}");
    }

    fn assert_has_variant(declaration: &EnumDeclaration, name: &str) {
        assert!(
            declaration
                .variants
                .iter()
                .any(|variant| variant.name.as_str() == name),
            "missing variant {name} on {}",
            declaration.name.as_str()
        );
    }

    fn assert_matches_checked_in_rust(&self) {
        let generated = self.generate_rust();
        if std::env::var_os("SCHEMA_RUST_NEXT_UPDATE_BIG_EXAMPLES").is_some() {
            std::fs::write(&self.rust_path, &generated).expect("write generated Rust fixture");
        }
        let expected = std::fs::read_to_string(&self.rust_path).expect("read generated Rust");
        assert_eq!(
            generated, expected,
            "generated Rust drifted for {}",
            self.name
        );
    }
}

#[test]
fn large_spirit_schema_lowers_to_typed_asschema_data() {
    BigRustFixture::local("spirit-reactive-large", "example:spirit-reactive-large")
        .assert_lowers_to_typed_asschema_data();
}

#[test]
fn large_spirit_schema_emits_checked_rust_snapshot() {
    BigRustFixture::local("spirit-reactive-large", "example:spirit-reactive-large")
        .assert_matches_checked_in_rust();
}

#[test]
fn large_triad_schema_lowers_to_typed_asschema_data() {
    BigRustFixture::local("triad-reactive-large", "example:triad-reactive-large")
        .assert_lowers_to_typed_asschema_data();
}

#[test]
fn large_triad_schema_emits_checked_rust_snapshot() {
    BigRustFixture::local("triad-reactive-large", "example:triad-reactive-large")
        .assert_matches_checked_in_rust();
}

#[test]
fn large_imported_schema_lowers_to_typed_asschema_data() {
    BigRustFixture::imported("imported-mail-consumer", "example:imported-mail-consumer")
        .assert_lowers_to_typed_asschema_data();
}

#[test]
fn large_imported_schema_emits_checked_cross_crate_rust_snapshot() {
    BigRustFixture::imported("imported-mail-consumer", "example:imported-mail-consumer")
        .assert_matches_checked_in_rust();
}

#[test]
fn generated_big_rust_contains_the_current_schema_stack_surfaces() {
    let spirit = BigRustFixture::local("spirit-reactive-large", "example:spirit-reactive-large")
        .generate_rust();
    let triad = BigRustFixture::local("triad-reactive-large", "example:triad-reactive-large")
        .generate_rust();
    let imported =
        BigRustFixture::imported("imported-mail-consumer", "example:imported-mail-consumer")
            .generate_rust();

    assert!(spirit.contains("pub enum Input"));
    assert!(spirit.contains("pub enum Output"));
    assert!(spirit.contains("pub fn encode_signal_frame"));
    assert!(spirit.contains("pub struct OriginRoute"));
    assert!(spirit.contains("pub struct NexusMail<Payload>"));
    assert!(spirit.contains("pub trait InputNexus"));
    assert!(spirit.contains("pub topics: Topics,"));
    assert!(spirit.contains("pub records: Vec<Entry>,"));
    assert!(spirit.contains("pub by_topic: std::collections::BTreeMap<Topic, RecordIdentifier>,"));

    assert!(triad.contains("pub enum SignalRequest"));
    assert!(triad.contains("pub enum NexusRequest"));
    assert!(triad.contains("pub enum SemaRequest"));
    assert!(triad.contains("pub struct PushSemaResult"));
    assert!(triad.contains("pub struct EntryWritten"));
    assert!(triad.contains("pub enum RuntimeEvent"));

    assert!(
        imported.contains("pub use marker_core::schema::mail::DatabaseMarker as DatabaseMarker;")
    );
    assert!(
        imported.contains("pub use marker_core::schema::mail::CommitSequence as CommitSequence;")
    );
    assert!(imported.contains("pub database_marker: DatabaseMarker,"));
    assert!(imported.contains("Marked(DatabaseMarker)"));
    assert!(
        !imported.contains("pub struct DatabaseMarker"),
        "imported data types must be used by alias, not re-emitted"
    );
}

#[test]
fn compiled_large_spirit_generated_rust_parses_frames_and_dispatches_mail() {
    let input = FixtureNota::new("nota/large-record-schema-rust.nota")
        .read()
        .parse::<spirit_large_generated::Input>()
        .expect("large spirit input parses");
    let frame = input.encode_signal_frame().expect("signal frame encodes");
    let (route, decoded) =
        spirit_large_generated::Input::decode_signal_frame(&frame).expect("signal frame decodes");
    let nexus = SpiritLargeNexus::new();

    let processed = decoded
        .dispatch_mail_with_nexus(
            spirit_large_generated::MessageIdentifier(99),
            spirit_large_generated::OriginRoute(7001),
            &nexus,
        )
        .expect("generated mail dispatches");

    assert_eq!(route, spirit_large_generated::InputRoute::Record);
    assert_eq!(
        processed.origin_route,
        spirit_large_generated::OriginRoute(7001)
    );
    assert_eq!(processed.reply, SpiritLargeReply::Recorded(1));
}

#[derive(Debug, PartialEq, Eq)]
enum SpiritLargeReply {
    Recorded(usize),
    Corrected,
    Observed,
    Watching,
    Unwatched,
    Reindexed,
    Compacted,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum SpiritLargeError {
    Rejected,
}

struct SpiritLargeNexus {
    records_seen: std::cell::Cell<usize>,
}

impl SpiritLargeNexus {
    fn new() -> Self {
        Self {
            records_seen: std::cell::Cell::new(0),
        }
    }
}

impl spirit_large_generated::InputNexus for SpiritLargeNexus {
    type Reply = SpiritLargeReply;
    type Error = SpiritLargeError;

    fn record(
        &self,
        mail: spirit_large_generated::NexusMail<spirit_large_generated::Entry>,
    ) -> Result<Self::Reply, Self::Error> {
        let _entry = mail.into_payload();
        self.records_seen.set(self.records_seen.get() + 1);
        Ok(SpiritLargeReply::Recorded(self.records_seen.get()))
    }

    fn correct(
        &self,
        _mail: spirit_large_generated::NexusMail<spirit_large_generated::Correction>,
    ) -> Result<Self::Reply, Self::Error> {
        Ok(SpiritLargeReply::Corrected)
    }

    fn observe(
        &self,
        _mail: spirit_large_generated::NexusMail<spirit_large_generated::Query>,
    ) -> Result<Self::Reply, Self::Error> {
        Ok(SpiritLargeReply::Observed)
    }

    fn watch(
        &self,
        _mail: spirit_large_generated::NexusMail<spirit_large_generated::WatchRequest>,
    ) -> Result<Self::Reply, Self::Error> {
        Ok(SpiritLargeReply::Watching)
    }

    fn unwatch(
        &self,
        _mail: spirit_large_generated::NexusMail<spirit_large_generated::SubscriptionToken>,
    ) -> Result<Self::Reply, Self::Error> {
        Ok(SpiritLargeReply::Unwatched)
    }

    fn reindex(
        &self,
        _mail: spirit_large_generated::NexusMail<()>,
    ) -> Result<Self::Reply, Self::Error> {
        Ok(SpiritLargeReply::Reindexed)
    }

    fn compact(
        &self,
        _mail: spirit_large_generated::NexusMail<()>,
    ) -> Result<Self::Reply, Self::Error> {
        Ok(SpiritLargeReply::Compacted)
    }
}

fn fixture_path(name: &str, extension: &str) -> PathBuf {
    manifest_dir()
        .join("tests")
        .join("fixtures")
        .join("big-schemas")
        .join(format!("{name}.{extension}"))
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
