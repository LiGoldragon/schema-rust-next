use std::{env, fs};

use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    let schema_path = arguments
        .next()
        .ok_or("usage: emit_schema <schema-path> <schema-identity> [version]")?;
    let identity = arguments
        .next()
        .ok_or("usage: emit_schema <schema-path> <schema-identity> [version]")?;
    let version = arguments.next().unwrap_or_else(|| "0.1.0".to_owned());

    let source = fs::read_to_string(schema_path)?;
    let asschema =
        SchemaEngine::default().lower_source(&source, SchemaIdentity::new(identity, version))?;
    print!("{}", RustEmitter::default().emit(&asschema).as_str());
    Ok(())
}
