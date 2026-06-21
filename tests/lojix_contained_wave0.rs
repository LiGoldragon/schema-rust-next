use schema_next::{ImportResolver, MacroContext, SchemaEngine, SchemaIdentity};
use schema_rust_next::{RustEmissionOptions, RustEmissionTarget, RustEmitter};

mod support;

use support::FixtureSchemaDirectory;

#[derive(Debug, Clone)]
struct LojixContainedWaveZeroFixture {
    signal: FixtureSchemaDirectory,
    meta: FixtureSchemaDirectory,
    daemon: FixtureSchemaDirectory,
}

impl LojixContainedWaveZeroFixture {
    fn new() -> Self {
        Self {
            signal: FixtureSchemaDirectory::new("lojix-contained-wave0-signal"),
            meta: FixtureSchemaDirectory::new("lojix-contained-wave0-meta"),
            daemon: FixtureSchemaDirectory::new("lojix-contained-wave0-daemon"),
        }
    }

    fn resolver(&self) -> ImportResolver {
        ImportResolver::new()
            .with_dependency("signal-lojix", self.signal.path(), "0.1.0")
            .with_dependency("meta-signal-lojix", self.meta.path(), "0.1.0")
    }

    fn emit_signal_contract(&self) -> String {
        self.emit_with_target(
            &self.signal,
            "lib.schema",
            "signal-lojix:lib",
            RustEmissionTarget::WireContract,
        )
    }

    fn emit_meta_contract(&self) -> String {
        self.emit_with_target(
            &self.meta,
            "lib.schema",
            "meta-signal-lojix:lib",
            RustEmissionTarget::WireContract,
        )
    }

    fn emit_daemon_nexus(&self) -> String {
        self.emit_with_target(
            &self.daemon,
            "nexus.schema",
            "lojix:nexus",
            RustEmissionTarget::NexusRuntime,
        )
    }

    fn emit_with_target(
        &self,
        directory: &FixtureSchemaDirectory,
        module_path: &str,
        identity: &str,
        target: RustEmissionTarget,
    ) -> String {
        let source = directory.schema(module_path).read();
        let schema = SchemaEngine::default()
            .lower_source_with_resolver(
                &source,
                SchemaIdentity::new(identity, "0.1.0"),
                &mut MacroContext::default(),
                &self.resolver(),
            )
            .expect("wave-zero schema lowers through the current toolchain");
        RustEmitter::new(RustEmissionOptions::binary_only().with_target(target))
            .emit_code_from_schema(&schema)
            .as_str()
            .to_owned()
    }
}

#[test]
fn ordinary_and_meta_contracts_keep_contained_and_production_targets_unrelated() {
    let fixture = LojixContainedWaveZeroFixture::new();
    let ordinary = fixture.emit_signal_contract();
    let meta = fixture.emit_meta_contract();

    assert!(
        ordinary.contains("pub enum ContainedTarget"),
        "ordinary contract must own the contained target type:\n{ordinary}"
    );
    assert!(
        ordinary.contains("pub struct DeployContainedRequest"),
        "ordinary contract must own DeployContainedRequest:\n{ordinary}"
    );
    assert!(
        ordinary.contains("pub contained_target: ContainedTarget"),
        "ordinary deploy request must carry only ContainedTarget:\n{ordinary}"
    );
    assert!(
        !ordinary.contains("ProductionNode"),
        "ordinary contract must not name the production target:\n{ordinary}"
    );

    assert!(
        meta.contains("pub struct ProductionNode"),
        "meta contract must own the production target type:\n{meta}"
    );
    assert!(
        meta.contains("pub production_node: ProductionNode"),
        "meta deploy request must carry only ProductionNode:\n{meta}"
    );
    assert!(
        meta.contains("pub use signal_lojix::schema::lib::DeployClosure as DeployClosure;"),
        "meta contract should reuse the shared deploy body by alias:\n{meta}"
    );
    assert!(
        !meta.contains("pub enum ContainedTarget"),
        "meta contract must not re-declare the contained target:\n{meta}"
    );
}

#[test]
fn daemon_nexus_routes_both_faces_without_a_shared_target_supertype() {
    let fixture = LojixContainedWaveZeroFixture::new();
    let nexus = fixture.emit_daemon_nexus();

    assert!(
        nexus.contains("pub use signal_lojix::schema::lib::DeployClosure as DeployClosure;"),
        "daemon nexus must import the shared deploy body:\n{nexus}"
    );
    assert!(
        nexus.contains("pub use signal_lojix::schema::lib::ContainedTarget as ContainedTarget;"),
        "daemon nexus must import contained target directly:\n{nexus}"
    );
    assert!(
        nexus.contains("pub use meta_signal_lojix::schema::lib::ProductionNode as ProductionNode;"),
        "daemon nexus must import production target directly:\n{nexus}"
    );
    assert!(
        nexus.contains("pub struct ContainedPipelineCommand"),
        "daemon nexus must emit the contained pipeline command:\n{nexus}"
    );
    assert!(
        nexus.contains("pub struct ProductionPipelineCommand"),
        "daemon nexus must emit the production pipeline command:\n{nexus}"
    );
    assert!(
        nexus.contains("pub contained_target: ContainedTarget"),
        "contained command must carry ContainedTarget:\n{nexus}"
    );
    assert!(
        nexus.contains("pub production_node: ProductionNode"),
        "production command must carry ProductionNode:\n{nexus}"
    );
    assert!(
        nexus.contains("RunContainedDeploy(ContainedPipelineCommand)"),
        "nexus action must route contained deploys through their command:\n{nexus}"
    );
    assert!(
        nexus.contains("RunProductionDeploy(ProductionPipelineCommand)"),
        "nexus action must route production deploys through their command:\n{nexus}"
    );

    for forbidden in [
        "ContainedOrProduction",
        "ProductionOrContained",
        "ContainedProductionTarget",
        "ProductionContainedTarget",
        "enum DeployTarget",
        "struct DeployTarget",
    ] {
        assert!(
            !nexus.contains(forbidden),
            "daemon nexus must not synthesize shared target supertype {forbidden}:\n{nexus}"
        );
    }
}
