{
  description = "schema-rust-next — Rust source emitter for assembled schemas";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, fenix, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        toolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "rustc"
          "rustfmt"
          "clippy"
          "rust-src"
        ];
        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        schemaFilter = path: type:
          type == "regular" && (
            pkgs.lib.hasSuffix ".schema" path
            || pkgs.lib.hasSuffix ".asschema" path
            || pkgs.lib.hasSuffix ".witness.txt" path
          );
        sourceFilter = path: type:
          (craneLib.filterCargoSources path type) || (schemaFilter path type);
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = sourceFilter;
          name = "source";
        };
        cargoVendorDirectory = craneLib.vendorCargoDeps { inherit src; };
        commonArguments = {
          inherit src cargoVendorDirectory;
          strictDeps = true;
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArguments;
      in
      {
        packages.default = craneLib.buildPackage (commonArguments // { inherit cargoArtifacts; });
        checks = {
          build = craneLib.cargoBuild (commonArguments // { inherit cargoArtifacts; });
          test = craneLib.cargoTest (commonArguments // { inherit cargoArtifacts; });
          no-old-signal-macro = pkgs.runCommand "schema-rust-next-no-old-signal-macro" { } ''
            if grep -R "signal_channel!" ${src}; then
              echo "schema-rust-next must not use the old signal_channel macro" >&2
              exit 1
            fi
            touch $out
          '';
          no-rust-macro-surface = pkgs.runCommand "schema-rust-next-no-rust-macro-surface" { } ''
            if grep -R "macro_rules!\\|proc_macro" ${src}/src; then
              echo "Rust emission must stay separate from Rust macros in src/" >&2
              exit 1
            fi
            touch $out
          '';
          generated-rkyv-boundary = pkgs.runCommand "schema-rust-next-generated-rkyv-boundary" { } ''
            grep -R "encode_signal_frame" ${src}/tests/emission.rs >/dev/null
            grep -R "decode_signal_frame" ${src}/tests/emission.rs >/dev/null
            grep -R "rkyv::Archive" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub enum InputRoute" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub enum OutputRoute" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub fn encode_signal_frame" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            touch $out
          '';
          generated-nexus-traits = pkgs.runCommand "schema-rust-next-generated-nexus-traits" { } ''
            grep -R "pub trait InputNexus" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub fn dispatch_mail_with_nexus<NexusActor>" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "generated::InputNexus for SpiritNexus" ${src}/tests/emission.rs >/dev/null
            grep -R "input dispatches through generated nexus trait" ${src}/tests/emission.rs >/dev/null
            touch $out
          '';
          generated-mail-events = pkgs.runCommand "schema-rust-next-generated-mail-events" { } ''
            grep -R "pub struct MessageIdentifier(pub Integer)" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub struct OriginRoute(pub Integer)" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub mod schema" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub enum Plane" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "Sema(super::Sema<SemaRoot>)" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub mod signal" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "impl OriginRoute" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub enum MessageRoot" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub struct MessageSent" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub origin_route: OriginRoute" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub short_header: Integer" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub struct NexusMail<Payload>" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub struct MessageProcessed<Reply>" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub trait MessageSentHook" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub trait MessageProcessedHook<Reply>" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "fn generated_signal_roots_emit_typed_message_sent_events" ${src}/tests/emission.rs >/dev/null
            grep -R "event.push_to" ${src}/tests/emission.rs >/dev/null
            touch $out
          '';
          generated-upgrade-traits = pkgs.runCommand "schema-rust-next-generated-upgrade-traits" { } ''
            grep -R "pub trait UpgradeFrom<Previous>" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub trait AcceptPrevious<Previous>" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "UpgradeFrom<PreviousEntry> for generated::Entry" ${src}/tests/emission.rs >/dev/null
            grep -R "accepted previous Entry as" ${src}/tests/emission.rs >/dev/null
            touch $out
          '';
          generated-nota-boundary = pkgs.runCommand "schema-rust-next-generated-nota-boundary" { } ''
            grep -R "parse::<generated::Input>" ${src}/tests/emission.rs >/dev/null
            grep -R "impl std::str::FromStr for Input" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            grep -R "pub fn to_nota" ${src}/tests/fixtures/spirit_generated.rs >/dev/null
            touch $out
          '';
          generated-schema-module-path = pkgs.runCommand "schema-rust-next-generated-schema-module-path" { } ''
            grep -R "src/schema/lib.rs" ${src}/tests/emission.rs >/dev/null
            grep -R "src/schema/signal/public.rs" ${src}/tests/emission.rs >/dev/null
            grep -R "struct RustModulePath" ${src}/src/lib.rs >/dev/null
            touch $out
          '';
          generated-cross-crate-imports = pkgs.runCommand "schema-rust-next-generated-cross-crate-imports" { } ''
            grep -R "imported_type_is_referenced_through_a_use_not_redeclared" ${src}/tests/cross_crate_import.rs >/dev/null
            grep -R "pub use marker_core::schema::mail::DatabaseMarker as DatabaseMarker" ${src}/tests/cross_crate_import.rs >/dev/null
            grep -R "Marked(DatabaseMarker)" ${src}/tests/cross_crate_import.rs >/dev/null
            touch $out
          '';
          asschema-fixtures-final-data = pkgs.runCommand "schema-rust-next-asschema-fixtures-final-data" { } ''
            grep -R "assert_asschema_is_final_data" ${src}/tests/big_emission.rs >/dev/null
            if grep -R -n --include='*.asschema' '@' ${src}/tests/fixtures; then
              echo ".asschema fixtures must not contain authored macro markers" >&2
              exit 1
            fi
            if grep -R -n --include='*.asschema' '\$' ${src}/tests/fixtures; then
              echo ".asschema fixtures must not contain macro captures" >&2
              exit 1
            fi
            if grep -R -n --include='*.asschema' -E '\(Map \(Plain' ${src}/tests/fixtures; then
              echo ".asschema Map must carry one vector payload" >&2
              exit 1
            fi
            touch $out
          '';
          no-nested-root-enum-examples = pkgs.runCommand "schema-rust-next-no-nested-root-enum-examples" { } ''
            if grep -R -n -E '\((Input|Output) \(\(' ${src}/tests; then
              echo "root Input/Output examples must use direct variants, not nested enum bodies" >&2
              exit 1
            fi
            if grep -R -n -E '@(Vec|Option|KeyValue|Bag|HashSet)' ${src}/tests ${src}/src; then
              echo "schema-rust-next examples must use no-sigil tagged macro invocation" >&2
              exit 1
            fi
            touch $out
          '';
          no-production-free-functions = pkgs.runCommand "schema-rust-next-no-production-free-functions" { } ''
            if grep -R -n -E '^(pub(\([^)]*\))? )?fn ' ${src}/src; then
              echo "production Rust must not use module-level free functions" >&2
              exit 1
            fi
            touch $out
          '';
          no-production-unit-structs = pkgs.runCommand "schema-rust-next-no-production-unit-structs" { } ''
            if grep -R -n -E '^struct [A-Za-z][A-Za-z0-9_]*;' ${src}/src; then
              echo "production Rust must not use unit structs as namespace/method holders" >&2
              exit 1
            fi
            touch $out
          '';
          generated-no-free-functions = pkgs.runCommand "schema-rust-next-generated-no-free-functions" { } ''
            if grep -R -n --include='*.generated.rs' -E '^(pub(\([^)]*\))? )?fn ' ${src}/tests/fixtures; then
              echo "generated Rust fixture must not use module-level free functions" >&2
              exit 1
            fi
            touch $out
          '';
          generated-no-legacy-helper-surface = pkgs.runCommand "schema-rust-next-generated-no-legacy-helper-surface" { } ''
            ! grep -R --include='*.generated.rs' "parse_nota_root" ${src}/tests/fixtures
            ! grep -R --include='*.generated.rs' "UnknownHeader { surface" ${src}/tests/fixtures
            ! grep -R "pub struct RustEmitter;" ${src}/src
            grep -R --include='*.generated.rs' "pub struct NotaSource" ${src}/tests/fixtures >/dev/null
            grep -R --include='*.generated.rs' "pub struct NotaBlock" ${src}/tests/fixtures >/dev/null
            touch $out
          '';
          doc = craneLib.cargoDoc (commonArguments // {
            inherit cargoArtifacts;
            RUSTDOCFLAGS = "-D warnings";
          });
          fmt = craneLib.cargoFmt { inherit src; };
          clippy = craneLib.cargoClippy (commonArguments // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });
        };
        devShells.default = pkgs.mkShell {
          name = "schema-rust-next";
          packages = [ pkgs.jujutsu pkgs.pkg-config toolchain ];
        };
      });
}
