use schema_rust_next::build::{DependencySchema, GenerationDriver, GenerationPlan, ModuleEmission};

mod support;

use support::FixtureSchemaDirectory;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DriverFixture {
    contract: FixtureSchemaDirectory,
    runtime: FixtureSchemaDirectory,
}

impl DriverFixture {
    fn new() -> Self {
        Self {
            contract: FixtureSchemaDirectory::new("driver-contract"),
            runtime: FixtureSchemaDirectory::new("driver-runtime"),
        }
    }

    fn runtime_plan(&self) -> GenerationPlan {
        GenerationPlan::daemon_runtime(self.runtime.crate_root(), "driver-runtime", "0.1.0")
            .with_dependency_schema(DependencySchema::new(
                "driver-contract",
                self.contract.path(),
                "0.1.0",
            ))
    }

    fn generated_runtime(&self) -> schema_rust_next::build::GeneratedPackage {
        GenerationDriver::new(self.runtime_plan())
            .generate()
            .expect("driver emits runtime package")
    }
}

#[test]
fn daemon_runtime_driver_emits_nexus_and_sema_files_with_plane_targets() {
    let generated = DriverFixture::new().generated_runtime();
    let nexus = generated
        .rust_file_named("src/schema/nexus.rs")
        .expect("nexus runtime file")
        .code
        .as_str();
    let sema = generated
        .rust_file_named("src/schema/sema.rs")
        .expect("sema runtime file")
        .code
        .as_str();

    assert!(
        nexus.contains("pub use driver_contract::schema::lib::Input as ContractInput;"),
        "nexus should import contract wire root through a Rust alias:\n{nexus}"
    );
    assert!(
        nexus.contains("pub trait NexusEngine"),
        "nexus runtime target should emit NexusEngine:\n{nexus}"
    );
    assert!(
        nexus.contains("#[cfg(feature = \"nota-text\")]\npub use nota_next::{"),
        "nexus runtime target should keep its NOTA surface feature-gated:\n{nexus}"
    );
    assert!(
        nexus.contains(
            "#[cfg_attr(feature = \"nota-text\", derive(nota_next::NotaDecode, nota_next::NotaEncode))]"
        ),
        "nexus runtime support nouns should derive NOTA only behind the feature:\n{nexus}"
    );
    assert!(
        nexus.contains("pub type NexusRunnerNextStep = triad_runtime::NextStep<ContractOutput, SemaWriteInput, SemaReadInput, std::convert::Infallible, NexusWork>;"),
        "nexus runtime target should emit runner glue over imported contract output:\n{nexus}"
    );
    assert!(
        nexus.contains("impl triad_runtime::NexusWork for NexusWork {}"),
        "nexus runtime target should mark the local work enum with the runtime role trait:\n{nexus}"
    );
    assert!(
        !nexus.contains("impl triad_runtime::SemaWriteInput for SemaWriteInput {}"),
        "nexus runtime target must not re-implement role traits for imported SEMA roots:\n{nexus}"
    );
    assert!(
        !nexus.contains("impl triad_runtime::SemaReadInput for SemaReadInput {}"),
        "nexus runtime target must not re-implement role traits for imported SEMA roots:\n{nexus}"
    );
    assert!(
        nexus.contains(
            "fn budget_exhausted_reply(&self, exhausted: triad_runtime::ContinuationExhausted) -> ContractOutput;"
        ),
        "nexus runtime target should ask the component for a typed exhaustion reply:\n{nexus}"
    );
    assert!(
        !nexus.contains("fn run_effect(&mut self, input"),
        "nexus runtime target should not require an effect hook without CommandEffect:\n{nexus}"
    );
    assert!(
        !nexus.contains("pub trait SemaEngine"),
        "nexus runtime target must not emit SemaEngine:\n{nexus}"
    );
    assert!(
        sema.contains("pub trait SemaEngine"),
        "sema runtime target should emit SemaEngine:\n{sema}"
    );
    assert!(
        sema.contains("impl triad_runtime::SemaWriteInput for SemaWriteInput {}"),
        "sema runtime target should mark its local write input root with the runtime role trait:\n{sema}"
    );
    assert!(
        sema.contains("impl triad_runtime::SemaReadInput for SemaReadInput {}"),
        "sema runtime target should mark its local read input root with the runtime role trait:\n{sema}"
    );
    assert!(
        sema.contains("#[cfg(feature = \"nota-text\")]\npub use nota_next::{"),
        "sema runtime target should keep its NOTA surface feature-gated:\n{sema}"
    );
    assert!(
        sema.contains(
            "#[cfg_attr(feature = \"nota-text\", derive(nota_next::NotaDecode, nota_next::NotaEncode))]"
        ),
        "sema runtime support nouns should derive NOTA only behind the feature:\n{sema}"
    );
    assert!(
        !sema.contains("pub trait NexusEngine"),
        "sema runtime target must not emit NexusEngine:\n{sema}"
    );
}

#[test]
fn generated_package_carries_source_and_rust_artifacts() {
    let generated = DriverFixture::new().generated_runtime();
    let module = generated
        .modules()
        .iter()
        .find(|module| module.module().as_str() == "nexus")
        .expect("nexus module");

    assert_eq!(
        module.source_artifact().path(),
        DriverFixture::new().runtime.path().join("nexus.schema")
    );
    assert_eq!(
        module.source_artifact().content(),
        "{\n  ContractInput driver-contract:lib:Input\n  ContractOutput driver-contract:lib:Output\n  SemaReadInput driver-runtime:sema:SemaReadInput\n  SemaReadOutput driver-runtime:sema:SemaReadOutput\n  SemaWriteInput driver-runtime:sema:SemaWriteInput\n  SemaWriteOutput driver-runtime:sema:SemaWriteOutput\n}\n[(SignalArrived ContractInput)]\n[(CommandSemaRead SemaReadInput) (CommandSemaWrite SemaWriteInput) (ReplyToSignal ContractOutput)]\n{\n  NexusWork [(SignalArrived ContractInput) (SemaReadCompleted SemaReadOutput) (SemaWriteCompleted SemaWriteOutput)]\n  NexusAction [(CommandSemaRead SemaReadInput) (CommandSemaWrite SemaWriteInput) (ReplyToSignal ContractOutput)]\n  DecisionReceipt { integer Integer }\n}"
    );
    assert_eq!(module.rust_file().path, "src/schema/nexus.rs");
    assert!(
        module
            .rust_file()
            .code
            .as_str()
            .contains("pub trait NexusEngine"),
        "driver should emit Rust from the typed schema source value"
    );
}

#[test]
fn component_runtime_compatibility_keeps_lib_component_runtime_explicit() {
    let plan = GenerationPlan::component_runtime_compatibility(
        FixtureSchemaDirectory::new("driver-contract").crate_root(),
        "driver-contract",
        "0.1.0",
    );
    assert_eq!(plan.modules(), &[ModuleEmission::lib_component_runtime()]);
}

#[test]
fn signal_runtime_module_selects_the_signal_runtime_target() {
    let plan = GenerationPlan::new(
        FixtureSchemaDirectory::new("driver-contract").crate_root(),
        "driver-contract",
        "0.1.0",
    )
    .with_module(ModuleEmission::signal_runtime_module("signal"));

    assert_eq!(
        plan.modules(),
        &[ModuleEmission::signal_runtime_module("signal")]
    );
}
