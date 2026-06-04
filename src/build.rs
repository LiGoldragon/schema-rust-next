use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use schema_next::{
    AsschemaArtifact, ImportResolver, Name, SchemaEngine, SchemaError, SchemaPackage,
    SchemaSourceArtifact,
};

use crate::{GeneratedFile, RustEmissionOptions, RustEmissionTarget, RustEmitter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationPlan {
    package: SchemaPackage,
    modules: Vec<ModuleEmission>,
    dependencies: Vec<DependencySchema>,
}

impl GenerationPlan {
    pub fn new(
        crate_root: impl Into<PathBuf>,
        crate_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            package: SchemaPackage::new(crate_root, crate_name, version),
            modules: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn wire_contract(
        crate_root: impl Into<PathBuf>,
        crate_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::new(crate_root, crate_name, version).with_module(ModuleEmission::wire_contract())
    }

    pub fn daemon_runtime(
        crate_root: impl Into<PathBuf>,
        crate_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::new(crate_root, crate_name, version)
            .with_module(ModuleEmission::nexus_runtime())
            .with_module(ModuleEmission::sema_runtime())
    }

    pub fn component_runtime_compatibility(
        crate_root: impl Into<PathBuf>,
        crate_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self::new(crate_root, crate_name, version)
            .with_module(ModuleEmission::lib_component_runtime())
    }

    pub fn with_module(mut self, module: ModuleEmission) -> Self {
        self.modules.push(module);
        self
    }

    pub fn with_dependency_schema(mut self, dependency: DependencySchema) -> Self {
        self.dependencies.push(dependency);
        self
    }

    pub fn with_optional_dependency_schema(mut self, dependency: Option<DependencySchema>) -> Self {
        if let Some(dependency) = dependency {
            self.dependencies.push(dependency);
        }
        self
    }

    pub fn with_dependency_schema_directory(
        self,
        crate_name: impl Into<String>,
        schema_directory: impl Into<PathBuf>,
        version: impl Into<String>,
    ) -> Self {
        self.with_dependency_schema(DependencySchema::new(crate_name, schema_directory, version))
    }

    pub fn package(&self) -> &SchemaPackage {
        &self.package
    }

    pub fn modules(&self) -> &[ModuleEmission] {
        &self.modules
    }

    pub fn dependencies(&self) -> &[DependencySchema] {
        &self.dependencies
    }

    fn import_resolver(&self) -> ImportResolver {
        self.dependencies.iter().fold(
            ImportResolver::new().with_package(self.package.clone()),
            |resolver, dependency| dependency.register(resolver),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleEmission {
    module: Name,
    options: RustEmissionOptions,
}

impl ModuleEmission {
    pub fn new(module: impl Into<String>, options: RustEmissionOptions) -> Self {
        Self {
            module: Name::new(module),
            options,
        }
    }

    pub fn wire_contract() -> Self {
        Self::wire_contract_module("lib")
    }

    pub fn wire_contract_module(module: impl Into<String>) -> Self {
        Self::new(
            module,
            RustEmissionOptions::feature_gated_nota("nota-text")
                .with_target(RustEmissionTarget::WireContract),
        )
    }

    pub fn nexus_runtime() -> Self {
        Self::new(
            "nexus",
            RustEmissionOptions::binary_only().with_target(RustEmissionTarget::NexusRuntime),
        )
    }

    pub fn sema_runtime() -> Self {
        Self::new(
            "sema",
            RustEmissionOptions::binary_only().with_target(RustEmissionTarget::SemaRuntime),
        )
    }

    pub fn lib_component_runtime() -> Self {
        Self::new(
            "lib",
            RustEmissionOptions::feature_gated_nota("nota-text")
                .with_target(RustEmissionTarget::ComponentRuntime),
        )
    }

    pub fn module(&self) -> &Name {
        &self.module
    }

    pub fn options(&self) -> &RustEmissionOptions {
        &self.options
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencySchema {
    crate_name: String,
    schema_directory: PathBuf,
    version: String,
}

impl DependencySchema {
    pub fn new(
        crate_name: impl Into<String>,
        schema_directory: impl Into<PathBuf>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            crate_name: crate_name.into(),
            schema_directory: schema_directory.into(),
            version: version.into(),
        }
    }

    pub fn from_cargo_metadata(
        crate_name: impl Into<String>,
        links_name: impl Into<String>,
        version: impl Into<String>,
    ) -> Result<Option<Self>, BuildError> {
        let crate_name = crate_name.into();
        let version = version.into();
        let metadata = CargoSchemaMetadata::new(links_name);
        let Some(schema_directory) = metadata.schema_directory()? else {
            return Ok(None);
        };
        Ok(Some(Self::new(crate_name, schema_directory, version)))
    }

    pub fn crate_name(&self) -> &str {
        &self.crate_name
    }

    pub fn schema_directory(&self) -> &Path {
        &self.schema_directory
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    fn register(&self, resolver: ImportResolver) -> ImportResolver {
        resolver.with_dependency(
            self.crate_name.clone(),
            self.schema_directory.clone(),
            self.version.clone(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoSchemaMetadata {
    links_name: String,
}

impl CargoSchemaMetadata {
    pub fn new(links_name: impl Into<String>) -> Self {
        Self {
            links_name: links_name.into(),
        }
    }

    pub fn emit_schema_directory(&self, crate_root: &Path) {
        let schema_directory = crate_root.join("schema");
        println!("cargo::metadata=schema-dir={}", schema_directory.display());
    }

    pub fn schema_directory(&self) -> Result<Option<PathBuf>, BuildError> {
        let variable = self.schema_directory_variable();
        match env::var_os(&variable) {
            Some(value) => Ok(Some(PathBuf::from(value))),
            None => Ok(None),
        }
    }

    pub fn schema_directory_variable(&self) -> String {
        format!("DEP_{}_SCHEMA_DIR", self.normalized_links_name())
    }

    fn normalized_links_name(&self) -> String {
        self.links_name
            .chars()
            .map(|character| match character {
                '-' => '_',
                other => other.to_ascii_uppercase(),
            })
            .collect()
    }
}

pub struct GenerationDriver {
    plan: GenerationPlan,
    engine: SchemaEngine,
}

impl GenerationDriver {
    pub fn new(plan: GenerationPlan) -> Self {
        Self {
            plan,
            engine: SchemaEngine::default(),
        }
    }

    pub fn with_engine(mut self, engine: SchemaEngine) -> Self {
        self.engine = engine;
        self
    }

    pub fn generate(&self) -> Result<GeneratedPackage, BuildError> {
        let resolver = self.plan.import_resolver();
        let mut modules = Vec::new();
        for emission in self.plan.modules() {
            modules.push(GeneratedModule::from_emission(
                self.plan.package(),
                emission,
                &self.engine,
                &resolver,
            )?);
        }
        Ok(GeneratedPackage::new(
            self.plan.package().root().to_path_buf(),
            modules,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedPackage {
    crate_root: PathBuf,
    modules: Vec<GeneratedModule>,
}

impl GeneratedPackage {
    pub fn new(crate_root: impl Into<PathBuf>, modules: Vec<GeneratedModule>) -> Self {
        Self {
            crate_root: crate_root.into(),
            modules,
        }
    }

    pub fn modules(&self) -> &[GeneratedModule] {
        &self.modules
    }

    pub fn rust_files(&self) -> Vec<&GeneratedFile> {
        self.modules
            .iter()
            .map(GeneratedModule::rust_file)
            .collect()
    }

    pub fn rust_file_named(&self, path: &str) -> Option<&GeneratedFile> {
        self.modules
            .iter()
            .map(GeneratedModule::rust_file)
            .find(|file| file.path == path)
    }

    pub fn assert_checked_in(&self) -> Result<(), BuildError> {
        self.check_with(FreshnessCheck::check_only())
    }

    pub fn write_or_check(
        self,
        update_environment_variable: impl Into<String>,
    ) -> Result<(), BuildError> {
        self.check_with(FreshnessCheck::from_environment(
            update_environment_variable,
        ))
    }

    fn check_with(&self, check: FreshnessCheck) -> Result<(), BuildError> {
        for module in &self.modules {
            module.check_generated_artifacts(&self.crate_root, &check)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedModule {
    module: Name,
    source_artifact: GeneratedArtifact,
    asschema_artifact: GeneratedArtifact,
    rust_file: GeneratedFile,
}

impl GeneratedModule {
    pub fn module(&self) -> &Name {
        &self.module
    }

    pub fn source_artifact(&self) -> &GeneratedArtifact {
        &self.source_artifact
    }

    pub fn asschema_artifact(&self) -> &GeneratedArtifact {
        &self.asschema_artifact
    }

    pub fn rust_file(&self) -> &GeneratedFile {
        &self.rust_file
    }

    fn from_emission(
        package: &SchemaPackage,
        emission: &ModuleEmission,
        engine: &SchemaEngine,
        resolver: &ImportResolver,
    ) -> Result<Self, BuildError> {
        let source = package.load_module(emission.module().clone())?;
        let source_artifact = SourceArtifactRoundTrip::new(
            source.path().to_path_buf(),
            SchemaSourceArtifact::new(source.to_schema_source()?),
        )
        .validate()?;
        let asschema = source.lower_with_resolver(engine, resolver)?;
        let asschema_artifact = AsschemaArtifact::new(asschema.clone()).to_nota_source();
        let rust_file = RustEmitter::new(emission.options().clone()).emit_file(&asschema);
        Ok(Self {
            module: emission.module().clone(),
            source_artifact,
            asschema_artifact: GeneratedArtifact::new(
                source.path().with_extension("asschema"),
                asschema_artifact,
            ),
            rust_file,
        })
    }

    fn check_generated_artifacts(
        &self,
        crate_root: &Path,
        check: &FreshnessCheck,
    ) -> Result<(), BuildError> {
        self.asschema_artifact.check_with(check)?;
        self.rust_artifact(crate_root).check_with(check)?;
        Ok(())
    }

    fn rust_artifact(&self, crate_root: &Path) -> GeneratedArtifact {
        GeneratedArtifact::new(
            crate_root.join(&self.rust_file.path),
            self.rust_file.code.as_str().to_owned(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceArtifactRoundTrip {
    path: PathBuf,
    artifact: SchemaSourceArtifact,
}

impl SourceArtifactRoundTrip {
    fn new(path: PathBuf, artifact: SchemaSourceArtifact) -> Self {
        Self { path, artifact }
    }

    fn validate(self) -> Result<GeneratedArtifact, BuildError> {
        let source_text = self.artifact.to_schema_text();
        let recovered = SchemaSourceArtifact::from_schema_text(&source_text)?;
        if recovered != self.artifact {
            return Err(BuildError::SchemaSourceRoundTrip { path: self.path });
        }
        Ok(GeneratedArtifact::new(self.path, source_text))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedArtifact {
    path: PathBuf,
    content: String,
}

impl GeneratedArtifact {
    pub fn new(path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    fn check_with(&self, check: &FreshnessCheck) -> Result<(), BuildError> {
        if check.updates_files() {
            self.write()?;
            return Ok(());
        }
        if self.matches_existing()? {
            return Ok(());
        }
        Err(BuildError::StaleGeneratedArtifact {
            path: self.path.clone(),
            update_environment_variable: check.update_environment_variable().to_owned(),
        })
    }

    fn matches_existing(&self) -> Result<bool, BuildError> {
        match fs::read_to_string(&self.path) {
            Ok(existing) => Ok(existing == self.content),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(BuildError::ReadGeneratedArtifact {
                path: self.path.clone(),
                source: error,
            }),
        }
    }

    fn write(&self) -> Result<(), BuildError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| BuildError::WriteGeneratedArtifact {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::write(&self.path, &self.content).map_err(|source| BuildError::WriteGeneratedArtifact {
            path: self.path.clone(),
            source,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FreshnessCheck {
    update_environment_variable: String,
    update_files: bool,
}

impl FreshnessCheck {
    fn check_only() -> Self {
        Self {
            update_environment_variable: "<update variable unavailable>".to_owned(),
            update_files: false,
        }
    }

    fn from_environment(update_environment_variable: impl Into<String>) -> Self {
        let update_environment_variable = update_environment_variable.into();
        let update_files = env::var_os(&update_environment_variable).is_some();
        Self {
            update_environment_variable,
            update_files,
        }
    }

    fn updates_files(&self) -> bool {
        self.update_files
    }

    fn update_environment_variable(&self) -> &str {
        &self.update_environment_variable
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("read generated artifact {path:?}: {source}")]
    ReadGeneratedArtifact {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("write generated artifact {path:?}: {source}")]
    WriteGeneratedArtifact {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "generated artifact {path:?} is stale; set {update_environment_variable}=1 to update it"
    )]
    StaleGeneratedArtifact {
        path: PathBuf,
        update_environment_variable: String,
    },
    #[error("schema source artifact did not round-trip through generated text at {path:?}")]
    SchemaSourceRoundTrip { path: PathBuf },
}
