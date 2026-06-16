use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::{RustEmissionOptions, RustEmissionTarget, RustEmitter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read_to_string(
        manifest.join("tests/fixtures/composition-demo/schema/configuration.schema"),
    )?;
    let schema = SchemaEngine::default()
        .lower_source(&source, SchemaIdentity::new("composition:demo", "0.1.0"))?;
    let options = RustEmissionOptions::feature_gated_nota("nota-text")
        .with_target(RustEmissionTarget::NexusRuntime);
    let code = RustEmitter::new(options)
        .emit_code_from_schema(&schema)
        .as_str()
        .to_owned();
    std::fs::write(
        manifest.join("tests/fixtures/composition_demo_generated.rs"),
        &code,
    )?;
    print!("{code}");
    Ok(())
}
