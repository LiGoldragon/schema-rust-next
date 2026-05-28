use schema_next::{Name, SchemaEngine, SchemaPackage};
use schema_rust_next::RustEmitter;

#[allow(dead_code)]
mod schema {
    pub mod proposal {
        include!("fixtures/horizon-concept/generated/src/schema/proposal.rs");
    }

    pub mod view {
        include!("fixtures/horizon-concept/generated/src/schema/view.rs");
    }

    pub mod lib {
        include!("fixtures/horizon-concept/generated/src/schema/lib.rs");
    }
}

#[test]
fn horizon_concept_loads_modules_lowers_imports_and_emits_rust_paths() {
    let package = SchemaPackage::new(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("concept")
            .join("horizon"),
        "horizon-concept",
        "0.1.0",
    );
    let engine = SchemaEngine::default();
    let lib = package.lower_lib(&engine).expect("lib schema lowers");
    let proposal = package
        .load_module(Name::new("proposal"))
        .expect("proposal schema loads")
        .lower(&engine)
        .expect("proposal schema lowers");
    let view = package
        .load_module(Name::new("view"))
        .expect("view schema loads")
        .lower(&engine)
        .expect("view schema lowers");

    assert_eq!(
        proposal.identity().component().as_str(),
        "horizon-concept:proposal"
    );
    assert_eq!(view.identity().component().as_str(), "horizon-concept:view");
    assert_eq!(
        lib.imports()
            .iter()
            .map(|import| (import.local_name.as_str(), import.source.name.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("Proposal", "horizon-concept:proposal:ClusterProposal"),
            ("View", "horizon-concept:view:Horizon"),
        ]
    );

    let emitted = RustEmitter::default().emit_file(&lib);
    assert_eq!(emitted.path, "src/schema/lib.rs");
    assert!(
        emitted
            .code
            .as_str()
            .contains("pub use crate::schema::proposal::ClusterProposal as Proposal;")
    );
    assert!(
        emitted
            .code
            .as_str()
            .contains("impl From<crate::schema::view::NotaDecodeError> for NotaDecodeError")
    );
}

#[test]
fn horizon_concept_generated_types_parse_project_signal_and_preserve_payloads() {
    let input = "(Project (([goldragon] ([ouranos] Workstation PersonaDevelopment) ([prometheus] Router NixCache)) ([goldragon] [ouranos])))"
        .parse::<schema::lib::Input>()
        .expect("generated project signal parses");

    let schema::lib::Input::Project(request) = &input;
    assert_eq!(request.proposal.cluster_name.0, "goldragon");
    assert_eq!(
        request.proposal.workstation.0.major_node_kind,
        schema::proposal::MajorNodeKind::Workstation
    );
    assert_eq!(
        request.proposal.router.0.node_feature,
        schema::proposal::NodeFeature::NixCache
    );
    assert_eq!(request.viewpoint.node_name.0, "ouranos");
    assert_eq!(input.route(), schema::lib::InputRoute::Project);
    assert_eq!(
        input.to_string(),
        "(Project (([goldragon] ([ouranos] Workstation PersonaDevelopment) ([prometheus] Router NixCache)) ([goldragon] [ouranos])))"
    );

    let frame = input
        .encode_signal_frame()
        .expect("generated signal frame encodes");
    let (route, decoded) =
        schema::lib::Input::decode_signal_frame(&frame).expect("generated signal frame decodes");

    assert_eq!(route, schema::lib::InputRoute::Project);
    assert_eq!(decoded, input);
}

#[test]
fn horizon_concept_generated_view_output_crosses_the_import_boundary() {
    let output = "(Projected ([goldragon] ([ouranos] [ouranos.goldragon.criome] [ouranos.goldragon.criome]) ([prometheus] [prometheus.goldragon.criome] [prometheus.goldragon.criome])))"
        .parse::<schema::lib::Output>()
        .expect("generated projected output parses");

    let schema::lib::Output::Projected(result) = &output else {
        panic!("expected projected output");
    };
    assert_eq!(result.0.cluster_name.0, "goldragon");
    assert_eq!(result.0.workstation_view.0.node_name.0, "ouranos");
    assert_eq!(
        result.0.router_view.0.domain_name.0,
        "prometheus.goldragon.criome"
    );
    assert_eq!(
        output.to_string(),
        "(Projected ([goldragon] ([ouranos] [ouranos.goldragon.criome] [ouranos.goldragon.criome]) ([prometheus] [prometheus.goldragon.criome] [prometheus.goldragon.criome])))"
    );
}
