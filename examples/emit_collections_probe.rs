use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

fn main() {
    let source = "\
{}
(Input ((Project Cluster)))
(Output ((Projected (KeyValueMap NodeName NodeConfig))))
{
  NodeName [Text]
  NodeConfig [Text]
  Service [Text]
  Cluster [(nodes (KeyValueMap NodeName NodeConfig)) (services (Vec Service)) (cache (Option Service))]
}";
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("probe:lib", "0.1.0"))
        .expect("lowers");
    let generated = RustEmitter::default().emit(&asschema);
    print!("{}", generated.as_str());
}
