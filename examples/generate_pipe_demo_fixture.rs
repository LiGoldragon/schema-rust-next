use schema_next::{ImportResolver, MacroContext, SchemaEngine, SchemaIdentity};
use schema_rust_next::{RustEmissionOptions, RustEmissionTarget, RustEmitter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let reaction_dir = manifest.join("tests/fixtures/pipe-demo/schema");
    let resolver = ImportResolver::new().with_dependency(
        "reaction",
        reaction_dir.to_str().expect("reaction fixture path is utf-8"),
        "0.1.0",
    );
    let source = std::fs::read_to_string(manifest.join("tests/fixtures/pipe-demo/schema/ledger.schema"))?;
    let mut context = MacroContext::default();
    let schema = SchemaEngine::default().lower_source_with_resolver(
        &source,
        SchemaIdentity::new("ledger:core", "0.1.0"),
        &mut context,
        &resolver,
    )?;
    let options = RustEmissionOptions::feature_gated_nota("nota-text")
        .with_target(RustEmissionTarget::NexusRuntime);
    let code = RustEmitter::new(options)
        .emit_code_from_schema(&schema)
        .as_str()
        .to_owned();
    std::fs::write(
        manifest.join("tests/fixtures/pipe_demo_ledger_generated.rs"),
        &code,
    )?;
    print!("{code}");
    Ok(())
}
