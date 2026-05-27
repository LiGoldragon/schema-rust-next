use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../tests/fixtures/spirit-min.schema");
    let asschema =
        SchemaEngine::default().lower_source(source, SchemaIdentity::new("spirit:lib", "0.1.0"))?;
    print!("{}", RustEmitter::default().emit(&asschema).as_str());
    Ok(())
}
