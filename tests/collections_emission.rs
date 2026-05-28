//! Collection emission (psyche record 1034).
//!
//! Proves the emitter turns collection `TypeReference`s into real
//! `Vec<T>` / `BTreeMap<K, V>` / `Option<T>` Rust, that the emitted
//! NOTA codec round-trips collection values, and that rkyv archives
//! the collection-bearing struct. The fixture is the actual emitter
//! output for a small collection-bearing schema; including it here
//! compiles it, so a regression in the emitted text fails the build.

use schema_next::{SchemaEngine, SchemaIdentity};
use schema_rust_next::RustEmitter;

#[allow(dead_code)]
mod generated {
    include!("fixtures/collections_generated.rs");
}

const COLLECTION_SCHEMA: &str = "\
{}
(Input ((Project Cluster)))
(Output ((Projected (KeyValueMap NodeName NodeConfig))))
{
  NodeName [Text]
  NodeConfig [Text]
  Service [Text]
  Cluster [(nodes (KeyValueMap NodeName NodeConfig)) (services (Vec Service)) (cache (Option Service))]
}";

#[test]
fn emitter_writes_collection_field_types() {
    let asschema = SchemaEngine::default()
        .lower_source(COLLECTION_SCHEMA, SchemaIdentity::new("probe:lib", "0.1.0"))
        .expect("schema lowers");
    let code = RustEmitter::default().emit(&asschema);
    let code = code.as_str();

    assert!(code.contains("pub nodes: std::collections::BTreeMap<NodeName, NodeConfig>,"));
    assert!(code.contains("pub services: Vec<Service>,"));
    assert!(code.contains("pub cache: Option<Service>,"));
    assert!(code.contains("Projected(std::collections::BTreeMap<NodeName, NodeConfig>),"));
    // The runtime codec block is present because the schema uses collections.
    assert!(code.contains("pub struct NotaCollection<'a>"));
    assert!(code.contains("pub fn parse_map<"));
}

#[test]
fn collection_free_schema_does_not_emit_the_collection_runtime() {
    // The gate: a schema with no collections emits NO NotaCollection
    // block, keeping legacy output byte-identical.
    let source = "{} (Input ((Mark Marker))) (Output ()) { Marker [Text] }";
    let asschema = SchemaEngine::default()
        .lower_source(source, SchemaIdentity::new("plain:lib", "0.1.0"))
        .expect("schema lowers");
    let code = RustEmitter::default().emit(&asschema);
    assert!(!code.as_str().contains("NotaCollection"));
}

#[test]
fn emitted_collection_struct_round_trips_through_nota() {
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(
        generated::NodeName(String::from("center")),
        generated::NodeConfig(String::from("config-center")),
    );
    nodes.insert(
        generated::NodeName(String::from("edge")),
        generated::NodeConfig(String::from("config-edge")),
    );
    let cluster = generated::Cluster {
        nodes,
        services: vec![
            generated::Service(String::from("dns")),
            generated::Service(String::from("vpn")),
        ],
        cache: Some(generated::Service(String::from("binary-cache"))),
    };

    let nota = cluster.to_nota();
    let parsed = generated::Cluster::from_nota_block(
        &generated::NotaSource::new(&nota)
            .parse_root()
            .expect("parse root"),
    )
    .expect("cluster decodes from its own nota");

    assert_eq!(parsed, cluster);
    // The map kept both keys in deterministic order.
    assert_eq!(parsed.nodes.len(), 2);
    assert_eq!(parsed.services.len(), 2);
    assert_eq!(
        parsed.cache,
        Some(generated::Service(String::from("binary-cache")))
    );
}

#[test]
fn emitted_option_none_round_trips() {
    let cluster = generated::Cluster {
        nodes: std::collections::BTreeMap::new(),
        services: Vec::new(),
        cache: None,
    };
    let nota = cluster.to_nota();
    assert!(nota.contains("None"));
    let parsed = generated::Cluster::from_nota_block(
        &generated::NotaSource::new(&nota)
            .parse_root()
            .expect("parse root"),
    )
    .expect("empty cluster decodes");
    assert_eq!(parsed.cache, None);
    assert!(parsed.nodes.is_empty());
    assert!(parsed.services.is_empty());
}

#[test]
fn emitted_collection_struct_archives_through_rkyv() {
    let mut nodes = std::collections::BTreeMap::new();
    nodes.insert(
        generated::NodeName(String::from("center")),
        generated::NodeConfig(String::from("config")),
    );
    let cluster = generated::Cluster {
        nodes,
        services: vec![generated::Service(String::from("dns"))],
        cache: None,
    };

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&cluster).expect("archive cluster");
    let decoded = rkyv::from_bytes::<generated::Cluster, rkyv::rancor::Error>(&bytes)
        .expect("decode cluster");
    assert_eq!(decoded, cluster);
}

#[test]
fn output_projection_variant_carries_a_map_payload_through_nota() {
    let mut configs = std::collections::BTreeMap::new();
    configs.insert(
        generated::NodeName(String::from("center")),
        generated::NodeConfig(String::from("resolved")),
    );
    let output = generated::Output::Projected(configs);
    let nota = output.to_nota();
    let parsed: generated::Output = nota.parse().expect("output round-trips");
    assert_eq!(parsed, output);
}
