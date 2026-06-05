use schema_next::{
    AliasDeclaration, Declaration, EnumDeclaration, EnumVariant, FieldDeclaration, ImportResolver,
    Name, NewtypeDeclaration, ResolvedImport, Schema, SchemaEngine, SchemaError, SchemaIdentity,
    SchemaSource, StructDeclaration, TypeDeclaration, TypeReference, Visibility,
};

pub mod build;
pub mod migration;
pub use migration::{DefaultRenderer, MigrationEmitter, TypeRenderer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedFile {
    pub path: String,
    pub code: RustCode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustCode(String);

impl RustCode {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct RustEmitter {
    generator_name: &'static str,
    options: RustEmissionOptions,
}

impl Default for RustEmitter {
    fn default() -> Self {
        Self {
            generator_name: "schema-rust-next",
            options: RustEmissionOptions::default(),
        }
    }
}

impl RustEmitter {
    pub fn new(options: RustEmissionOptions) -> Self {
        Self {
            generator_name: "schema-rust-next",
            options,
        }
    }

    pub fn emit_file_from_schema(&self, schema: &Schema) -> GeneratedFile {
        schema.lower_to_rust_file(self)
    }

    pub fn emit_file_from_schema_source(
        &self,
        source: &SchemaSource,
        identity: SchemaIdentity,
        engine: &SchemaEngine,
        resolver: &ImportResolver,
    ) -> Result<GeneratedFile, SchemaError> {
        source.lower_to_rust_file(identity, engine, resolver, self)
    }

    pub fn emit_code_from_schema(&self, schema: &Schema) -> RustCode {
        schema.lower_to_rust_code(self)
    }

    pub fn emit_module_from_schema(&self, schema: &Schema) -> RustModule {
        schema.lower_to_rust_module(self)
    }

    pub fn emit_module_from_schema_source(
        &self,
        source: &SchemaSource,
        identity: SchemaIdentity,
        engine: &SchemaEngine,
        resolver: &ImportResolver,
    ) -> Result<RustModule, SchemaError> {
        source.lower_to_rust_module(identity, engine, resolver, self)
    }
}

pub trait RustSchemaLowering {
    fn lower_to_rust_file(&self, emitter: &RustEmitter) -> GeneratedFile;
    fn lower_to_rust_code(&self, emitter: &RustEmitter) -> RustCode;
    fn lower_to_rust_module(&self, emitter: &RustEmitter) -> RustModule;
}

impl RustSchemaLowering for Schema {
    fn lower_to_rust_file(&self, emitter: &RustEmitter) -> GeneratedFile {
        let module = self.lower_to_rust_module(emitter);
        GeneratedFile {
            path: module.file_path().to_owned(),
            code: module.render(),
        }
    }

    fn lower_to_rust_code(&self, emitter: &RustEmitter) -> RustCode {
        self.lower_to_rust_module(emitter).render()
    }

    fn lower_to_rust_module(&self, emitter: &RustEmitter) -> RustModule {
        RustModule::from_schema(self, emitter.generator_name, emitter.options.clone())
    }
}

pub trait RustSchemaSourceLowering {
    fn lower_to_rust_file(
        &self,
        identity: SchemaIdentity,
        engine: &SchemaEngine,
        resolver: &ImportResolver,
        emitter: &RustEmitter,
    ) -> Result<GeneratedFile, SchemaError>;

    fn lower_to_rust_module(
        &self,
        identity: SchemaIdentity,
        engine: &SchemaEngine,
        resolver: &ImportResolver,
        emitter: &RustEmitter,
    ) -> Result<RustModule, SchemaError>;
}

impl RustSchemaSourceLowering for SchemaSource {
    fn lower_to_rust_file(
        &self,
        identity: SchemaIdentity,
        engine: &SchemaEngine,
        resolver: &ImportResolver,
        emitter: &RustEmitter,
    ) -> Result<GeneratedFile, SchemaError> {
        let module = self.lower_to_rust_module(identity, engine, resolver, emitter)?;
        Ok(GeneratedFile {
            path: module.file_path().to_owned(),
            code: module.render(),
        })
    }

    fn lower_to_rust_module(
        &self,
        identity: SchemaIdentity,
        engine: &SchemaEngine,
        resolver: &ImportResolver,
        emitter: &RustEmitter,
    ) -> Result<RustModule, SchemaError> {
        let schema = engine.lower_schema_source_with_resolver(self, identity, resolver)?;
        Ok(schema.lower_to_rust_module(emitter))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustModule {
    file_path: String,
    generator_name: String,
    scalar_aliases: Vec<RustScalarAlias>,
    imports: Vec<RustImport>,
    declarations: Vec<RustDeclaration>,
    root_enums: Vec<RustEnum>,
    support: RustSupportModel,
    options: RustEmissionOptions,
}

impl RustModule {
    pub fn from_schema(
        schema: &Schema,
        generator_name: impl Into<String>,
        options: RustEmissionOptions,
    ) -> Self {
        let declarations = schema
            .namespace()
            .iter()
            .map(RustDeclaration::from_schema_declaration)
            .collect::<Vec<_>>();
        let root_enums = schema
            .input_and_output()
            .into_iter()
            .map(RustEnum::from_schema_enum)
            .collect::<Vec<_>>();
        Self {
            file_path: RustModulePath::new(schema.identity().component().clone()).to_file_path(),
            generator_name: generator_name.into(),
            scalar_aliases: RustScalarAlias::default_aliases(),
            imports: schema
                .resolved_imports()
                .iter()
                .map(RustImport::from_resolved_import)
                .collect(),
            declarations,
            root_enums,
            support: RustSupportModel::from_schema(schema),
            options,
        }
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn declarations(&self) -> &[RustDeclaration] {
        &self.declarations
    }

    pub fn scalar_aliases(&self) -> &[RustScalarAlias] {
        &self.scalar_aliases
    }

    pub fn imports(&self) -> &[RustImport] {
        &self.imports
    }

    pub fn root_enums(&self) -> &[RustEnum] {
        &self.root_enums
    }

    pub fn declaration_named(&self, name: &str) -> Option<&RustDeclaration> {
        self.declarations
            .iter()
            .find(|declaration| declaration.name().as_str() == name)
    }

    pub fn render(&self) -> RustCode {
        let mut writer = RustWriter::new(self.options.clone());
        writer.note_map_key_types(self.support.map_key_type_names().to_vec());
        writer.note_private_type_names(self.support.private_type_names().to_vec());
        writer.line(format!("// @generated by {}", self.generator_name));
        writer.blank();
        for alias in &self.scalar_aliases {
            writer.emit_scalar_alias(alias);
        }
        writer.blank();
        writer.emit_imports(&self.imports);
        writer.emit_nota_support();
        if writer.nota_surface().emits_nota() {
            writer.blank();
        }

        for declaration in &self.declarations {
            writer.emit_type(declaration);
            writer.blank();
        }

        for root_enum in &self.root_enums {
            writer.emit_root_enum(root_enum);
            writer.blank();
        }

        writer.emit_newtype_inherent_impls(&self.declarations);
        writer.emit_enum_variant_constructors(&self.declarations, &self.root_enums);
        writer.emit_enum_payload_from_impls(&self.declarations, &self.root_enums);
        writer.emit_nota_type_bridges(&self.declarations);
        for root_enum in &self.root_enums {
            writer.emit_nota_root_enum_support(root_enum);
            writer.blank();
        }

        writer.emit_short_headers(&self.root_enums);
        writer.blank();
        writer.emit_signal_frame_support(&self.root_enums);
        if writer.emits_runtime_support() {
            writer.emit_plane_route_support(&self.declarations);
            writer.emit_trace_support(&self.declarations, &self.root_enums);
            writer.emit_mail_event_support(&self.root_enums);
            writer.emit_plane_namespaces(&self.declarations, &self.root_enums);
            writer.emit_plane_projection_support(&self.declarations, &self.root_enums);
            writer.emit_schema_plane_trait_support(&self.declarations, &self.root_enums);
            writer.emit_upgrade_support();
        }
        RustCode(writer.finish())
    }
}

/// The emission knobs passed to [`RustEmitter::new`].
///
/// The default is [`NotaSurface::FeatureGated`] with feature name
/// `"nota-text"`. That is the recommended shape per the codec opt-in
/// design: rkyv is the universal base, and NOTA encode/decode are an
/// opt-in surface that text-facing clients (CLIs, launchers, REPLs)
/// enable through a cargo feature. Binary-only consumers (daemons,
/// future binary-only clients) build the contract crate with the
/// default features off and carry no `nota-next` in their dependency
/// closure. The default target is [`RustEmissionTarget::ComponentRuntime`]
/// so existing all-in-one runtime consumers keep their generated engine traits.
/// New signal and meta-signal contract repos should opt into
/// [`RustEmissionTarget::WireContract`]. Daemon-local signal runtime schemas
/// should opt into [`RustEmissionTarget::SignalRuntime`]. New daemon decision
/// and storage plane schemas should opt into [`RustEmissionTarget::NexusRuntime`]
/// or [`RustEmissionTarget::SemaRuntime`] for per-plane runtime emission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustEmissionOptions {
    pub nota_surface: NotaSurface,
    pub target: RustEmissionTarget,
}

impl Default for RustEmissionOptions {
    fn default() -> Self {
        Self::feature_gated_nota("nota-text")
    }
}

impl RustEmissionOptions {
    /// Always emit `nota_next::NotaDecode` / `nota_next::NotaEncode`
    /// derives, the inherent `from_nota_block` / `to_nota` bridges, the
    /// root `FromStr` / `Display` impls, and the `use nota_next::*`
    /// pull-in — without any cargo-feature gate.
    pub fn always_enabled_nota() -> Self {
        Self {
            nota_surface: NotaSurface::AlwaysEnabled,
            target: RustEmissionTarget::ComponentRuntime,
        }
    }

    /// Emit the NOTA surface guarded by `#[cfg_attr(feature = "<feature>",
    /// derive(...))]` on data types and `#[cfg(feature = "<feature>")]`
    /// on the inherent bridges, FromStr/Display impls, and the
    /// `use nota_next::*` items. Consumers enable the feature only in
    /// text-facing crates (CLI, launcher) and leave it off in
    /// daemon-only crates so `nota-next` stays out of the binary-only
    /// dependency closure.
    pub fn feature_gated_nota(feature: impl Into<String>) -> Self {
        Self {
            nota_surface: NotaSurface::FeatureGated {
                feature: feature.into(),
            },
            target: RustEmissionTarget::ComponentRuntime,
        }
    }

    /// Emit no NOTA surface at all. The generated source contains no
    /// `nota_next::*` references, no `FromStr` / `Display` impls
    /// (since both depend on `NotaDecode` / `NotaEncode`), and no
    /// inherent `from_nota_block` / `to_nota` bridge methods. The
    /// resulting Rust file compiles without `nota-next` in the
    /// dependency closure. This is the daemon-only / binary-only
    /// shape.
    pub fn binary_only() -> Self {
        Self {
            nota_surface: NotaSurface::Disabled,
            target: RustEmissionTarget::ComponentRuntime,
        }
    }

    pub fn with_target(mut self, target: RustEmissionTarget) -> Self {
        self.target = target;
        self
    }

    fn nota_surface(&self) -> &NotaSurface {
        &self.nota_surface
    }

    pub fn target(&self) -> RustEmissionTarget {
        self.target
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustEmissionTarget {
    /// External signal or meta-signal wire vocabulary plus codecs only.
    WireContract,
    /// Bootstrap all-in-one runtime emission for unsplit schemas.
    ComponentRuntime,
    /// Daemon-side Signal plane runtime support over signal roots.
    SignalRuntime,
    /// Daemon-side Nexus plane runtime support only.
    NexusRuntime,
    /// Daemon-side SEMA plane runtime support only.
    SemaRuntime,
}

impl RustEmissionTarget {
    fn emits_runtime_support(self) -> bool {
        self.runtime_planes().emits_any()
    }

    fn runtime_planes(self) -> RuntimePlaneSet {
        match self {
            Self::WireContract => RuntimePlaneSet::none(),
            Self::ComponentRuntime => RuntimePlaneSet::all(),
            Self::SignalRuntime => RuntimePlaneSet::signal_only(),
            Self::NexusRuntime => RuntimePlaneSet::nexus_only(),
            Self::SemaRuntime => RuntimePlaneSet::sema_only(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RuntimePlaneSet {
    signal: bool,
    nexus: bool,
    sema: bool,
}

impl RuntimePlaneSet {
    fn none() -> Self {
        Self {
            signal: false,
            nexus: false,
            sema: false,
        }
    }

    fn all() -> Self {
        Self {
            signal: true,
            nexus: true,
            sema: true,
        }
    }

    fn signal_only() -> Self {
        Self {
            signal: true,
            nexus: false,
            sema: false,
        }
    }

    fn nexus_only() -> Self {
        Self {
            signal: false,
            nexus: true,
            sema: false,
        }
    }

    fn sema_only() -> Self {
        Self {
            signal: false,
            nexus: false,
            sema: true,
        }
    }

    fn emits_signal(self) -> bool {
        self.signal
    }

    fn emits_nexus(self) -> bool {
        self.nexus
    }

    fn emits_sema(self) -> bool {
        self.sema
    }

    fn emits_any(self) -> bool {
        self.signal || self.nexus || self.sema
    }

    fn emits_all(self) -> bool {
        self.signal && self.nexus && self.sema
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotaSurface {
    AlwaysEnabled,
    FeatureGated { feature: String },
    Disabled,
}

impl NotaSurface {
    fn emits_nota(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    fn feature_gate_attribute(&self) -> Option<String> {
        match self {
            Self::AlwaysEnabled | Self::Disabled => None,
            Self::FeatureGated { feature } => Some(format!("#[cfg(feature = \"{feature}\")]")),
        }
    }

    fn feature_gated_derive_attribute(&self) -> Option<String> {
        match self {
            Self::AlwaysEnabled | Self::Disabled => None,
            Self::FeatureGated { feature } => Some(format!(
                "#[cfg_attr(feature = \"{feature}\", derive(nota_next::NotaDecode, nota_next::NotaEncode))]"
            )),
        }
    }

    fn includes_nota_in_derive(&self) -> bool {
        matches!(self, Self::AlwaysEnabled)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustScalarAlias {
    name: String,
    rust_type: String,
}

impl RustScalarAlias {
    fn new(name: impl Into<String>, rust_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rust_type: rust_type.into(),
        }
    }

    fn default_aliases() -> Vec<Self> {
        vec![
            Self::new("String", "std::string::String"),
            Self::new("Integer", "u64"),
            Self::new("Boolean", "bool"),
            Self::new("Path", "std::string::String"),
        ]
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rust_type(&self) -> &str {
        &self.rust_type
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustImport {
    use_item: String,
}

impl RustImport {
    fn from_resolved_import(import: &ResolvedImport) -> Self {
        Self {
            use_item: import.use_item(),
        }
    }

    pub fn use_item(&self) -> &str {
        &self.use_item
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustDeclaration {
    visibility: Visibility,
    name: Name,
    value: RustTypeDeclaration,
}

impl RustDeclaration {
    fn from_schema_declaration(declaration: &Declaration) -> Self {
        Self {
            visibility: declaration.visibility(),
            name: declaration.name().clone(),
            value: RustTypeDeclaration::from_schema_type(declaration.value()),
        }
    }

    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn value(&self) -> &RustTypeDeclaration {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustTypeDeclaration {
    Alias(RustAlias),
    Struct(RustStruct),
    Enum(RustEnum),
    Newtype(RustNewtype),
}

impl RustTypeDeclaration {
    fn from_schema_type(declaration: &TypeDeclaration) -> Self {
        match declaration {
            TypeDeclaration::Alias(declaration) => {
                Self::Alias(RustAlias::from_schema_alias(declaration))
            }
            TypeDeclaration::Struct(declaration) => {
                Self::Struct(RustStruct::from_schema_struct(declaration))
            }
            TypeDeclaration::Enum(declaration) => {
                Self::Enum(RustEnum::from_schema_enum(declaration))
            }
            TypeDeclaration::Newtype(declaration) => {
                Self::Newtype(RustNewtype::from_schema_newtype(declaration))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustAlias {
    name: Name,
    reference: TypeReference,
}

impl RustAlias {
    fn from_schema_alias(declaration: &AliasDeclaration) -> Self {
        Self {
            name: declaration.name.clone(),
            reference: declaration.reference.clone(),
        }
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn reference(&self) -> &TypeReference {
        &self.reference
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustNewtype {
    name: Name,
    reference: TypeReference,
}

impl RustNewtype {
    fn from_schema_newtype(declaration: &NewtypeDeclaration) -> Self {
        Self {
            name: declaration.name.clone(),
            reference: declaration.reference.clone(),
        }
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn reference(&self) -> &TypeReference {
        &self.reference
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustStruct {
    name: Name,
    fields: Vec<RustField>,
}

impl RustStruct {
    fn from_schema_struct(declaration: &StructDeclaration) -> Self {
        Self {
            name: declaration.name.clone(),
            fields: declaration
                .fields
                .iter()
                .map(RustField::from_schema_field)
                .collect(),
        }
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn fields(&self) -> &[RustField] {
        &self.fields
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustField {
    name: Name,
    reference: TypeReference,
}

impl RustField {
    fn from_schema_field(field: &FieldDeclaration) -> Self {
        Self {
            name: field.name.clone(),
            reference: field.reference.clone(),
        }
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn reference(&self) -> &TypeReference {
        &self.reference
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustEnum {
    name: Name,
    variants: Vec<RustEnumVariant>,
}

impl RustEnum {
    fn from_schema_enum(declaration: &EnumDeclaration) -> Self {
        Self {
            name: declaration.name.clone(),
            variants: declaration
                .variants
                .iter()
                .map(RustEnumVariant::from_schema_variant)
                .collect(),
        }
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn variants(&self) -> &[RustEnumVariant] {
        &self.variants
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustEnumVariant {
    name: Name,
    payload: Option<TypeReference>,
}

impl RustEnumVariant {
    fn from_schema_variant(variant: &EnumVariant) -> Self {
        Self {
            name: variant.name.clone(),
            payload: variant.payload.clone(),
        }
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn payload(&self) -> Option<&TypeReference> {
        self.payload.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RustSupportModel {
    map_key_type_names: Vec<String>,
    private_type_names: Vec<String>,
}

impl RustSupportModel {
    fn from_schema(schema: &Schema) -> Self {
        Self {
            map_key_type_names: CollectionScan::new(schema).map_key_type_names(),
            private_type_names: schema
                .namespace()
                .iter()
                .filter(|declaration| declaration.is_private())
                .map(|declaration| declaration.name().as_str().to_owned())
                .collect(),
        }
    }

    fn map_key_type_names(&self) -> &[String] {
        &self.map_key_type_names
    }

    fn private_type_names(&self) -> &[String] {
        &self.private_type_names
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RustModulePath {
    schema_name: Name,
}

impl RustModulePath {
    fn new(schema_name: Name) -> Self {
        Self { schema_name }
    }

    fn to_file_path(&self) -> String {
        let mut segments = self.module_segments();
        let file = segments.pop().unwrap_or_else(|| "lib".to_owned());
        if segments.is_empty() {
            format!("src/schema/{file}.rs")
        } else {
            format!("src/schema/{}/{}.rs", segments.join("/"), file)
        }
    }

    fn module_segments(&self) -> Vec<String> {
        let segments = self.schema_name.namespace_segments();
        let module_segments = if segments.len() > 1 {
            &segments[1..]
        } else {
            segments.as_slice()
        };
        module_segments
            .iter()
            .map(|segment| Name::new(*segment).field_name())
            .collect()
    }
}

/// Decides which assembled schema type names appear as map keys.
#[derive(Clone, Copy, Debug)]
struct CollectionScan<'schema> {
    schema: &'schema Schema,
}

impl<'schema> CollectionScan<'schema> {
    fn new(schema: &'schema Schema) -> Self {
        Self { schema }
    }

    /// The plain type names that appear as a `BTreeMap` key anywhere in
    /// the schema (field references, variant payloads, and nested
    /// collection positions). These types need the ordering derives.
    fn map_key_type_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for declaration in self.schema.namespace() {
            match declaration.value() {
                TypeDeclaration::Alias(declaration) => {
                    Self::collect_alias_map_keys(declaration, &mut names);
                }
                TypeDeclaration::Struct(declaration) => {
                    Self::collect_declaration_map_keys(declaration, &mut names);
                }
                TypeDeclaration::Newtype(declaration) => {
                    Self::collect_newtype_map_keys(declaration, &mut names);
                }
                TypeDeclaration::Enum(declaration) => {
                    Self::collect_enum_map_keys(declaration, &mut names);
                }
            }
        }
        for root in self.schema.input_and_output() {
            Self::collect_enum_map_keys(root, &mut names);
        }
        names
    }

    fn collect_alias_map_keys(declaration: &AliasDeclaration, names: &mut Vec<String>) {
        Self::collect_map_keys(&declaration.reference, names);
    }

    fn collect_enum_map_keys(declaration: &EnumDeclaration, names: &mut Vec<String>) {
        for variant in &declaration.variants {
            if let Some(payload) = &variant.payload {
                Self::collect_map_keys(payload, names);
            }
        }
    }

    fn collect_declaration_map_keys(declaration: &StructDeclaration, names: &mut Vec<String>) {
        for field in &declaration.fields {
            Self::collect_map_keys(&field.reference, names);
        }
    }

    fn collect_newtype_map_keys(declaration: &NewtypeDeclaration, names: &mut Vec<String>) {
        Self::collect_map_keys(&declaration.reference, names);
    }

    fn collect_map_keys(reference: &TypeReference, names: &mut Vec<String>) {
        match reference {
            TypeReference::String
            | TypeReference::Integer
            | TypeReference::Boolean
            | TypeReference::Path
            | TypeReference::Plain(_) => {}
            TypeReference::Vector(inner) | TypeReference::Optional(inner) => {
                Self::collect_map_keys(inner, names);
            }
            TypeReference::Map(key, value) => {
                if let TypeReference::Plain(name) = key.as_ref() {
                    let name = name.as_str().to_owned();
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                Self::collect_map_keys(key, names);
                Self::collect_map_keys(value, names);
            }
        }
    }
}

struct RustWriter {
    output: String,
    map_key_types: Vec<String>,
    private_type_names: Vec<String>,
    nota_surface: NotaSurface,
    target: RustEmissionTarget,
}

struct EnumConstructorPayload {
    argument_type: String,
    expression: String,
}

impl EnumConstructorPayload {
    fn new(argument_type: String, expression: String) -> Self {
        Self {
            argument_type,
            expression,
        }
    }

    fn argument_type(&self) -> &str {
        &self.argument_type
    }

    fn expression(&self) -> &str {
        &self.expression
    }
}

struct SplitSemaProjection<'schema> {
    signal_input: &'schema RustEnum,
    signal_output: &'schema RustEnum,
    nexus_work: &'schema RustEnum,
    nexus_action: &'schema RustEnum,
    sema_write_input: &'schema RustEnum,
    sema_write_output: &'schema RustEnum,
    sema_read_input: &'schema RustEnum,
    sema_read_output: &'schema RustEnum,
}

struct NexusRunnerShape {
    reply_type: String,
    sema_write_input_type: Option<String>,
    sema_write_output_type: Option<String>,
    sema_read_input_type: Option<String>,
    sema_read_output_type: Option<String>,
    effect_command_type: Option<String>,
    effect_result_type: Option<String>,
    has_continue: bool,
}

impl NexusRunnerShape {
    fn sema_write_input_type(&self) -> &str {
        self.sema_write_input_type
            .as_deref()
            .unwrap_or("std::convert::Infallible")
    }

    fn sema_read_input_type(&self) -> &str {
        self.sema_read_input_type
            .as_deref()
            .unwrap_or("std::convert::Infallible")
    }

    fn effect_command_type(&self) -> &str {
        self.effect_command_type
            .as_deref()
            .unwrap_or("std::convert::Infallible")
    }

    fn emits_sema_write(&self) -> bool {
        self.sema_write_input_type.is_some()
    }

    fn emits_sema_read(&self) -> bool {
        self.sema_read_input_type.is_some()
    }

    fn emits_effect(&self) -> bool {
        self.effect_command_type.is_some()
    }
}

struct TraceInterfaceRoot<'schema> {
    object_variant: &'static str,
    name_prefix: &'static str,
    type_name: &'schema Name,
    enum_declaration: &'schema RustEnum,
}

impl RustWriter {
    fn new(options: RustEmissionOptions) -> Self {
        Self {
            output: String::new(),
            map_key_types: Vec::new(),
            private_type_names: Vec::new(),
            nota_surface: options.nota_surface().clone(),
            target: options.target(),
        }
    }

    fn nota_surface(&self) -> &NotaSurface {
        &self.nota_surface
    }

    fn emits_runtime_support(&self) -> bool {
        self.target.emits_runtime_support()
    }

    fn runtime_planes(&self) -> RuntimePlaneSet {
        self.target.runtime_planes()
    }

    fn emits_all_runtime_planes(&self) -> bool {
        self.runtime_planes().emits_all()
    }

    /// Record the set of type names used as a `BTreeMap` key anywhere
    /// in the schema. A map key type additionally derives `PartialOrd,
    /// Ord` (and the archived form does too) so `BTreeMap<Key, _>`
    /// compiles. Non-key types keep the original derive set, so a
    /// collection-free schema's emission stays byte-identical.
    fn note_map_key_types(&mut self, key_types: Vec<String>) {
        self.map_key_types = key_types;
    }

    fn note_private_type_names(&mut self, names: Vec<String>) {
        self.private_type_names = names;
    }

    /// The derive attribute line for a data-bearing emitted type. Adds
    /// the ordering derives when the type is used as a map key.
    fn data_type_derive(&self, type_name: &Name) -> String {
        self.derive_attribute(
            false,
            self.map_key_types
                .iter()
                .any(|key| key == type_name.as_str()),
        )
    }

    fn copy_data_type_derive(&self) -> String {
        self.derive_attribute(true, false)
    }

    fn root_data_type_derive(&self) -> String {
        self.derive_attribute(false, false)
    }

    fn derive_attribute(&self, includes_copy: bool, includes_ordering: bool) -> String {
        let mut lines = Vec::new();
        if let Some(attribute) = self.nota_surface.feature_gated_derive_attribute() {
            lines.push(attribute);
        }
        let mut derives = Vec::new();
        if self.nota_surface.includes_nota_in_derive() {
            derives.push("nota_next::NotaDecode");
            derives.push("nota_next::NotaEncode");
        }
        derives.push("rkyv::Archive");
        derives.push("rkyv::Serialize");
        derives.push("rkyv::Deserialize");
        derives.push("Clone");
        if includes_copy {
            derives.push("Copy");
        }
        derives.push("Debug");
        derives.push("PartialEq");
        derives.push("Eq");
        if includes_ordering {
            derives.push("PartialOrd");
            derives.push("Ord");
        }
        lines.push(format!("#[derive({})]", derives.join(", ")));
        if includes_ordering {
            lines.push("#[rkyv(derive(PartialEq, Eq, PartialOrd, Ord))]".to_owned());
        }
        lines.join("\n")
    }

    fn line(&mut self, line: impl AsRef<str>) {
        self.output.push_str(line.as_ref());
        self.output.push('\n');
    }

    fn blank(&mut self) {
        self.output.push('\n');
    }

    fn finish(self) -> String {
        self.output
    }

    fn rust_visibility(&self, visibility: Visibility) -> &'static str {
        match visibility {
            Visibility::Public => "pub",
            Visibility::Private => "pub(crate)",
        }
    }

    fn field_visibility(&self, visibility: Visibility, reference: &TypeReference) -> &'static str {
        if visibility == Visibility::Public && self.references_private_type(reference) {
            "pub(crate)"
        } else {
            self.rust_visibility(visibility)
        }
    }

    fn references_private_type(&self, reference: &TypeReference) -> bool {
        match reference {
            TypeReference::Plain(name) => self
                .private_type_names
                .iter()
                .any(|private_name| private_name == name.as_str()),
            TypeReference::Vector(inner) | TypeReference::Optional(inner) => {
                self.references_private_type(inner)
            }
            TypeReference::Map(key, value) => {
                self.references_private_type(key) || self.references_private_type(value)
            }
            TypeReference::String
            | TypeReference::Integer
            | TypeReference::Boolean
            | TypeReference::Path => false,
        }
    }

    /// Emit a `pub use` alias for each cross-crate import.
    ///
    /// The dependency crate emits its own definition of the type; the
    /// consumer references that type through the local alias instead of
    /// re-declaring it. Later fields or variants that name the imported
    /// type therefore use the dependency crate's type identity.
    fn emit_scalar_alias(&mut self, alias: &RustScalarAlias) {
        self.line(format!(
            "pub type {} = {};",
            alias.name(),
            alias.rust_type()
        ));
    }

    fn emit_imports(&mut self, imports: &[RustImport]) {
        if imports.is_empty() {
            return;
        }
        for import in imports {
            self.line(import.use_item());
        }
        self.blank();
    }

    fn emit_type(&mut self, declaration: &RustDeclaration) {
        match declaration.value() {
            RustTypeDeclaration::Alias(value) => self.emit_alias(declaration.visibility(), value),
            RustTypeDeclaration::Struct(value) => self.emit_struct(declaration.visibility(), value),
            RustTypeDeclaration::Newtype(value) => {
                self.emit_newtype(declaration.visibility(), value)
            }
            RustTypeDeclaration::Enum(value) => self.emit_enum(declaration.visibility(), value),
        }
    }

    fn emit_nota_support(&mut self) {
        if !self.nota_surface.emits_nota() {
            return;
        }
        self.emit_nota_gate();
        self.line("pub use nota_next::{");
        self.line("    NotaDecode, NotaDecodeError, NotaEncode, NotaSource,");
        self.line("};");
    }

    fn emit_alias(&mut self, visibility: Visibility, declaration: &RustAlias) {
        self.line(format!(
            "{} type {} = {};",
            self.rust_visibility(visibility),
            declaration.name(),
            self.rust_type(declaration.reference())
        ));
    }

    fn emit_newtype(&mut self, visibility: Visibility, declaration: &RustNewtype) {
        let derive = self.data_type_derive(declaration.name());
        self.line(derive);
        self.line(format!(
            "{} struct {}({} {});",
            self.rust_visibility(visibility),
            declaration.name(),
            self.rust_visibility(visibility),
            self.rust_type(declaration.reference())
        ));
    }

    fn emit_newtype_inherent_impls(&mut self, declarations: &[RustDeclaration]) {
        let newtypes: Vec<_> = declarations
            .iter()
            .filter_map(|declaration| match declaration.value() {
                RustTypeDeclaration::Newtype(value) => Some(value),
                _ => None,
            })
            .collect();
        for newtype in newtypes {
            self.emit_newtype_inherent_impl(newtype);
            self.blank();
        }
    }

    fn emit_newtype_inherent_impl(&mut self, declaration: &RustNewtype) {
        let name = declaration.name();
        let payload_type = self.rust_type(declaration.reference());
        self.line(format!("impl {name} {{"));
        self.line(format!(
            "    pub fn new(payload: {payload_type}) -> Self {{"
        ));
        self.line("        Self(payload)");
        self.line("    }");
        self.blank();
        self.line(format!("    pub fn payload(&self) -> &{payload_type} {{"));
        self.line("        &self.0");
        self.line("    }");
        self.blank();
        self.line(format!(
            "    pub fn into_payload(self) -> {payload_type} {{"
        ));
        self.line("        self.0");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line(format!("impl From<{payload_type}> for {name} {{"));
        self.line(format!("    fn from(payload: {payload_type}) -> Self {{"));
        self.line("        Self::new(payload)");
        self.line("    }");
        self.line("}");
    }

    fn emit_struct(&mut self, visibility: Visibility, declaration: &RustStruct) {
        let derive = self.data_type_derive(declaration.name());
        self.line(derive);
        self.line(format!(
            "{} struct {} {{",
            self.rust_visibility(visibility),
            declaration.name()
        ));
        for field in declaration.fields() {
            self.line(format!(
                "    {} {}: {},",
                self.field_visibility(visibility, field.reference()),
                field.name().as_str(),
                self.rust_type(field.reference())
            ));
        }
        self.line("}");
    }

    fn emit_enum(&mut self, visibility: Visibility, declaration: &RustEnum) {
        let derive = self.data_type_derive(declaration.name());
        self.line(derive);
        self.line(format!(
            "{} enum {} {{",
            self.rust_visibility(visibility),
            declaration.name()
        ));
        for variant in declaration.variants() {
            self.emit_variant(variant);
        }
        self.line("}");
    }

    fn emit_root_enum(&mut self, root_enum: &RustEnum) {
        self.line(self.root_data_type_derive());
        self.line(format!("pub enum {} {{", root_enum.name()));
        for variant in root_enum.variants() {
            self.emit_variant(variant);
        }
        self.line("}");
    }

    fn emit_variant(&mut self, variant: &RustEnumVariant) {
        match variant.payload() {
            Some(reference) => self.line(format!(
                "    {}({}),",
                variant.name(),
                self.rust_type(reference)
            )),
            None => self.line(format!("    {},", variant.name())),
        }
    }

    fn emit_enum_payload_from_impls(
        &mut self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) {
        let alias_names = self.alias_names(declarations);
        for declaration in declarations {
            if let RustTypeDeclaration::Enum(value) = declaration.value() {
                self.emit_enum_payload_from_impls_for(value, &alias_names);
            }
        }
        for root_enum in root_enums {
            self.emit_enum_payload_from_impls_for(root_enum, &alias_names);
        }
    }

    fn alias_names<'declaration>(
        &self,
        declarations: &'declaration [RustDeclaration],
    ) -> Vec<&'declaration str> {
        declarations
            .iter()
            .filter_map(|declaration| match declaration.value() {
                RustTypeDeclaration::Alias(_) => Some(declaration.name().as_str()),
                _ => None,
            })
            .collect()
    }

    fn emit_enum_payload_from_impls_for(
        &mut self,
        declaration: &RustEnum,
        alias_names: &[&str],
    ) -> bool {
        let mut emitted = false;
        for variant in self.unique_non_alias_plain_payload_variants(declaration, alias_names) {
            let Some(payload) = self.plain_payload_name(variant) else {
                continue;
            };
            self.line(format!(
                "impl From<{payload}> for {} {{",
                declaration.name()
            ));
            self.line(format!("    fn from(payload: {payload}) -> Self {{"));
            self.line(format!("        Self::{}(payload)", variant.name()));
            self.line("    }");
            self.line("}");
            self.blank();
            emitted = true;
        }
        emitted
    }

    fn emit_enum_variant_constructors(
        &mut self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) {
        let newtypes: Vec<_> = declarations
            .iter()
            .filter_map(|declaration| match declaration.value() {
                RustTypeDeclaration::Newtype(value) => Some(value),
                _ => None,
            })
            .collect();
        for declaration in declarations {
            if let RustTypeDeclaration::Enum(value) = declaration.value() {
                self.emit_enum_variant_constructors_for(value, &newtypes);
            }
        }
        for root_enum in root_enums {
            self.emit_enum_variant_constructors_for(root_enum, &newtypes);
        }
    }

    fn emit_enum_variant_constructors_for(
        &mut self,
        declaration: &RustEnum,
        newtypes: &[&RustNewtype],
    ) {
        let payload_variants: Vec<_> = declaration
            .variants()
            .iter()
            .filter(|variant| variant.payload().is_some())
            .collect();
        if payload_variants.is_empty() {
            return;
        }
        self.line(format!("impl {} {{", declaration.name()));
        for (index, variant) in payload_variants.iter().enumerate() {
            if index > 0 {
                self.blank();
            }
            let method_name = self.rust_method_name(variant.name());
            let Some(payload) = variant.payload() else {
                continue;
            };
            let constructor = self.enum_variant_constructor_payload(payload, newtypes);
            self.line(format!(
                "    pub fn {method_name}(payload: {}) -> Self {{",
                constructor.argument_type()
            ));
            self.line(format!(
                "        Self::{}({})",
                variant.name(),
                constructor.expression()
            ));
            self.line("    }");
        }
        self.line("}");
        self.blank();
    }

    fn enum_variant_constructor_payload(
        &self,
        payload: &TypeReference,
        newtypes: &[&RustNewtype],
    ) -> EnumConstructorPayload {
        match payload {
            TypeReference::Plain(name) => newtypes
                .iter()
                .find(|newtype| newtype.name() == name)
                .map(|newtype| {
                    EnumConstructorPayload::new(
                        self.rust_type(newtype.reference()),
                        format!("{}::new(payload)", newtype.name()),
                    )
                })
                .unwrap_or_else(|| {
                    EnumConstructorPayload::new(self.rust_type(payload), "payload".to_owned())
                }),
            _ => EnumConstructorPayload::new(self.rust_type(payload), "payload".to_owned()),
        }
    }

    fn unique_plain_payload_variants<'declaration>(
        &self,
        declaration: &'declaration RustEnum,
    ) -> Vec<&'declaration RustEnumVariant> {
        declaration
            .variants()
            .iter()
            .filter(|variant| {
                let Some(payload) = self.plain_payload_name(variant) else {
                    return false;
                };
                declaration
                    .variants()
                    .iter()
                    .filter(|other| self.plain_payload_name(other) == Some(payload))
                    .count()
                    == 1
            })
            .collect()
    }

    fn unique_non_alias_plain_payload_variants<'declaration>(
        &self,
        declaration: &'declaration RustEnum,
        alias_names: &[&str],
    ) -> Vec<&'declaration RustEnumVariant> {
        self.unique_plain_payload_variants(declaration)
            .into_iter()
            .filter(|variant| {
                self.plain_payload_name(variant)
                    .is_none_or(|payload| !alias_names.contains(&payload))
            })
            .collect()
    }

    fn plain_payload_name<'variant>(
        &self,
        variant: &'variant RustEnumVariant,
    ) -> Option<&'variant str> {
        match variant.payload() {
            Some(TypeReference::Plain(name)) => Some(name.as_str()),
            _ => None,
        }
    }

    fn emit_nota_type_bridges(&mut self, declarations: &[RustDeclaration]) {
        if !self.nota_surface.emits_nota() {
            return;
        }
        for declaration in declarations {
            if matches!(declaration.value(), RustTypeDeclaration::Alias(_)) {
                continue;
            }
            self.emit_nota_inherent_bridge(declaration.name().as_str());
            self.blank();
        }
    }

    fn emit_nota_inherent_bridge(&mut self, name: &str) {
        self.emit_nota_gate();
        self.line(format!("impl {name} {{"));
        self.line("    pub fn from_nota_block(block: &nota_next::Block) -> Result<Self, NotaDecodeError> {");
        self.line("        <Self as NotaDecode>::from_nota_block(block)");
        self.line("    }");
        self.blank();
        self.line("    pub fn to_nota(&self) -> String {");
        self.line("        <Self as NotaEncode>::to_nota(self)");
        self.line("    }");
        self.line("}");
    }

    fn emit_nota_copy_inherent_bridge(&mut self, name: &str) {
        self.emit_nota_gate();
        self.line(format!("impl {name} {{"));
        self.line("    pub fn from_nota_block(block: &nota_next::Block) -> Result<Self, NotaDecodeError> {");
        self.line("        <Self as NotaDecode>::from_nota_block(block)");
        self.line("    }");
        self.blank();
        self.line("    pub fn to_nota(self) -> String {");
        self.line("        <Self as NotaEncode>::to_nota(&self)");
        self.line("    }");
        self.line("}");
    }

    fn emit_nota_root_enum_support(&mut self, root_enum: &RustEnum) {
        if !self.nota_surface.emits_nota() {
            return;
        }
        self.emit_nota_inherent_bridge(root_enum.name().as_str());
        self.blank();
        self.emit_nota_gate();
        self.line(format!(
            "impl std::str::FromStr for {} {{",
            root_enum.name()
        ));
        self.line("    type Err = NotaDecodeError;");
        self.blank();
        self.line("    fn from_str(source: &str) -> Result<Self, Self::Err> {");
        self.line("        NotaSource::new(source).parse::<Self>()");
        self.line("    }");
        self.line("}");
        self.blank();
        self.emit_nota_gate();
        self.line(format!(
            "impl std::fmt::Display for {} {{",
            root_enum.name()
        ));
        self.line(
            "    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {",
        );
        self.line("        formatter.write_str(&<Self as NotaEncode>::to_nota(self))");
        self.line("    }");
        self.line("}");
    }

    fn emit_nota_gate(&mut self) {
        if let Some(attribute) = self.nota_surface.feature_gate_attribute() {
            self.line(attribute);
        }
    }

    fn emit_short_headers(&mut self, root_enums: &[RustEnum]) {
        self.line("pub mod short_header {");
        for (root_index, root_enum) in root_enums.iter().enumerate() {
            for (variant_index, variant) in root_enum.variants().iter().enumerate() {
                let constant = format!(
                    "{}_{}",
                    self.constant_name(root_enum.name()),
                    self.constant_name(variant.name())
                );
                let value = ((root_index as u64) << 56) | ((variant_index as u64) << 48);
                self.line(format!("    pub const {constant}: u64 = 0x{value:016X};"));
            }
        }
        self.line("}");
    }

    fn emit_signal_frame_support(&mut self, root_enums: &[RustEnum]) {
        self.line("const SIGNAL_SHORT_HEADER_BYTE_COUNT: usize = 8;");
        self.blank();
        self.line("#[derive(Clone, Debug, PartialEq, Eq)]");
        self.line("pub enum SignalFrameError {");
        self.line("    ArchiveEncode,");
        self.line("    ArchiveDecode,");
        self.line("    FrameTooShort { found: usize },");
        self.line("    UnknownHeader { root_enum: &'static str, header: u64 },");
        self.line("    HeaderMismatch { expected: u64, found: u64 },");
        self.line("}");
        self.blank();
        self.line("impl std::fmt::Display for SignalFrameError {");
        self.line(
            "    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {",
        );
        self.line("        match self {");
        self.line("            Self::ArchiveEncode => formatter.write_str(\"failed to encode rkyv archive\"),");
        self.line("            Self::ArchiveDecode => formatter.write_str(\"failed to decode rkyv archive\"),");
        self.line("            Self::FrameTooShort { found } => write!(formatter, \"signal frame too short: {found} bytes\"),");
        self.line("            Self::UnknownHeader { root_enum, header } => write!(formatter, \"unknown {root_enum} short header 0x{header:016X}\"),");
        self.line("            Self::HeaderMismatch { expected, found } => write!(formatter, \"decoded payload header mismatch: expected 0x{expected:016X}, found 0x{found:016X}\"),");
        self.line("        }");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line("impl std::error::Error for SignalFrameError {}");
        self.blank();

        for root_enum in root_enums {
            self.emit_route_enum(root_enum);
            self.blank();
        }

        for root_enum in root_enums {
            self.emit_signal_frame_impl(root_enum);
            self.blank();
        }
    }

    fn emit_route_enum(&mut self, root_enum: &RustEnum) {
        self.line(self.copy_data_type_derive());
        self.line(format!("pub enum {}Route {{", root_enum.name()));
        for variant in root_enum.variants() {
            self.line(format!("    {},", variant.name()));
        }
        self.line("}");
    }

    fn emit_plane_route_support(&mut self, declarations: &[RustDeclaration]) {
        for declaration in self.plane_route_enums(declarations) {
            self.emit_route_enum(declaration);
            self.blank();
            self.emit_route_impl(declaration);
            self.blank();
        }
    }

    fn emit_route_impl(&mut self, declaration: &RustEnum) {
        let route_name = format!("{}Route", declaration.name());
        self.line(format!("impl {} {{", declaration.name()));
        self.line(format!("    pub fn route(&self) -> {route_name} {{"));
        self.line("        match self {");
        for variant in declaration.variants() {
            match variant.payload() {
                Some(_) => self.line(format!(
                    "            Self::{}(_) => {route_name}::{},",
                    variant.name(),
                    variant.name()
                )),
                None => self.line(format!(
                    "            Self::{} => {route_name}::{},",
                    variant.name(),
                    variant.name()
                )),
            }
        }
        self.line("        }");
        self.line("    }");
        self.line("}");
    }

    fn emit_signal_frame_impl(&mut self, root_enum: &RustEnum) {
        let route_name = format!("{}Route", root_enum.name());
        self.line(format!("impl {} {{", root_enum.name()));
        self.line(format!("    pub fn route(&self) -> {route_name} {{"));
        self.line("        match self {");
        for variant in root_enum.variants() {
            match variant.payload() {
                Some(_) => self.line(format!(
                    "            Self::{}(_) => {route_name}::{},",
                    variant.name(),
                    variant.name()
                )),
                None => self.line(format!(
                    "            Self::{} => {route_name}::{},",
                    variant.name(),
                    variant.name()
                )),
            }
        }
        self.line("        }");
        self.line("    }");
        self.blank();
        self.line("    pub fn short_header(&self) -> u64 {");
        self.line("        match self {");
        for variant in root_enum.variants() {
            let constant = format!(
                "{}_{}",
                self.constant_name(root_enum.name()),
                self.constant_name(variant.name())
            );
            match variant.payload() {
                Some(_) => self.line(format!(
                    "            Self::{}(_) => short_header::{constant},",
                    variant.name()
                )),
                None => self.line(format!(
                    "            Self::{} => short_header::{constant},",
                    variant.name()
                )),
            }
        }
        self.line("        }");
        self.line("    }");
        self.blank();
        self.line(format!(
            "    pub fn route_from_short_header(header: u64) -> Result<{route_name}, SignalFrameError> {{"
        ));
        self.line("        match header {");
        for variant in root_enum.variants() {
            let constant = format!(
                "{}_{}",
                self.constant_name(root_enum.name()),
                self.constant_name(variant.name())
            );
            self.line(format!(
                "            short_header::{constant} => Ok({route_name}::{}),",
                variant.name()
            ));
        }
        self.line(format!(
            "            _ => Err(SignalFrameError::UnknownHeader {{ root_enum: \"{}\", header }}),",
            root_enum.name()
        ));
        self.line("        }");
        self.line("    }");
        self.blank();
        self.line("    pub fn encode_signal_frame(&self) -> Result<Vec<u8>, SignalFrameError> {");
        self.line("        let archive = rkyv::to_bytes::<rkyv::rancor::Error>(self)");
        self.line("            .map_err(|_| SignalFrameError::ArchiveEncode)?;");
        self.line("        let mut frame = Vec::with_capacity(SIGNAL_SHORT_HEADER_BYTE_COUNT + archive.len());");
        self.line("        frame.extend_from_slice(&self.short_header().to_le_bytes());");
        self.line("        frame.extend_from_slice(&archive);");
        self.line("        Ok(frame)");
        self.line("    }");
        self.blank();
        self.line(format!(
            "    pub fn decode_signal_frame(frame: &[u8]) -> Result<({route_name}, Self), SignalFrameError> {{"
        ));
        self.line("        if frame.len() < SIGNAL_SHORT_HEADER_BYTE_COUNT {");
        self.line(
            "            return Err(SignalFrameError::FrameTooShort { found: frame.len() });",
        );
        self.line("        }");
        self.line("        let mut header_bytes = [0_u8; SIGNAL_SHORT_HEADER_BYTE_COUNT];");
        self.line(
            "        header_bytes.copy_from_slice(&frame[..SIGNAL_SHORT_HEADER_BYTE_COUNT]);",
        );
        self.line("        let header = u64::from_le_bytes(header_bytes);");
        self.line("        let route = Self::route_from_short_header(header)?;");
        self.line("        let value = rkyv::from_bytes::<Self, rkyv::rancor::Error>(&frame[SIGNAL_SHORT_HEADER_BYTE_COUNT..])");
        self.line("            .map_err(|_| SignalFrameError::ArchiveDecode)?;");
        self.line("        let expected = value.short_header();");
        self.line("        if expected != header {");
        self.line(
            "            return Err(SignalFrameError::HeaderMismatch { expected, found: header });",
        );
        self.line("        }");
        self.line("        Ok((route, value))");
        self.line("    }");
        self.line("}");
    }

    fn emit_trace_support(&mut self, declarations: &[RustDeclaration], root_enums: &[RustEnum]) {
        let signal_roots = if self.runtime_planes().emits_signal() {
            self.trace_signal_roots(root_enums)
        } else {
            Vec::new()
        };
        let nexus_roots = if self.runtime_planes().emits_nexus() {
            self.trace_nexus_roots(declarations)
        } else {
            Vec::new()
        };
        let sema_roots = if self.runtime_planes().emits_sema() {
            self.trace_sema_roots(declarations, root_enums)
        } else {
            Vec::new()
        };
        let signal_actor_variants = self.trace_signal_actor_variants(declarations, root_enums);
        let nexus_actor_variants = self.trace_nexus_actor_variants(declarations);
        let sema_actor_variants = self.trace_sema_actor_variants(declarations, root_enums);
        let has_signal = !signal_roots.is_empty() || !signal_actor_variants.is_empty();
        let has_nexus = !nexus_roots.is_empty() || !nexus_actor_variants.is_empty();
        let has_sema = !sema_roots.is_empty() || !sema_actor_variants.is_empty();
        if !has_signal && !has_nexus && !has_sema {
            return;
        }
        self.emit_object_name_enum(
            "SignalObjectName",
            "Signal",
            &signal_roots,
            &signal_actor_variants,
        );
        self.emit_object_name_enum(
            "NexusObjectName",
            "Nexus",
            &nexus_roots,
            &nexus_actor_variants,
        );
        self.emit_object_name_enum("SemaObjectName", "Sema", &sema_roots, &sema_actor_variants);
        self.line(self.copy_data_type_derive());
        self.line("pub enum ObjectName {");
        if has_signal {
            self.line("    Signal(SignalObjectName),");
        }
        if has_nexus {
            self.line("    Nexus(NexusObjectName),");
        }
        if has_sema {
            self.line("    Sema(SemaObjectName),");
        }
        self.line("}");
        self.blank();
        self.line(self.copy_data_type_derive());
        self.line("pub struct TraceEvent(pub ObjectName);");
        self.blank();
        self.line("impl ObjectName {");
        self.line("    pub fn name(self) -> &'static str {");
        self.line("        match self {");
        if has_signal {
            self.line("            Self::Signal(object_name) => object_name.name(),");
        }
        if has_nexus {
            self.line("            Self::Nexus(object_name) => object_name.name(),");
        }
        if has_sema {
            self.line("            Self::Sema(object_name) => object_name.name(),");
        }
        self.line("        }");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line("impl TraceEvent {");
        self.line("    pub fn new(object_name: ObjectName) -> Self {");
        self.line("        Self(object_name)");
        self.line("    }");
        self.blank();
        self.line("    pub fn object_name(&self) -> ObjectName {");
        self.line("        self.0");
        self.line("    }");
        self.blank();
        self.line("    pub fn name(&self) -> &'static str {");
        self.line("        self.0.name()");
        self.line("    }");
        self.line("}");
        self.blank();
    }

    fn emit_object_name_enum(
        &mut self,
        enum_name: &str,
        rendered_prefix: &str,
        interface_roots: &[TraceInterfaceRoot<'_>],
        actor_variants: &[&str],
    ) {
        if interface_roots.is_empty() && actor_variants.is_empty() {
            return;
        }
        self.line(self.copy_data_type_derive());
        self.line(format!("pub enum {enum_name} {{"));
        for root in interface_roots {
            self.line(format!(
                "    {}({}Route),",
                root.object_variant, root.type_name
            ));
        }
        for variant in actor_variants {
            self.line(format!("    {variant},"));
        }
        self.line("}");
        self.blank();
        self.line(format!("impl {enum_name} {{"));
        self.line("    pub fn name(self) -> &'static str {");
        self.line("        match self {");
        for root in interface_roots {
            self.line(format!(
                "            Self::{}(route) => match route {{",
                root.object_variant
            ));
            for variant in root.enum_declaration.variants() {
                self.line(format!(
                    "                {}Route::{} => \"{}{}\",",
                    root.type_name,
                    variant.name(),
                    root.name_prefix,
                    variant.name()
                ));
            }
            self.line("            },");
        }
        for variant in actor_variants {
            self.line(format!(
                "            Self::{variant} => \"{rendered_prefix}{variant}\","
            ));
        }
        self.line("        }");
        self.line("    }");
        self.line("}");
        self.blank();
    }

    fn plane_route_enums<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
    ) -> Vec<&'schema RustEnum> {
        declarations
            .iter()
            .filter_map(|declaration| match declaration.value() {
                RustTypeDeclaration::Enum(value)
                    if self.emits_plane_route_type(declaration.name().as_str()) =>
                {
                    Some(value)
                }
                _ => None,
            })
            .collect()
    }

    fn trace_signal_roots<'schema>(
        &self,
        root_enums: &'schema [RustEnum],
    ) -> Vec<TraceInterfaceRoot<'schema>> {
        let mut roots = Vec::new();
        if let Some(input) = self.root_enum_named(root_enums, "Input") {
            roots.push(TraceInterfaceRoot {
                object_variant: "Input",
                name_prefix: "SignalInput",
                type_name: input.name(),
                enum_declaration: input,
            });
        }
        if let Some(output) = self.root_enum_named(root_enums, "Output") {
            roots.push(TraceInterfaceRoot {
                object_variant: "Output",
                name_prefix: "SignalOutput",
                type_name: output.name(),
                enum_declaration: output,
            });
        }
        roots
    }

    fn trace_nexus_roots<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
    ) -> Vec<TraceInterfaceRoot<'schema>> {
        let mut roots = Vec::new();
        if let Some(input) = self.declaration_enum_named(declarations, "NexusWork") {
            roots.push(TraceInterfaceRoot {
                object_variant: "Work",
                name_prefix: "NexusWork",
                type_name: input.name(),
                enum_declaration: input,
            });
        }
        if let Some(output) = self.declaration_enum_named(declarations, "NexusAction") {
            roots.push(TraceInterfaceRoot {
                object_variant: "Action",
                name_prefix: "NexusAction",
                type_name: output.name(),
                enum_declaration: output,
            });
        }
        roots
    }

    fn trace_sema_roots<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
        root_enums: &'schema [RustEnum],
    ) -> Vec<TraceInterfaceRoot<'schema>> {
        let mut roots = Vec::new();
        for root in [
            self.sema_write_input_root(declarations, root_enums)
                .map(|root| (root, "WriteInput", "SemaWriteInput")),
            self.sema_read_input_root(declarations, root_enums)
                .map(|root| (root, "ReadInput", "SemaReadInput")),
            self.sema_write_output_root(declarations, root_enums)
                .map(|root| (root, "WriteOutput", "SemaWriteOutput")),
            self.sema_read_output_root(declarations, root_enums)
                .map(|root| (root, "ReadOutput", "SemaReadOutput")),
        ]
        .into_iter()
        .flatten()
        {
            let (declaration, object_variant, name_prefix) = root;
            roots.push(TraceInterfaceRoot {
                object_variant,
                name_prefix,
                type_name: declaration.name(),
                enum_declaration: declaration,
            });
        }
        roots
    }

    fn trace_signal_actor_variants(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> Vec<&'static str> {
        let mut variants = Vec::new();
        let has_signal_roots =
            self.has_root_enum(root_enums, "Input") && self.has_root_enum(root_enums, "Output");
        let has_concrete_nexus =
            self.has_type(declarations, "NexusWork") && self.has_type(declarations, "NexusAction");
        if has_signal_roots
            && (matches!(self.target, RustEmissionTarget::SignalRuntime)
                || (self.runtime_planes().emits_signal() && has_concrete_nexus))
        {
            variants.extend([
                "Started", "Stopped", "Admitted", "Rejected", "Triaged", "Replied",
            ]);
        }
        variants
    }

    fn trace_nexus_actor_variants(&self, declarations: &[RustDeclaration]) -> Vec<&'static str> {
        let mut variants = Vec::new();
        if self.runtime_planes().emits_nexus()
            && self.has_type(declarations, "NexusWork")
            && self.has_type(declarations, "NexusAction")
        {
            variants.extend(["Started", "Stopped", "Entered", "Decided"]);
        }
        variants
    }

    fn trace_sema_actor_variants(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> Vec<&'static str> {
        let mut variants = Vec::new();
        if !self.runtime_planes().emits_sema() {
            return variants;
        }
        let has_write = self.has_sema_write_roots(declarations, root_enums);
        let has_read = self.has_sema_read_roots(declarations, root_enums);
        if has_write || has_read {
            variants.extend(["Started", "Stopped"]);
        }
        if has_write {
            variants.push("WriteApplied");
        }
        if has_read {
            variants.push("ReadObserved");
        }
        variants
    }

    fn is_plane_route_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "NexusWork"
                | "NexusAction"
                | "WriteInput"
                | "WriteOutput"
                | "ReadInput"
                | "ReadOutput"
                | "SemaWriteInput"
                | "SemaWriteOutput"
                | "SemaReadInput"
                | "SemaReadOutput"
        )
    }

    fn emits_plane_route_type(&self, type_name: &str) -> bool {
        self.is_plane_route_type(type_name)
            && ((self.runtime_planes().emits_nexus()
                && matches!(type_name, "NexusWork" | "NexusAction"))
                || (self.runtime_planes().emits_sema()
                    && matches!(
                        type_name,
                        "WriteInput"
                            | "WriteOutput"
                            | "ReadInput"
                            | "ReadOutput"
                            | "SemaWriteInput"
                            | "SemaWriteOutput"
                            | "SemaReadInput"
                            | "SemaReadOutput"
                    )))
    }

    fn emit_mail_event_support(&mut self, root_enums: &[RustEnum]) {
        if self.runtime_planes().emits_signal() {
            self.line(self.copy_data_type_derive());
            self.line("pub struct MessageIdentifier(pub Integer);");
            if self.nota_surface.emits_nota() {
                self.emit_nota_copy_inherent_bridge("MessageIdentifier");
            }
            self.blank();
        }
        self.line(self.copy_data_type_derive());
        self.line("pub struct OriginRoute(pub Integer);");
        if self.nota_surface.emits_nota() {
            self.emit_nota_copy_inherent_bridge("OriginRoute");
        }
        self.blank();
        if self.emits_all_runtime_planes() {
            self.emit_signal_message_root_support(root_enums);
            self.blank();
            self.emit_schema_plane_support();
            self.blank();
        }
        if self.runtime_planes().emits_signal() {
            self.emit_plane_envelope("Signal");
            self.blank();
        }
        if self.runtime_planes().emits_nexus() {
            self.emit_plane_envelope("Nexus");
            self.blank();
        }
        if self.runtime_planes().emits_sema() {
            self.emit_plane_envelope("Sema");
            self.blank();
        }
        if self.runtime_planes().emits_signal() {
            if !self.emits_all_runtime_planes() {
                self.emit_signal_message_root_support(root_enums);
                self.blank();
            }
            self.emit_signal_mail_lifecycle_support(root_enums);
        }
    }

    fn emit_signal_message_root_support(&mut self, root_enums: &[RustEnum]) {
        self.line(self.copy_data_type_derive());
        self.line("pub enum MessageRoot {");
        for root_enum in root_enums {
            self.line(format!("    {},", root_enum.name()));
        }
        self.line("}");
    }

    fn emit_signal_mail_lifecycle_support(&mut self, root_enums: &[RustEnum]) {
        self.line("#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]");
        self.line("pub struct MessageSent {");
        self.line("    pub identifier: MessageIdentifier,");
        self.line("    pub origin_route: OriginRoute,");
        self.line("    pub root: MessageRoot,");
        self.line("    pub short_header: Integer,");
        self.line("}");
        self.blank();
        self.line("#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]");
        self.line("pub struct MessageProcessed<Reply> {");
        self.line("    pub identifier: MessageIdentifier,");
        self.line("    pub origin_route: OriginRoute,");
        self.line("    pub reply: Reply,");
        self.line("}");
        self.blank();
        self.line("pub trait MessageSentHook {");
        self.line("    type Error;");
        self.blank();
        self.line("    fn message_sent(&mut self, event: MessageSent) -> Result<(), Self::Error>;");
        self.line("}");
        self.blank();
        self.line("pub trait MessageProcessedHook<Reply> {");
        self.line("    type Error;");
        self.blank();
        self.line("    fn message_processed(&mut self, event: MessageProcessed<Reply>) -> Result<(), Self::Error>;");
        self.line("}");
        self.blank();
        self.line("impl MessageSent {");
        self.line("    pub fn origin_route(&self) -> OriginRoute {");
        self.line("        self.origin_route");
        self.line("    }");
        self.blank();
        self.line("    pub fn push_to<Hook>(&self, hook: &mut Hook) -> Result<(), Hook::Error>");
        self.line("    where");
        self.line("        Hook: MessageSentHook,");
        self.line("    {");
        self.line("        hook.message_sent(self.clone())");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line("impl<Reply> MessageProcessed<Reply> {");
        self.line("    pub fn new(identifier: MessageIdentifier, origin_route: OriginRoute, reply: Reply) -> Self {");
        self.line("        Self { identifier, origin_route, reply }");
        self.line("    }");
        self.blank();
        self.line("    pub fn identifier(&self) -> MessageIdentifier {");
        self.line("        self.identifier");
        self.line("    }");
        self.blank();
        self.line("    pub fn origin_route(&self) -> OriginRoute {");
        self.line("        self.origin_route");
        self.line("    }");
        self.blank();
        self.line("    pub fn into_reply(self) -> Reply {");
        self.line("        self.reply");
        self.line("    }");
        self.blank();
        self.line("    pub fn push_to<Hook>(&self, hook: &mut Hook) -> Result<(), Hook::Error>");
        self.line("    where");
        self.line("        Hook: MessageProcessedHook<Reply>,");
        self.line("        Reply: Clone,");
        self.line("    {");
        self.line("        hook.message_processed(self.clone())");
        self.line("    }");
        self.line("}");
        self.blank();
        for root_enum in root_enums {
            self.line(format!("impl {} {{", root_enum.name()));
            self.line(
                "    pub fn with_origin_route(self, origin_route: OriginRoute) -> Signal<Self> {",
            );
            self.line("        Signal::new(origin_route, self)");
            self.line("    }");
            self.blank();
            self.line("}");
            self.blank();
            self.line(format!("impl signal::Signal<{}> {{", root_enum.name()));
            self.line(
                "    pub fn message_sent(&self, identifier: MessageIdentifier) -> MessageSent {",
            );
            self.line("        MessageSent {");
            self.line("            identifier,");
            self.line("            origin_route: self.origin_route(),");
            self.line(format!(
                "            root: MessageRoot::{},",
                root_enum.name()
            ));
            self.line("            short_header: self.root().short_header(),");
            self.line("        }");
            self.line("    }");
            self.line("}");
            self.blank();
        }
    }

    fn emit_schema_plane_support(&mut self) {
        self.line("pub mod schema {");
        self.line("    #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]");
        self.line("    pub enum Plane<SignalRoot, NexusRoot, SemaRoot> {");
        self.line("        Signal(super::Signal<SignalRoot>),");
        self.line("        Nexus(super::Nexus<NexusRoot>),");
        self.line("        Sema(super::Sema<SemaRoot>),");
        self.line("    }");
        self.blank();
        self.line(
            "    impl<SignalRoot, NexusRoot, SemaRoot> Plane<SignalRoot, NexusRoot, SemaRoot> {",
        );
        self.line("        pub fn origin_route(&self) -> super::OriginRoute {");
        self.line("            match self {");
        self.line("                Self::Signal(message) => message.origin_route(),");
        self.line("                Self::Nexus(message) => message.origin_route(),");
        self.line("                Self::Sema(message) => message.origin_route(),");
        self.line("            }");
        self.line("        }");
        self.line("    }");
        self.line("}");
    }

    fn emit_plane_envelope(&mut self, name: &str) {
        self.line("#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]");
        self.line(format!("pub struct {name}<Root> {{"));
        self.line("    pub origin_route: OriginRoute,");
        self.line("    pub root: Root,");
        self.line("}");
        self.blank();
        self.line(format!("impl<Root> {name}<Root> {{"));
        self.line("    pub fn new(origin_route: OriginRoute, root: Root) -> Self {");
        self.line("        Self { origin_route, root }");
        self.line("    }");
        self.blank();
        self.line("    pub fn origin_route(&self) -> OriginRoute {");
        self.line("        self.origin_route");
        self.line("    }");
        self.blank();
        self.line("    pub fn root(&self) -> &Root {");
        self.line("        &self.root");
        self.line("    }");
        self.blank();
        self.line("    pub fn into_root(self) -> Root {");
        self.line("        self.root");
        self.line("    }");
        self.blank();
        self.line(format!("    pub fn map_root<NextRoot>(self, map: impl FnOnce(Root) -> NextRoot) -> {name}<NextRoot> {{"));
        self.line(format!(
            "        {name}::new(self.origin_route, map(self.root))"
        ));
        self.line("    }");
        self.line("}");
    }

    fn emit_plane_namespaces(&mut self, declarations: &[RustDeclaration], root_enums: &[RustEnum]) {
        if self.runtime_planes().emits_signal()
            && (self.has_root_enum(root_enums, "Input") || self.has_root_enum(root_enums, "Output"))
        {
            self.line("#[allow(clippy::module_inception)]");
            self.line("pub mod signal {");
            if self.has_root_enum(root_enums, "Input") {
                self.line("    pub type Input = super::Input;");
            }
            if self.has_root_enum(root_enums, "Output") {
                self.line("    pub type Output = super::Output;");
            }
            self.line("    pub type Signal<Root> = super::Signal<Root>;");
            self.line("}");
            self.blank();
        }
        if self.runtime_planes().emits_nexus()
            && (self.has_type(declarations, "NexusWork")
                || self.has_type(declarations, "NexusAction"))
        {
            self.line("#[allow(clippy::module_inception)]");
            self.line("pub mod nexus {");
            if self.has_type(declarations, "NexusWork") {
                self.line("    pub type Work = super::NexusWork;");
            }
            if self.has_type(declarations, "NexusAction") {
                self.line("    pub type Action = super::NexusAction;");
            }
            self.line("    pub type Nexus<Root> = super::Nexus<Root>;");
            self.line("}");
            self.blank();
        }
        let sema_write_input_name = self.sema_write_input_type_name(declarations, root_enums);
        let sema_write_output_name = self.sema_write_output_type_name(declarations, root_enums);
        let sema_read_input_name = self.sema_read_input_type_name(declarations, root_enums);
        let sema_read_output_name = self.sema_read_output_type_name(declarations, root_enums);
        if self.runtime_planes().emits_sema()
            && (sema_write_input_name.is_some()
                || sema_write_output_name.is_some()
                || sema_read_input_name.is_some()
                || sema_read_output_name.is_some())
        {
            self.line("#[allow(clippy::module_inception)]");
            self.line("pub mod sema {");
            if let Some(type_name) = sema_write_input_name {
                self.line(format!("    pub type WriteInput = super::{type_name};"));
            }
            if let Some(type_name) = sema_write_output_name {
                self.line(format!("    pub type WriteOutput = super::{type_name};"));
            }
            if let Some(type_name) = sema_read_input_name {
                self.line(format!("    pub type ReadInput = super::{type_name};"));
            }
            if let Some(type_name) = sema_read_output_name {
                self.line(format!("    pub type ReadOutput = super::{type_name};"));
            }
            self.line("    pub type Sema<Root> = super::Sema<Root>;");
            self.line("}");
            self.blank();
        }
        if self.runtime_planes().emits_nexus() && self.has_type(declarations, "NexusWork") {
            self.emit_plane_origin_route_constructor("NexusWork", "nexus::Nexus", "nexus::Nexus");
        }
        if self.runtime_planes().emits_nexus() && self.has_type(declarations, "NexusAction") {
            self.emit_plane_origin_route_constructor("NexusAction", "nexus::Nexus", "nexus::Nexus");
        }
        if self.runtime_planes().emits_sema()
            && let Some(type_name) = sema_write_input_name
        {
            self.emit_plane_origin_route_constructor(type_name, "sema::Sema", "sema::Sema");
        }
        if self.runtime_planes().emits_sema()
            && let Some(type_name) = sema_write_output_name
        {
            self.emit_plane_origin_route_constructor(type_name, "sema::Sema", "sema::Sema");
        }
        if self.runtime_planes().emits_sema()
            && let Some(type_name) = sema_read_input_name
        {
            self.emit_plane_origin_route_constructor(type_name, "sema::Sema", "sema::Sema");
        }
        if self.runtime_planes().emits_sema()
            && let Some(type_name) = sema_read_output_name
        {
            self.emit_plane_origin_route_constructor(type_name, "sema::Sema", "sema::Sema");
        }
    }

    fn emit_plane_origin_route_constructor(
        &mut self,
        type_name: &str,
        return_type: &str,
        constructor: &str,
    ) {
        self.line(format!("impl {type_name} {{"));
        self.line(format!(
            "    pub fn with_origin_route(self, origin_route: OriginRoute) -> {return_type}<Self> {{"
        ));
        self.line(format!("        {constructor}::new(origin_route, self)"));
        self.line("    }");
        self.line("}");
        self.blank();
    }

    fn emit_plane_projection_support(
        &mut self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) {
        if !self.emits_all_runtime_planes() {
            return;
        }
        let signal_input = self.root_enum_named(root_enums, "Input");
        let signal_output = self.root_enum_named(root_enums, "Output");
        let nexus_work = self.declaration_enum_named(declarations, "NexusWork");
        let nexus_action = self.declaration_enum_named(declarations, "NexusAction");
        let sema_write_input = self.declaration_enum_named(declarations, "SemaWriteInput");
        let sema_write_output = self.declaration_enum_named(declarations, "SemaWriteOutput");
        let sema_read_input = self.declaration_enum_named(declarations, "SemaReadInput");
        let sema_read_output = self.declaration_enum_named(declarations, "SemaReadOutput");

        if let (
            Some(signal_input),
            Some(signal_output),
            Some(nexus_work),
            Some(nexus_action),
            Some(sema_write_input),
            Some(sema_write_output),
            Some(sema_read_input),
            Some(sema_read_output),
        ) = (
            signal_input,
            signal_output,
            nexus_work,
            nexus_action,
            sema_write_input,
            sema_write_output,
            sema_read_input,
            sema_read_output,
        ) {
            let projection = SplitSemaProjection {
                signal_input,
                signal_output,
                nexus_work,
                nexus_action,
                sema_write_input,
                sema_write_output,
                sema_read_input,
                sema_read_output,
            };
            if self.can_emit_split_nexus_work_projection(&projection) {
                self.emit_split_nexus_work_projection(&projection);
            }
            self.emit_nexus_action_projection(nexus_action);
            if self.enum_has_unique_payload_variant(
                nexus_work,
                "SemaWriteCompleted",
                "SemaWriteOutput",
            ) {
                self.emit_split_sema_output_projection("WriteOutput", "SemaWriteOutput");
            }
            if self.enum_has_unique_payload_variant(
                nexus_work,
                "SemaReadCompleted",
                "SemaReadOutput",
            ) {
                self.emit_split_sema_output_projection("ReadOutput", "SemaReadOutput");
            }
        }
    }

    fn emit_nexus_runner_next_step_projection(&mut self, shape: &NexusRunnerShape) {
        self.line(format!(
            "pub type NexusRunnerNextStep = triad_runtime::NextStep<{}, {}, {}, {}, NexusWork>;",
            shape.reply_type,
            shape.sema_write_input_type(),
            shape.sema_read_input_type(),
            shape.effect_command_type()
        ));
        self.blank();
        self.line("impl NexusAction {");
        self.line("    pub fn into_runner_next_step(self) -> NexusRunnerNextStep {");
        self.line("        match self {");
        if shape.emits_sema_write() {
            self.line("            Self::CommandSemaWrite(input) => triad_runtime::NextStep::SemaWrite(input),");
        }
        if shape.emits_sema_read() {
            self.line("            Self::CommandSemaRead(input) => triad_runtime::NextStep::SemaRead(input),");
        }
        self.line(
            "            Self::ReplyToSignal(output) => triad_runtime::NextStep::Reply(output),",
        );
        if shape.emits_effect() {
            self.line("            Self::CommandEffect(effect) => triad_runtime::NextStep::RunEffect(effect),");
        }
        if shape.has_continue {
            self.line(
                "            Self::Continue(work) => triad_runtime::NextStep::Continue(work),",
            );
        }
        self.line("        }");
        self.line("    }");
        self.line("}");
        self.blank();
    }

    fn emit_nexus_runner_adapter(&mut self, shape: &NexusRunnerShape) {
        self.line("struct NexusRunnerAdapter<'engine, Engine> {");
        self.line("    engine: &'engine mut Engine,");
        self.line("    origin_route: OriginRoute,");
        self.line("}");
        self.blank();
        self.line("impl<'engine, Engine> NexusRunnerAdapter<'engine, Engine> {");
        self.line("    fn new(engine: &'engine mut Engine, origin_route: OriginRoute) -> Self {");
        self.line("        Self { engine, origin_route }");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line("impl<'engine, Engine> triad_runtime::RunnerEngines for NexusRunnerAdapter<'engine, Engine>");
        self.line("where");
        self.line("    Engine: NexusEngine,");
        self.line("{");
        self.line(format!("    type Reply = {};", shape.reply_type));
        self.line(format!(
            "    type SemaWrite = {};",
            shape.sema_write_input_type()
        ));
        self.line(format!(
            "    type SemaRead = {};",
            shape.sema_read_input_type()
        ));
        self.line(format!(
            "    type Effect = {};",
            shape.effect_command_type()
        ));
        self.line("    type Work = NexusWork;");
        self.blank();
        self.line("    fn decide_next_step(&mut self, work: Self::Work) -> triad_runtime::runner::RunnerNextStep<Self> {");
        self.line("        let action = NexusEngine::decide(self.engine, work.with_origin_route(self.origin_route)).into_root();");
        self.line("        action.into_runner_next_step()");
        self.line("    }");
        self.blank();
        self.line("    fn apply_sema_write(&mut self, write: Self::SemaWrite) -> Self::Work {");
        if let Some(output_type) = shape.sema_write_output_type.as_deref() {
            self.line(format!(
                "        let output: {output_type} = NexusEngine::apply_sema_write(self.engine, self.origin_route, write);"
            ));
            self.line("        NexusWork::sema_write_completed(output)");
        } else {
            self.line("        match write {}");
        }
        self.line("    }");
        self.blank();
        self.line("    fn observe_sema_read(&self, read: Self::SemaRead) -> Self::Work {");
        if let Some(output_type) = shape.sema_read_output_type.as_deref() {
            self.line(format!(
                "        let output: {output_type} = NexusEngine::observe_sema_read(self.engine, self.origin_route, read);"
            ));
            self.line("        NexusWork::sema_read_completed(output)");
        } else {
            self.line("        match read {}");
        }
        self.line("    }");
        self.blank();
        self.line("    fn run_effect(&mut self, effect: Self::Effect) -> Self::Work {");
        if let Some(output_type) = shape.effect_result_type.as_deref() {
            self.line(format!(
                "        let output: {output_type} = NexusEngine::run_effect(self.engine, effect);"
            ));
            self.line("        NexusWork::effect_completed(output)");
        } else {
            self.line("        match effect {}");
        }
        self.line("    }");
        self.blank();
        self.line("    fn budget_exhausted_reply(&self, exhausted: triad_runtime::ContinuationExhausted) -> Self::Reply {");
        self.line("        NexusEngine::budget_exhausted_reply(self.engine, exhausted)");
        self.line("    }");
        self.line("}");
        self.blank();
    }

    fn emit_split_nexus_work_projection(&mut self, projection: &SplitSemaProjection<'_>) {
        self.line("impl nexus::Nexus<nexus::Work> {");
        self.line("    pub fn into_nexus_action(self) -> nexus::Nexus<nexus::Action> {");
        self.line("        let origin_route = self.origin_route();");
        self.line("        match self.into_root() {");
        self.line("            NexusWork::SignalArrived(input) => match input {");
        for variant in projection.signal_input.variants() {
            if let Some(target_variant) =
                self.exact_target_variant_for_source(variant, projection.sema_write_input)
            {
                self.line(format!(
                    "                Input::{}(payload) => NexusAction::from(SemaWriteInput::{}(payload)),",
                    variant.name(),
                    target_variant.name()
                ));
                continue;
            }
            if let Some(target_variant) =
                self.exact_target_variant_for_source(variant, projection.sema_read_input)
            {
                self.line(format!(
                    "                Input::{}(payload) => NexusAction::from(SemaReadInput::{}(payload)),",
                    variant.name(),
                    target_variant.name()
                ));
                continue;
            }
            let write_fallback =
                self.fallback_target_variant_for_source(variant, projection.sema_write_input);
            let read_fallback =
                self.fallback_target_variant_for_source(variant, projection.sema_read_input);
            match (write_fallback, read_fallback) {
                (Some(target_variant), None) => {
                    self.line(format!(
                        "                Input::{}(payload) => NexusAction::from(SemaWriteInput::{}(payload)),",
                        variant.name(),
                        target_variant.name()
                    ));
                }
                (None, Some(target_variant)) => {
                    self.line(format!(
                        "                Input::{}(payload) => NexusAction::from(SemaReadInput::{}(payload)),",
                        variant.name(),
                        target_variant.name()
                    ));
                }
                (Some(_), Some(_)) | (None, None) => {}
            }
        }
        self.line("            },");
        self.line("            NexusWork::SemaWriteCompleted(output) => match output {");
        for variant in projection.sema_write_output.variants() {
            if let Some(target_variant) =
                self.target_variant_for_source(variant, projection.signal_output)
            {
                self.line(format!(
                    "                SemaWriteOutput::{}(payload) => NexusAction::from(Output::{}(payload)),",
                    variant.name(),
                    target_variant.name()
                ));
            }
        }
        self.line("            },");
        self.line("            NexusWork::SemaReadCompleted(output) => match output {");
        for variant in projection.sema_read_output.variants() {
            if let Some(target_variant) =
                self.target_variant_for_source(variant, projection.signal_output)
            {
                self.line(format!(
                    "                SemaReadOutput::{}(payload) => NexusAction::from(Output::{}(payload)),",
                    variant.name(),
                    target_variant.name()
                ));
            }
        }
        self.line("            },");
        self.line(
            "            _ => panic!(\"nexus work cannot project to a generated nexus action\"),",
        );
        self.line("        }");
        self.line("        .with_origin_route(origin_route)");
        self.line("    }");
        self.line("}");
        self.blank();
    }

    fn emit_nexus_action_projection(&mut self, nexus_action: &RustEnum) {
        let has_sema_write =
            self.enum_has_variant_payload(nexus_action, "CommandSemaWrite", "SemaWriteInput");
        let has_sema_read =
            self.enum_has_variant_payload(nexus_action, "CommandSemaRead", "SemaReadInput");
        let has_signal = self.enum_has_variant_payload(nexus_action, "ReplyToSignal", "Output");
        if !has_sema_write && !has_sema_read && !has_signal {
            return;
        }
        self.line("impl nexus::Nexus<nexus::Action> {");
        if has_sema_write {
            self.line("    pub fn into_sema_write_input(self) -> sema::Sema<sema::WriteInput> {");
            self.line("        let origin_route = self.origin_route();");
            self.line("        match self.into_root() {");
            self.line("            NexusAction::CommandSemaWrite(input) => input.with_origin_route(origin_route),");
            self.line("            _ => panic!(\"nexus action is not a SEMA write input\"),");
            self.line("        }");
            self.line("    }");
            if has_sema_read || has_signal {
                self.blank();
            }
        }
        if has_sema_read {
            self.line("    pub fn into_sema_read_input(self) -> sema::Sema<sema::ReadInput> {");
            self.line("        let origin_route = self.origin_route();");
            self.line("        match self.into_root() {");
            self.line("            NexusAction::CommandSemaRead(input) => input.with_origin_route(origin_route),");
            self.line("            _ => panic!(\"nexus action is not a SEMA read input\"),");
            self.line("        }");
            self.line("    }");
            if has_signal {
                self.blank();
            }
        }
        if has_signal {
            self.line("    pub fn into_signal_output(self) -> signal::Signal<signal::Output> {");
            self.line("        let origin_route = self.origin_route();");
            self.line("        match self.into_root() {");
            self.line("            NexusAction::ReplyToSignal(output) => output.with_origin_route(origin_route),");
            self.line("            _ => panic!(\"nexus action is not a signal reply\"),");
            self.line("        }");
            self.line("    }");
        }
        self.line("}");
        self.blank();
    }

    fn emit_split_sema_output_projection(&mut self, plane_alias: &str, type_name: &str) {
        self.line(format!("impl sema::Sema<sema::{plane_alias}> {{"));
        self.line("    pub fn into_nexus_work(self) -> nexus::Nexus<nexus::Work> {");
        self.line("        let origin_route = self.origin_route();");
        self.line("        NexusWork::from(self.into_root()).with_origin_route(origin_route)");
        self.line("    }");
        self.line("}");
        self.blank();
        let _ = type_name;
    }

    fn can_emit_split_nexus_work_projection(&self, projection: &SplitSemaProjection<'_>) -> bool {
        self.enum_has_unique_payload_variant(projection.nexus_work, "SignalArrived", "Input")
            && self.enum_has_unique_payload_variant(
                projection.nexus_work,
                "SemaWriteCompleted",
                "SemaWriteOutput",
            )
            && self.enum_has_unique_payload_variant(
                projection.nexus_work,
                "SemaReadCompleted",
                "SemaReadOutput",
            )
            && self.enum_has_unique_payload_variant(
                projection.nexus_action,
                "CommandSemaWrite",
                "SemaWriteInput",
            )
            && self.enum_has_unique_payload_variant(
                projection.nexus_action,
                "CommandSemaRead",
                "SemaReadInput",
            )
            && self.enum_has_unique_payload_variant(
                projection.nexus_action,
                "ReplyToSignal",
                "Output",
            )
            && self.all_payloads_project_to_one_of(
                projection.signal_input,
                projection.sema_write_input,
                projection.sema_read_input,
            )
            && self.all_payloads_project_to(projection.sema_write_output, projection.signal_output)
            && self.all_payloads_project_to(projection.sema_read_output, projection.signal_output)
    }

    fn all_payloads_project_to_one_of(
        &self,
        source: &RustEnum,
        first_target: &RustEnum,
        second_target: &RustEnum,
    ) -> bool {
        source.variants().iter().all(|variant| {
            self.exact_target_variant_for_source(variant, first_target)
                .is_some()
                || self
                    .exact_target_variant_for_source(variant, second_target)
                    .is_some()
                || matches!(
                    (
                        self.fallback_target_variant_for_source(variant, first_target),
                        self.fallback_target_variant_for_source(variant, second_target),
                    ),
                    (Some(_), None) | (None, Some(_))
                )
        })
    }

    fn all_payloads_project_to(&self, source: &RustEnum, target: &RustEnum) -> bool {
        source
            .variants()
            .iter()
            .all(|variant| self.target_variant_for_source(variant, target).is_some())
    }

    fn target_variant_for_source<'target>(
        &self,
        source_variant: &RustEnumVariant,
        target: &'target RustEnum,
    ) -> Option<&'target RustEnumVariant> {
        self.exact_target_variant_for_source(source_variant, target)
            .or_else(|| self.fallback_target_variant_for_source(source_variant, target))
    }

    fn exact_target_variant_for_source<'target>(
        &self,
        source_variant: &RustEnumVariant,
        target: &'target RustEnum,
    ) -> Option<&'target RustEnumVariant> {
        let payload_name = self.plain_payload_name(source_variant)?;
        target.variants().iter().find(|target_variant| {
            target_variant.name().as_str() == source_variant.name().as_str()
                && self.plain_payload_name(target_variant) == Some(payload_name)
        })
    }

    fn fallback_target_variant_for_source<'target>(
        &self,
        source_variant: &RustEnumVariant,
        target: &'target RustEnum,
    ) -> Option<&'target RustEnumVariant> {
        let payload_name = self.plain_payload_name(source_variant)?;
        self.unique_plain_payload_variants(target)
            .into_iter()
            .find(|target_variant| self.plain_payload_name(target_variant) == Some(payload_name))
    }

    fn enum_has_unique_payload_variant(
        &self,
        declaration: &RustEnum,
        variant_name: &str,
        payload_name: &str,
    ) -> bool {
        self.unique_plain_payload_variants(declaration)
            .iter()
            .any(|variant| {
                variant.name().as_str() == variant_name
                    && self.plain_payload_name(variant) == Some(payload_name)
            })
    }

    fn enum_has_variant_payload(
        &self,
        declaration: &RustEnum,
        variant_name: &str,
        payload_name: &str,
    ) -> bool {
        declaration.variants().iter().any(|variant| {
            variant.name().as_str() == variant_name
                && self.plain_payload_name(variant) == Some(payload_name)
        })
    }

    fn enum_has_variant_named(&self, declaration: &RustEnum, variant_name: &str) -> bool {
        declaration
            .variants()
            .iter()
            .any(|variant| variant.name().as_str() == variant_name)
    }

    fn variant_plain_payload_name(
        &self,
        declaration: &RustEnum,
        variant_name: &str,
    ) -> Option<String> {
        declaration
            .variants()
            .iter()
            .find(|variant| variant.name().as_str() == variant_name)
            .and_then(|variant| self.plain_payload_name(variant))
            .map(ToOwned::to_owned)
    }

    fn type_name_matches_plain_or_alias(
        &self,
        declarations: &[RustDeclaration],
        type_name: &str,
        expected_type_name: &str,
    ) -> bool {
        if type_name == expected_type_name {
            return true;
        }
        declarations.iter().any(|declaration| {
            declaration.name().as_str() == type_name
                && matches!(
                    declaration.value(),
                    RustTypeDeclaration::Alias(alias)
                        if matches!(
                            alias.reference(),
                            TypeReference::Plain(target) if target.as_str() == expected_type_name
                        )
                )
        })
    }

    fn nexus_runner_shape(&self, declarations: &[RustDeclaration]) -> Option<NexusRunnerShape> {
        let nexus_work = self.declaration_enum_named(declarations, "NexusWork")?;
        let nexus_action = self.declaration_enum_named(declarations, "NexusAction")?;
        let reply_type = self.variant_plain_payload_name(nexus_action, "ReplyToSignal")?;

        let sema_write_input_type =
            self.variant_plain_payload_name(nexus_action, "CommandSemaWrite");
        let sema_write_output_type =
            self.variant_plain_payload_name(nexus_work, "SemaWriteCompleted");
        let sema_read_input_type = self.variant_plain_payload_name(nexus_action, "CommandSemaRead");
        let sema_read_output_type =
            self.variant_plain_payload_name(nexus_work, "SemaReadCompleted");
        let effect_command_type = self.variant_plain_payload_name(nexus_action, "CommandEffect");
        let effect_result_type = self.variant_plain_payload_name(nexus_work, "EffectCompleted");
        let continue_type = self.variant_plain_payload_name(nexus_action, "Continue");
        let has_continue_variant = self.enum_has_variant_named(nexus_action, "Continue");
        let has_continue = continue_type.as_deref().is_some_and(|type_name| {
            self.type_name_matches_plain_or_alias(declarations, type_name, "NexusWork")
        });

        if self.enum_has_variant_named(nexus_action, "CommandSemaWrite")
            != sema_write_input_type.is_some()
            || self.enum_has_variant_named(nexus_work, "SemaWriteCompleted")
                != sema_write_output_type.is_some()
            || self.enum_has_variant_named(nexus_action, "CommandSemaRead")
                != sema_read_input_type.is_some()
            || self.enum_has_variant_named(nexus_work, "SemaReadCompleted")
                != sema_read_output_type.is_some()
            || self.enum_has_variant_named(nexus_action, "CommandEffect")
                != effect_command_type.is_some()
            || self.enum_has_variant_named(nexus_work, "EffectCompleted")
                != effect_result_type.is_some()
            || has_continue_variant != has_continue
        {
            return None;
        }

        if sema_write_input_type.is_some() != sema_write_output_type.is_some()
            || sema_read_input_type.is_some() != sema_read_output_type.is_some()
            || effect_command_type.is_some() != effect_result_type.is_some()
        {
            return None;
        }

        let recognized_action_variants = nexus_action.variants().iter().all(|variant| {
            matches!(
                variant.name().as_str(),
                "ReplyToSignal"
                    | "CommandSemaWrite"
                    | "CommandSemaRead"
                    | "CommandEffect"
                    | "Continue"
            )
        });

        if !recognized_action_variants {
            return None;
        }

        Some(NexusRunnerShape {
            reply_type,
            sema_write_input_type,
            sema_write_output_type,
            sema_read_input_type,
            sema_read_output_type,
            effect_command_type,
            effect_result_type,
            has_continue,
        })
    }

    fn emit_upgrade_support(&mut self) {
        self.line("pub trait UpgradeFrom<Previous>: Sized {");
        self.line("    type Error;");
        self.blank();
        self.line("    fn upgrade_from(previous: Previous) -> Result<Self, Self::Error>;");
        self.line("}");
        self.blank();
        self.line("pub trait AcceptPrevious<Previous>: UpgradeFrom<Previous> {");
        self.line("    fn accept_previous(previous: Previous) -> Result<Self, Self::Error> {");
        self.line("        Self::upgrade_from(previous)");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line("impl<Current, Previous> AcceptPrevious<Previous> for Current where Current: UpgradeFrom<Previous> {}");
    }

    fn emits_signal_engine_support(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> bool {
        if !self.has_root_enum(root_enums, "Input") || !self.has_root_enum(root_enums, "Output") {
            return false;
        }
        if matches!(self.target, RustEmissionTarget::SignalRuntime) {
            return true;
        }
        self.runtime_planes().emits_signal()
            && self.has_type(declarations, "NexusWork")
            && self.has_type(declarations, "NexusAction")
    }

    fn emits_concrete_signal_engine_support(&self, declarations: &[RustDeclaration]) -> bool {
        self.runtime_planes().emits_nexus()
            && self.has_type(declarations, "NexusWork")
            && self.has_type(declarations, "NexusAction")
    }

    fn emits_nexus_engine_support(&self, declarations: &[RustDeclaration]) -> bool {
        self.runtime_planes().emits_nexus()
            && self.has_type(declarations, "NexusWork")
            && self.has_type(declarations, "NexusAction")
    }

    fn sema_write_input_type_name(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> Option<&'static str> {
        if self.has_type(declarations, "WriteInput") || self.has_root_enum(root_enums, "WriteInput")
        {
            Some("WriteInput")
        } else if self.has_type(declarations, "SemaWriteInput") {
            Some("SemaWriteInput")
        } else {
            None
        }
    }

    fn sema_write_output_type_name(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> Option<&'static str> {
        if self.has_type(declarations, "WriteOutput")
            || self.has_root_enum(root_enums, "WriteOutput")
        {
            Some("WriteOutput")
        } else if self.has_type(declarations, "SemaWriteOutput") {
            Some("SemaWriteOutput")
        } else {
            None
        }
    }

    fn sema_read_input_type_name(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> Option<&'static str> {
        if self.has_type(declarations, "ReadInput") || self.has_root_enum(root_enums, "ReadInput") {
            Some("ReadInput")
        } else if self.has_type(declarations, "SemaReadInput") {
            Some("SemaReadInput")
        } else {
            None
        }
    }

    fn sema_read_output_type_name(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> Option<&'static str> {
        if self.has_type(declarations, "ReadOutput") || self.has_root_enum(root_enums, "ReadOutput")
        {
            Some("ReadOutput")
        } else if self.has_type(declarations, "SemaReadOutput") {
            Some("SemaReadOutput")
        } else {
            None
        }
    }

    fn sema_write_input_root<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
        root_enums: &'schema [RustEnum],
    ) -> Option<&'schema RustEnum> {
        self.declaration_enum_named(declarations, "WriteInput")
            .or_else(|| self.root_enum_named(root_enums, "WriteInput"))
            .or_else(|| self.declaration_enum_named(declarations, "SemaWriteInput"))
    }

    fn sema_write_output_root<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
        root_enums: &'schema [RustEnum],
    ) -> Option<&'schema RustEnum> {
        self.declaration_enum_named(declarations, "WriteOutput")
            .or_else(|| self.root_enum_named(root_enums, "WriteOutput"))
            .or_else(|| self.declaration_enum_named(declarations, "SemaWriteOutput"))
    }

    fn sema_read_input_root<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
        root_enums: &'schema [RustEnum],
    ) -> Option<&'schema RustEnum> {
        self.declaration_enum_named(declarations, "ReadInput")
            .or_else(|| self.root_enum_named(root_enums, "ReadInput"))
            .or_else(|| self.declaration_enum_named(declarations, "SemaReadInput"))
    }

    fn sema_read_output_root<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
        root_enums: &'schema [RustEnum],
    ) -> Option<&'schema RustEnum> {
        self.declaration_enum_named(declarations, "ReadOutput")
            .or_else(|| self.root_enum_named(root_enums, "ReadOutput"))
            .or_else(|| self.declaration_enum_named(declarations, "SemaReadOutput"))
    }

    fn has_sema_write_roots(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> bool {
        self.sema_write_input_root(declarations, root_enums)
            .is_some()
            && self
                .sema_write_output_root(declarations, root_enums)
                .is_some()
    }

    fn has_sema_read_roots(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> bool {
        self.sema_read_input_root(declarations, root_enums)
            .is_some()
            && self
                .sema_read_output_root(declarations, root_enums)
                .is_some()
    }

    fn emits_sema_apply_support(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> bool {
        self.runtime_planes().emits_sema() && self.has_sema_write_roots(declarations, root_enums)
    }

    fn emits_sema_observe_support(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> bool {
        self.runtime_planes().emits_sema() && self.has_sema_read_roots(declarations, root_enums)
    }

    fn emit_schema_plane_trait_support(
        &mut self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) {
        let emits_signal_engine = self.emits_signal_engine_support(declarations, root_enums);
        let emits_concrete_signal_engine =
            emits_signal_engine && self.emits_concrete_signal_engine_support(declarations);
        let emits_nexus_engine = self.emits_nexus_engine_support(declarations);
        let emits_sema_apply = self.emits_sema_apply_support(declarations, root_enums);
        let emits_sema_observe = self.emits_sema_observe_support(declarations, root_enums);
        let emits_sema_engine = emits_sema_apply || emits_sema_observe;
        let nexus_runner_shape = if emits_nexus_engine {
            self.nexus_runner_shape(declarations)
        } else {
            None
        };

        if emits_signal_engine || emits_nexus_engine || emits_sema_engine {
            self.emit_actor_lifecycle_support();
        }

        if let Some(shape) = nexus_runner_shape.as_ref() {
            self.emit_nexus_runner_next_step_projection(shape);
        }

        if emits_signal_engine {
            self.line("pub trait SignalEngine {");
            if !emits_concrete_signal_engine {
                self.line("    type NexusInput;");
                self.line("    type NexusOutput;");
                self.blank();
            }
            self.line("    fn on_start(&mut self) -> Result<(), ActorStartFailure> {");
            self.line("        Ok(())");
            self.line("    }");
            self.line("    fn on_stop(&mut self) -> Result<(), ActorStopFailure> {");
            self.line("        Ok(())");
            self.line("    }");
            self.blank();
            self.line("    fn trace_signal_activation(&self, _object_name: SignalObjectName) {}");
            self.line("    fn trace_signal_admitted(&self) {");
            self.line("        self.trace_signal_activation(SignalObjectName::Admitted);");
            self.line("    }");
            self.line("    fn trace_signal_rejected(&self) {");
            self.line("        self.trace_signal_activation(SignalObjectName::Rejected);");
            self.line("    }");
            self.line("    fn trace_signal_triaged(&self) {");
            self.line("        self.trace_signal_activation(SignalObjectName::Triaged);");
            self.line("    }");
            self.line("    fn trace_signal_replied(&self) {");
            self.line("        self.trace_signal_activation(SignalObjectName::Replied);");
            self.line("    }");
            self.blank();
            if emits_concrete_signal_engine {
                self.line(
                    "    fn triage_inner(&self, input: signal::Signal<signal::Input>) -> nexus::Nexus<nexus::Work>;",
                );
                self.line(
                    "    fn reply_inner(&self, output: nexus::Nexus<nexus::Action>) -> signal::Signal<signal::Output>;",
                );
            } else {
                self.line(
                    "    fn triage_inner(&self, input: signal::Signal<signal::Input>) -> Self::NexusInput;",
                );
                self.line(
                    "    fn reply_inner(&self, output: Self::NexusOutput) -> signal::Signal<signal::Output>;",
                );
            }
            self.blank();
            if emits_concrete_signal_engine {
                self.line("    fn triage(&self, input: signal::Signal<signal::Input>) -> nexus::Nexus<nexus::Work> {");
            } else {
                self.line(
                    "    fn triage(&self, input: signal::Signal<signal::Input>) -> Self::NexusInput {",
                );
            }
            self.line("        let output = self.triage_inner(input);");
            self.line("        self.trace_signal_triaged();");
            self.line("        output");
            self.line("    }");
            self.blank();
            if emits_concrete_signal_engine {
                self.line("    fn reply(&self, output: nexus::Nexus<nexus::Action>) -> signal::Signal<signal::Output> {");
            } else {
                self.line(
                    "    fn reply(&self, output: Self::NexusOutput) -> signal::Signal<signal::Output> {",
                );
            }
            self.line("        let signal_output = self.reply_inner(output);");
            self.line("        self.trace_signal_replied();");
            self.line("        signal_output");
            self.line("    }");
            self.line("}");
            self.blank();
        }
        if emits_nexus_engine {
            self.line("pub trait NexusEngine {");
            self.line("    fn on_start(&mut self) -> Result<(), ActorStartFailure> {");
            self.line("        Ok(())");
            self.line("    }");
            self.line("    fn on_stop(&mut self) -> Result<(), ActorStopFailure> {");
            self.line("        Ok(())");
            self.line("    }");
            self.blank();
            self.line("    fn trace_nexus_activation(&self, _object_name: NexusObjectName) {}");
            self.line("    fn trace_nexus_entered(&self) {");
            self.line("        self.trace_nexus_activation(NexusObjectName::Entered);");
            self.line("    }");
            self.line("    fn trace_nexus_decided(&self) {");
            self.line("        self.trace_nexus_activation(NexusObjectName::Decided);");
            self.line("    }");
            self.blank();
            if let Some(shape) = nexus_runner_shape.as_ref() {
                self.line("    fn continuation_limit(&self) -> triad_runtime::ContinuationLimit {");
                self.line("        triad_runtime::ContinuationLimit::default()");
                self.line("    }");
                self.blank();
                if let (Some(input_type), Some(output_type)) = (
                    shape.sema_write_input_type.as_deref(),
                    shape.sema_write_output_type.as_deref(),
                ) {
                    self.line(format!(
                        "    fn apply_sema_write(&mut self, origin_route: OriginRoute, input: {input_type}) -> {output_type};"
                    ));
                }
                if let (Some(input_type), Some(output_type)) = (
                    shape.sema_read_input_type.as_deref(),
                    shape.sema_read_output_type.as_deref(),
                ) {
                    self.line(format!(
                        "    fn observe_sema_read(&self, origin_route: OriginRoute, input: {input_type}) -> {output_type};"
                    ));
                }
                if let (Some(input_type), Some(output_type)) = (
                    shape.effect_command_type.as_deref(),
                    shape.effect_result_type.as_deref(),
                ) {
                    self.line(format!(
                        "    fn run_effect(&mut self, input: {input_type}) -> {output_type};"
                    ));
                }
                self.line(format!(
                    "    fn budget_exhausted_reply(&self, exhausted: triad_runtime::ContinuationExhausted) -> {};",
                    shape.reply_type
                ));
                self.blank();
            }
            self.line("    fn decide(&mut self, input: nexus::Nexus<nexus::Work>) -> nexus::Nexus<nexus::Action>;");
            self.blank();
            self.line("    fn execute(&mut self, input: nexus::Nexus<nexus::Work>) -> nexus::Nexus<nexus::Action>");
            if nexus_runner_shape.is_some() {
                self.line("    where");
                self.line("        Self: Sized,");
            }
            self.line("    {");
            self.line("        self.trace_nexus_entered();");
            if nexus_runner_shape.is_some() {
                self.line("        let origin_route = input.origin_route();");
                self.line("        let first_work = input.into_root();");
                self.line(
                    "        let runner = triad_runtime::Runner::new(self.continuation_limit());",
                );
                self.line(
                    "        let mut runner_adapter = NexusRunnerAdapter::new(self, origin_route);",
                );
                self.line("        let reply = runner.drive(&mut runner_adapter, first_work);");
                self.line("        let output = NexusAction::reply_to_signal(reply).with_origin_route(origin_route);");
            } else {
                self.line("        let output = self.decide(input);");
            }
            self.line("        self.trace_nexus_decided();");
            self.line("        output");
            self.line("    }");
            self.line("}");
            self.blank();
            if let Some(shape) = nexus_runner_shape.as_ref() {
                self.emit_nexus_runner_adapter(shape);
            }
        }
        if emits_sema_engine {
            self.line("pub trait SemaEngine {");
            self.line("    fn on_start(&mut self) -> Result<(), ActorStartFailure> {");
            self.line("        Ok(())");
            self.line("    }");
            self.line("    fn on_stop(&mut self) -> Result<(), ActorStopFailure> {");
            self.line("        Ok(())");
            self.line("    }");
            self.blank();
            self.line("    fn trace_sema_activation(&self, _object_name: SemaObjectName) {}");
            if emits_sema_apply {
                self.line("    fn trace_sema_write_applied(&self) {");
                self.line("        self.trace_sema_activation(SemaObjectName::WriteApplied);");
                self.line("    }");
            }
            if emits_sema_observe {
                self.line("    fn trace_sema_read_observed(&self) {");
                self.line("        self.trace_sema_activation(SemaObjectName::ReadObserved);");
                self.line("    }");
            }
            self.blank();
            if emits_sema_apply {
                self.line("    fn apply_inner(&mut self, input: sema::Sema<sema::WriteInput>) -> sema::Sema<sema::WriteOutput>;");
            }
            if emits_sema_observe {
                self.line("    fn observe_inner(&self, input: sema::Sema<sema::ReadInput>) -> sema::Sema<sema::ReadOutput>;");
            }
            self.blank();
            if emits_sema_apply {
                self.line("    fn apply(&mut self, input: sema::Sema<sema::WriteInput>) -> sema::Sema<sema::WriteOutput> {");
                self.line("        let output = self.apply_inner(input);");
                self.line("        self.trace_sema_write_applied();");
                self.line("        output");
                self.line("    }");
                if emits_sema_observe {
                    self.blank();
                }
            }
            if emits_sema_observe {
                self.line("    fn observe(&self, input: sema::Sema<sema::ReadInput>) -> sema::Sema<sema::ReadOutput> {");
                self.line("        let output = self.observe_inner(input);");
                self.line("        self.trace_sema_read_observed();");
                self.line("        output");
                self.line("    }");
            }
            self.line("}");
            self.blank();
        }
    }

    fn emit_actor_lifecycle_support(&mut self) {
        self.line("#[derive(Clone, Debug, PartialEq, Eq)]");
        self.line("pub enum ActorStartFailure {");
        self.line("    ResourceBusy(String),");
        self.line("    ConfigurationInvalid(String),");
        self.line("}");
        self.blank();
        self.line("impl std::fmt::Display for ActorStartFailure {");
        self.line(
            "    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {",
        );
        self.line("        match self {");
        self.line("            Self::ResourceBusy(message) => write!(formatter, \"actor resource busy: {message}\"),");
        self.line("            Self::ConfigurationInvalid(message) => write!(formatter, \"actor configuration invalid: {message}\"),");
        self.line("        }");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line("impl std::error::Error for ActorStartFailure {}");
        self.blank();
        self.line("#[derive(Clone, Debug, PartialEq, Eq)]");
        self.line("pub enum ActorStopFailure {");
        self.line("    ResourceLocked(String),");
        self.line("    ChildStillRunning(String),");
        self.line("}");
        self.blank();
        self.line("impl std::fmt::Display for ActorStopFailure {");
        self.line(
            "    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {",
        );
        self.line("        match self {");
        self.line("            Self::ResourceLocked(message) => write!(formatter, \"actor resource locked: {message}\"),");
        self.line("            Self::ChildStillRunning(message) => write!(formatter, \"actor child still running: {message}\"),");
        self.line("        }");
        self.line("    }");
        self.line("}");
        self.blank();
        self.line("impl std::error::Error for ActorStopFailure {}");
        self.blank();
    }

    fn has_root_enum(&self, root_enums: &[RustEnum], type_name: &str) -> bool {
        root_enums
            .iter()
            .any(|declaration| declaration.name().as_str() == type_name)
    }

    fn has_type(&self, declarations: &[RustDeclaration], type_name: &str) -> bool {
        declarations
            .iter()
            .any(|declaration| declaration.name().as_str() == type_name)
    }

    fn root_enum_named<'root>(
        &self,
        root_enums: &'root [RustEnum],
        type_name: &str,
    ) -> Option<&'root RustEnum> {
        root_enums
            .iter()
            .find(|declaration| declaration.name().as_str() == type_name)
    }

    fn declaration_enum_named<'declaration>(
        &self,
        declarations: &'declaration [RustDeclaration],
        type_name: &str,
    ) -> Option<&'declaration RustEnum> {
        declarations
            .iter()
            .find(|declaration| declaration.name().as_str() == type_name)
            .and_then(|declaration| match declaration.value() {
                RustTypeDeclaration::Enum(value) => Some(value),
                _ => None,
            })
    }

    /// The Rust type for a reference.
    ///
    /// Scalar leaves map to emitted scalar aliases. A plain declared-name
    /// leaf maps to its local or imported type name. The
    /// collection variants recurse: `Vector` → `Vec<inner>`, `Map` →
    /// `BTreeMap<key, value>` (the `KeyValue` keyword), `Optional` →
    /// `Option<inner>`. `BTreeMap` is written fully-qualified so no
    /// `use` is emitted and the ordering is deterministic (rkyv + NOTA
    /// round-trips need a stable key order).
    fn rust_type(&self, reference: &TypeReference) -> String {
        match reference {
            TypeReference::String => "String".to_owned(),
            TypeReference::Integer => "Integer".to_owned(),
            TypeReference::Boolean => "Boolean".to_owned(),
            TypeReference::Path => "Path".to_owned(),
            TypeReference::Plain(name) => name.as_str().to_owned(),
            TypeReference::Vector(inner) => format!("Vec<{}>", self.rust_type(inner)),
            TypeReference::Map(key, value) => format!(
                "std::collections::BTreeMap<{}, {}>",
                self.rust_type(key),
                self.rust_type(value)
            ),
            TypeReference::Optional(inner) => format!("Option<{}>", self.rust_type(inner)),
        }
    }

    fn constant_name(&self, name: &Name) -> String {
        let mut output = String::new();
        for (index, character) in name.as_str().chars().enumerate() {
            if character.is_ascii_uppercase() {
                if index > 0 {
                    output.push('_');
                }
                output.push(character);
            } else if character == '-' {
                output.push('_');
            } else {
                output.push(character.to_ascii_uppercase());
            }
        }
        output
    }

    fn rust_method_name(&self, name: &Name) -> String {
        let method_name = name.field_name();
        if RustKeyword::new(&method_name).is_reserved() {
            format!("r#{method_name}")
        } else {
            method_name
        }
    }
}

struct RustKeyword<'name> {
    name: &'name str,
}

impl<'name> RustKeyword<'name> {
    fn new(name: &'name str) -> Self {
        Self { name }
    }

    fn is_reserved(&self) -> bool {
        matches!(
            self.name,
            "as" | "break"
                | "const"
                | "continue"
                | "crate"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "Self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "type"
                | "unsafe"
                | "use"
                | "where"
                | "while"
                | "async"
                | "await"
                | "dyn"
                | "abstract"
                | "become"
                | "box"
                | "do"
                | "final"
                | "macro"
                | "override"
                | "priv"
                | "typeof"
                | "unsized"
                | "virtual"
                | "yield"
                | "try"
        )
    }
}
