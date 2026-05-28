use schema_next::{Asschema, Name, SchemaEngine, SchemaModuleSource, SchemaPackage};
use schema_rust_next::RustEmitter;
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

struct HorizonConcept {
    package: SchemaPackage,
    engine: SchemaEngine,
    emitter: RustEmitter,
    output_directory: PathBuf,
}

impl HorizonConcept {
    fn from_environment() -> Self {
        let manifest_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
        let package = SchemaPackage::new(
            manifest_directory.join("concept").join("horizon"),
            "horizon-concept",
            "0.1.0",
        );
        let output_directory = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
            manifest_directory
                .join("target")
                .join("horizon-schema-concept")
        });

        Self {
            package,
            engine: SchemaEngine::default(),
            emitter: RustEmitter::default(),
            output_directory,
        }
    }

    fn run(&self) -> Result<(), Box<dyn Error>> {
        self.reset_output_directory()?;
        self.write_pipeline_note()?;
        self.write_module(self.package.load_module(Name::new("proposal"))?)?;
        self.write_module(self.package.load_module(Name::new("view"))?)?;
        self.write_module(self.package.load_lib()?)?;
        Ok(())
    }

    fn reset_output_directory(&self) -> Result<(), Box<dyn Error>> {
        if self.output_directory.exists() {
            fs::remove_dir_all(&self.output_directory)?;
        }
        fs::create_dir_all(&self.output_directory)?;
        Ok(())
    }

    fn write_pipeline_note(&self) -> Result<(), Box<dyn Error>> {
        self.write_file(
            "00-pipeline.txt",
            "Horizon pure-schema concept pipeline\n\
             1. Load schema/*.schema through schema-next::SchemaPackage.\n\
             2. Lower each module with SchemaEngine into Asschema.\n\
             3. Emit Rust data types with schema-rust-next::RustEmitter.\n\
             4. Compile the generated files in tests/horizon_concept.rs.\n",
        )
    }

    fn write_module(&self, source: SchemaModuleSource) -> Result<(), Box<dyn Error>> {
        let asschema = source.lower(&self.engine)?;
        let module_name = asschema.identity().component().local_part().to_owned();
        let generated = self.emitter.emit_file(&asschema);

        self.write_file(
            format!("01-input-schema/{module_name}.schema"),
            source.source(),
        )?;
        self.write_file(
            format!("02-assembled-schema/{module_name}.asschema.debug"),
            self.describe_asschema(&asschema),
        )?;
        self.write_file(
            format!("03-generated-rust/{}", generated.path),
            generated.code.as_str(),
        )?;
        Ok(())
    }

    fn describe_asschema(&self, asschema: &Asschema) -> String {
        format!("{asschema:#?}")
    }

    fn write_file(
        &self,
        relative_path: impl AsRef<Path>,
        content: impl AsRef<str>,
    ) -> Result<(), Box<dyn Error>> {
        let path = self.output_directory.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content.as_ref())?;
        Ok(())
    }
}

fn main() {
    if let Err(error) = HorizonConcept::from_environment().run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
