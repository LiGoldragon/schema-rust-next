use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use schema_next::{
    AliasDeclaration, Declaration, EnumDeclaration, EnumVariant, FieldDeclaration, ImportResolver,
    Name, NewtypeDeclaration, ResolvedImport, Schema, SchemaEngine, SchemaError, SchemaIdentity,
    SchemaSource, StreamDeclaration, StructDeclaration, TypeDeclaration, TypeReference, Visibility,
};

pub mod build;
pub mod daemon_emit;
pub mod migration;
pub use daemon_emit::{
    DaemonModule, MetaListenerTier, NexusDaemonShape, SocketModeBits, WorkingListenerTier,
};
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
        let context = RustLoweringContext::from_emitter(emitter);
        self.lower_to_rust(&context)
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

#[derive(Clone, Debug)]
pub struct RustLoweringContext {
    generator_name: String,
    options: RustEmissionOptions,
}

impl RustLoweringContext {
    pub fn new(generator_name: impl Into<String>, options: RustEmissionOptions) -> Self {
        Self {
            generator_name: generator_name.into(),
            options,
        }
    }

    pub fn from_emitter(emitter: &RustEmitter) -> Self {
        Self::new(emitter.generator_name, emitter.options.clone())
    }

    fn generator_name(&self) -> &str {
        &self.generator_name
    }

    fn options(&self) -> RustEmissionOptions {
        self.options.clone()
    }
}

pub trait LowerToRust<Target> {
    fn lower_to_rust(&self, context: &RustLoweringContext) -> Target;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustModule {
    file_path: String,
    generator_name: String,
    scalar_aliases: Vec<RustScalarAlias>,
    imports: Vec<RustImport>,
    declarations: Vec<RustDeclaration>,
    root_enums: Vec<RustEnum>,
    streams: Vec<StreamDeclaration>,
    support: RustSupportModel,
    options: RustEmissionOptions,
}

impl RustModule {
    pub fn from_schema(
        schema: &Schema,
        generator_name: impl Into<String>,
        options: RustEmissionOptions,
    ) -> Self {
        let context = RustLoweringContext::new(generator_name, options);
        schema.lower_to_rust(&context)
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

    pub fn streams(&self) -> &[StreamDeclaration] {
        &self.streams
    }

    pub fn declaration_named(&self, name: &str) -> Option<&RustDeclaration> {
        self.declarations
            .iter()
            .find(|declaration| declaration.name().as_str() == name)
    }

    pub fn render(&self) -> RustCode {
        let mut writer = RustModuleRenderer::new(self.options.clone());
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

        if writer.emits_short_headers() {
            writer.emit_short_headers(&self.root_enums);
            writer.blank();
        }
        if writer.emits_wire_frame() {
            writer.emit_signal_frame_codec(&self.root_enums);
        }
        if writer.emits_signal() {
            if let Some(event_payload) =
                writer.streaming_event_payload(&self.root_enums, &self.streams)
            {
                writer.emit_signal_frame_streaming_support(event_payload);
                writer.blank();
            }
        }
        if writer.emits_runtime_support() {
            writer.emit_plane_route_support(&self.declarations);
            writer.emit_trace_support(&self.declarations, &self.root_enums);
            writer.emit_mail_event_support(&self.root_enums);
            writer.emit_plane_namespaces(&self.declarations, &self.root_enums);
            writer.emit_plane_projection_support(&self.declarations, &self.root_enums);
            writer.emit_runtime_role_trait_impls(&self.declarations, &self.root_enums);
            writer.emit_schema_plane_trait_support(&self.declarations, &self.root_enums);
            writer.emit_upgrade_support();
        }
        RustCode(writer.finish())
    }
}

impl LowerToRust<RustModule> for Schema {
    fn lower_to_rust(&self, context: &RustLoweringContext) -> RustModule {
        let declarations = self
            .namespace()
            .iter()
            .map(|declaration| declaration.lower_to_rust(context))
            .collect::<Vec<_>>();
        let root_enums = self
            .input_and_output()
            .into_iter()
            .map(|root| root.lower_to_rust(context))
            .collect::<Vec<_>>();
        RustModule {
            file_path: RustModulePath::new(self.identity().component().clone()).to_file_path(),
            generator_name: context.generator_name().to_owned(),
            scalar_aliases: RustScalarAlias::default_aliases(),
            imports: self
                .resolved_imports()
                .iter()
                .map(|import| import.lower_to_rust(context))
                .collect(),
            declarations,
            root_enums,
            streams: self.streams().to_vec(),
            support: <Self as LowerToRust<RustSupportModel>>::lower_to_rust(self, context),
            options: context.options(),
        }
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

    /// Whether this target faces the wire and therefore needs the basic
    /// signal-frame codec (route enums, `short_header`,
    /// `encode_signal_frame` / `decode_signal_frame`, `SignalFrameError`).
    ///
    /// A separately-generated `WireContract` crate IS the wire framing —
    /// peers and the owning daemon import it and call the codec on it — so
    /// it carries the codec even though it emits no daemon-side runtime
    /// planes. `SignalRuntime` and `ComponentRuntime` (bootstrap) also face
    /// the wire. The `NexusRuntime` / `SemaRuntime` internal planes never
    /// touch the wire and must not receive frame codec code.
    fn emits_wire_frame(self) -> bool {
        match self {
            Self::WireContract | Self::SignalRuntime | Self::ComponentRuntime => true,
            Self::NexusRuntime | Self::SemaRuntime => false,
        }
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

    /// The planes this set emits, in canonical signal-nexus-sema order.
    fn active_planes(self) -> Vec<Plane> {
        let mut planes = Vec::new();
        if self.signal {
            planes.push(Plane::Signal);
        }
        if self.nexus {
            planes.push(Plane::Nexus);
        }
        if self.sema {
            planes.push(Plane::Sema);
        }
        planes
    }
}

/// Runtime plane axis. This owns only plane-intrinsic names; target
/// selection and schema-presence checks stay on the emitter/writer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Plane {
    Signal,
    Nexus,
    Sema,
}

impl Plane {
    fn module_name(&self) -> &'static str {
        match self {
            Self::Signal => "signal",
            Self::Nexus => "nexus",
            Self::Sema => "sema",
        }
    }

    fn wrapper_name(&self) -> &'static str {
        match self {
            Self::Signal => "Signal",
            Self::Nexus => "Nexus",
            Self::Sema => "Sema",
        }
    }

    fn alias_names(&self) -> &'static [&'static str] {
        match self {
            Self::Signal => &["Input", "Output"],
            Self::Nexus => &["Work", "Action"],
            Self::Sema => &["WriteInput", "WriteOutput", "ReadInput", "ReadOutput"],
        }
    }

    fn canonical_source_type_names(&self) -> &'static [&'static str] {
        match self {
            Self::Signal => &["Input", "Output"],
            Self::Nexus => &["NexusWork", "NexusAction"],
            Self::Sema => &[
                "SemaWriteInput",
                "SemaWriteOutput",
                "SemaReadInput",
                "SemaReadOutput",
            ],
        }
    }

    fn engine_trait_name(&self) -> &'static str {
        match self {
            Self::Signal => "SignalEngine",
            Self::Nexus => "NexusEngine",
            Self::Sema => "SemaEngine",
        }
    }

    fn trace_enum_name(&self) -> &'static str {
        match self {
            Self::Signal => "SignalObjectName",
            Self::Nexus => "NexusObjectName",
            Self::Sema => "SemaObjectName",
        }
    }

    fn trace_activation_method_name(&self) -> &'static str {
        match self {
            Self::Signal => "trace_signal_activation",
            Self::Nexus => "trace_nexus_activation",
            Self::Sema => "trace_sema_activation",
        }
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
    pub fn use_item(&self) -> &str {
        &self.use_item
    }
}

impl LowerToRust<RustImport> for ResolvedImport {
    fn lower_to_rust(&self, _context: &RustLoweringContext) -> RustImport {
        RustImport {
            use_item: self.use_item(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustDeclaration {
    visibility: Visibility,
    name: Name,
    value: RustTypeDeclaration,
}

impl RustDeclaration {
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

impl LowerToRust<RustDeclaration> for Declaration {
    fn lower_to_rust(&self, context: &RustLoweringContext) -> RustDeclaration {
        RustDeclaration {
            visibility: self.visibility(),
            name: self.name().clone(),
            value: self.value().lower_to_rust(context),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustTypeDeclaration {
    Alias(RustAlias),
    Struct(RustStruct),
    Enum(RustEnum),
    Newtype(RustNewtype),
}

impl LowerToRust<RustTypeDeclaration> for TypeDeclaration {
    fn lower_to_rust(&self, context: &RustLoweringContext) -> RustTypeDeclaration {
        match self {
            TypeDeclaration::Alias(declaration) => {
                RustTypeDeclaration::Alias(declaration.lower_to_rust(context))
            }
            TypeDeclaration::Struct(declaration) => {
                RustTypeDeclaration::Struct(declaration.lower_to_rust(context))
            }
            TypeDeclaration::Enum(declaration) => {
                RustTypeDeclaration::Enum(declaration.lower_to_rust(context))
            }
            TypeDeclaration::Newtype(declaration) => {
                RustTypeDeclaration::Newtype(declaration.lower_to_rust(context))
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
    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn reference(&self) -> &TypeReference {
        &self.reference
    }
}

impl LowerToRust<RustAlias> for AliasDeclaration {
    fn lower_to_rust(&self, _context: &RustLoweringContext) -> RustAlias {
        RustAlias {
            name: self.name.clone(),
            reference: self.reference.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustNewtype {
    name: Name,
    reference: TypeReference,
}

impl RustNewtype {
    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn reference(&self) -> &TypeReference {
        &self.reference
    }
}

impl LowerToRust<RustNewtype> for NewtypeDeclaration {
    fn lower_to_rust(&self, _context: &RustLoweringContext) -> RustNewtype {
        RustNewtype {
            name: self.name.clone(),
            reference: self.reference.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustStruct {
    name: Name,
    fields: Vec<RustField>,
}

impl RustStruct {
    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn fields(&self) -> &[RustField] {
        &self.fields
    }
}

impl LowerToRust<RustStruct> for StructDeclaration {
    fn lower_to_rust(&self, context: &RustLoweringContext) -> RustStruct {
        RustStruct {
            name: self.name.clone(),
            fields: self
                .fields
                .iter()
                .map(|field| field.lower_to_rust(context))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustField {
    name: Name,
    reference: TypeReference,
}

impl RustField {
    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn reference(&self) -> &TypeReference {
        &self.reference
    }
}

impl LowerToRust<RustField> for FieldDeclaration {
    fn lower_to_rust(&self, _context: &RustLoweringContext) -> RustField {
        RustField {
            name: self.name.clone(),
            reference: self.reference.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustEnum {
    name: Name,
    variants: Vec<RustEnumVariant>,
}

impl RustEnum {
    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn variants(&self) -> &[RustEnumVariant] {
        &self.variants
    }
}

impl LowerToRust<RustEnum> for EnumDeclaration {
    fn lower_to_rust(&self, context: &RustLoweringContext) -> RustEnum {
        RustEnum {
            name: self.name.clone(),
            variants: self
                .variants
                .iter()
                .map(|variant| variant.lower_to_rust(context))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustEnumVariant {
    name: Name,
    payload: Option<TypeReference>,
}

impl RustEnumVariant {
    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn payload(&self) -> Option<&TypeReference> {
        self.payload.as_ref()
    }
}

impl LowerToRust<RustEnumVariant> for EnumVariant {
    fn lower_to_rust(&self, _context: &RustLoweringContext) -> RustEnumVariant {
        RustEnumVariant {
            name: self.name.clone(),
            payload: self.payload.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RustSupportModel {
    map_key_type_names: Vec<String>,
    private_type_names: Vec<String>,
}

impl RustSupportModel {
    fn map_key_type_names(&self) -> &[String] {
        &self.map_key_type_names
    }

    fn private_type_names(&self) -> &[String] {
        &self.private_type_names
    }
}

impl LowerToRust<RustSupportModel> for Schema {
    fn lower_to_rust(&self, _context: &RustLoweringContext) -> RustSupportModel {
        RustSupportModel {
            map_key_type_names: CollectionScan::new(self).map_key_type_names(),
            private_type_names: self
                .namespace()
                .iter()
                .filter(|declaration| declaration.is_private())
                .map(|declaration| declaration.name().as_str().to_owned())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct RustRenderContext {
    map_key_type_names: Vec<String>,
    private_type_names: Vec<String>,
    nota_surface: NotaSurface,
}

impl RustRenderContext {
    fn new(
        map_key_type_names: Vec<String>,
        private_type_names: Vec<String>,
        nota_surface: NotaSurface,
    ) -> Self {
        Self {
            map_key_type_names,
            private_type_names,
            nota_surface,
        }
    }

    fn data_type_attributes(&self, type_name: &Name) -> Vec<TokenStream> {
        self.derive_attributes(
            false,
            self.map_key_type_names
                .iter()
                .any(|name| name == type_name.as_str()),
        )
    }

    fn root_data_type_attributes(&self) -> Vec<TokenStream> {
        self.derive_attributes(false, false)
    }

    fn derive_attributes(&self, includes_copy: bool, includes_ordering: bool) -> Vec<TokenStream> {
        let mut attributes = Vec::new();
        if let NotaSurface::FeatureGated { feature } = &self.nota_surface {
            attributes.push(quote! {
                #[cfg_attr(feature = #feature, derive(nota_next::NotaDecode, nota_next::NotaEncode))]
            });
        }
        let nota_derives = if self.nota_surface.includes_nota_in_derive() {
            quote! { nota_next::NotaDecode, nota_next::NotaEncode, }
        } else {
            TokenStream::new()
        };
        let copy_derive = if includes_copy {
            quote! { Copy, }
        } else {
            TokenStream::new()
        };
        let ordering_derives = if includes_ordering {
            quote! { PartialOrd, Ord, }
        } else {
            TokenStream::new()
        };
        attributes.push(quote! {
            #[derive(
                #nota_derives
                rkyv::Archive,
                rkyv::Serialize,
                rkyv::Deserialize,
                Clone,
                #copy_derive
                Debug,
                PartialEq,
                Eq,
                #ordering_derives
            )]
        });
        if includes_ordering {
            attributes.push(quote! {
                #[rkyv(derive(PartialEq, Eq, PartialOrd, Ord))]
            });
        }
        attributes
    }

    fn visibility_tokens(&self, visibility: Visibility) -> TokenStream {
        match visibility {
            Visibility::Public => quote! { pub },
            Visibility::Private => quote! { pub(crate) },
        }
    }

    /// The `#[cfg(feature = "<feature>")]` gate applied to NOTA-only
    /// items, as a token attribute. `None` when the surface is always
    /// enabled or disabled (no gate line).
    fn nota_feature_gate(&self) -> Option<TokenStream> {
        match &self.nota_surface {
            NotaSurface::AlwaysEnabled | NotaSurface::Disabled => None,
            NotaSurface::FeatureGated { feature } => Some(quote! {
                #[cfg(feature = #feature)]
            }),
        }
    }

    fn field_visibility_tokens(
        &self,
        visibility: Visibility,
        reference: &TypeReference,
    ) -> TokenStream {
        if visibility == Visibility::Public && self.references_private_type(reference) {
            quote! { pub(crate) }
        } else {
            self.visibility_tokens(visibility)
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
}

#[derive(Clone, Copy)]
struct RustIdentifier<'name> {
    name: &'name str,
}

impl<'name> RustIdentifier<'name> {
    fn new(name: &'name str) -> Self {
        Self { name }
    }

    fn ident(&self) -> Ident {
        if RustKeyword::new(self.name).is_reserved() {
            Ident::new_raw(self.name, Span::call_site())
        } else {
            Ident::new(self.name, Span::call_site())
        }
    }
}

impl ToTokens for RustIdentifier<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.ident().to_tokens(tokens);
    }
}

struct RustTypeReferenceTokens<'reference> {
    reference: &'reference TypeReference,
}

impl<'reference> RustTypeReferenceTokens<'reference> {
    fn new(reference: &'reference TypeReference) -> Self {
        Self { reference }
    }
}

impl ToTokens for RustTypeReferenceTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.reference {
            TypeReference::String => quote! { String }.to_tokens(tokens),
            TypeReference::Integer => quote! { Integer }.to_tokens(tokens),
            TypeReference::Boolean => quote! { Boolean }.to_tokens(tokens),
            TypeReference::Path => quote! { Path }.to_tokens(tokens),
            TypeReference::Plain(name) => RustIdentifier::new(name.as_str()).to_tokens(tokens),
            TypeReference::Vector(inner) => {
                let inner = Self::new(inner);
                quote! { Vec<#inner> }.to_tokens(tokens);
            }
            TypeReference::Map(key, value) => {
                let key = Self::new(key);
                let value = Self::new(value);
                quote! { std::collections::BTreeMap<#key, #value> }.to_tokens(tokens);
            }
            TypeReference::Optional(inner) => {
                let inner = Self::new(inner);
                quote! { Option<#inner> }.to_tokens(tokens);
            }
        }
    }
}

struct RustTypeTokens<'source> {
    source: &'source str,
}

impl<'source> RustTypeTokens<'source> {
    fn new(source: &'source str) -> Self {
        Self { source }
    }
}

impl ToTokens for RustTypeTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        syn::parse_str::<syn::Type>(self.source)
            .expect("generated Rust type token parses")
            .to_tokens(tokens);
    }
}

/// Renders a scalar alias declaration (`pub type Name = path::To::Type;`)
/// as Rust tokens. The alias name is an identifier; the target is an
/// already-resolved Rust type path parsed into a type token.
struct RustScalarAliasTokens<'alias> {
    alias: &'alias RustScalarAlias,
}

impl<'alias> RustScalarAliasTokens<'alias> {
    fn new(alias: &'alias RustScalarAlias) -> Self {
        Self { alias }
    }
}

impl ToTokens for RustScalarAliasTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = RustIdentifier::new(self.alias.name());
        let rust_type = RustTypeTokens::new(self.alias.rust_type());
        quote! {
            pub type #name = #rust_type;
        }
        .to_tokens(tokens);
    }
}

/// The `<Name>Route` enum derived from a source enum: one unit variant per
/// source variant, carrying the copy-data derive set. Owns the source enum
/// and the render context that supplies the derives.
struct RouteEnumTokens<'enum_source, 'context> {
    source: &'enum_source RustEnum,
    context: &'context RustRenderContext,
}

impl<'enum_source, 'context> RouteEnumTokens<'enum_source, 'context> {
    fn new(source: &'enum_source RustEnum, context: &'context RustRenderContext) -> Self {
        Self { source, context }
    }
}

impl ToTokens for RouteEnumTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attributes = self.context.derive_attributes(true, false);
        let route_name = RouteName::new(self.source.name()).ident();
        let variants = self
            .source
            .variants()
            .iter()
            .map(|variant| RustIdentifier::new(variant.name().as_str()).ident());
        quote! {
            #(#attributes)*
            pub enum #route_name {
                #(#variants,)*
            }
        }
        .to_tokens(tokens);
    }
}

/// The `route(&self) -> <Name>Route` projection on a source enum: a `match`
/// that maps each source variant to its route variant, ignoring any payload.
struct RouteImplTokens<'enum_source> {
    source: &'enum_source RustEnum,
}

impl<'enum_source> RouteImplTokens<'enum_source> {
    fn new(source: &'enum_source RustEnum) -> Self {
        Self { source }
    }
}

impl ToTokens for RouteImplTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = RustIdentifier::new(self.source.name().as_str()).ident();
        let route_name = RouteName::new(self.source.name()).ident();
        let arms = self
            .source
            .variants()
            .iter()
            .map(|variant| RouteMatchArm::new(variant, &route_name));
        quote! {
            impl #name {
                pub fn route(&self) -> #route_name {
                    match self {
                        #(#arms)*
                    }
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// One `Self::Variant(_) => <Route>::Variant,` arm of a `route` match. Owns
/// the source variant and the route enum name; the payload presence decides
/// whether the pattern binds `(_)`.
struct RouteMatchArm<'variant, 'route> {
    variant: &'variant RustEnumVariant,
    route_name: &'route Ident,
}

impl<'variant, 'route> RouteMatchArm<'variant, 'route> {
    fn new(variant: &'variant RustEnumVariant, route_name: &'route Ident) -> Self {
        Self {
            variant,
            route_name,
        }
    }
}

impl ToTokens for RouteMatchArm<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let variant = RustIdentifier::new(self.variant.name().as_str()).ident();
        let route_name = self.route_name;
        if self.variant.payload().is_some() {
            quote! { Self::#variant(_) => #route_name::#variant, }.to_tokens(tokens);
        } else {
            quote! { Self::#variant => #route_name::#variant, }.to_tokens(tokens);
        }
    }
}

/// The full signal-frame binary codec impl for one root enum: `route`,
/// `short_header`, `route_from_short_header`, `encode_signal_frame`, and
/// `decode_signal_frame`. Owns the root enum and renders the per-variant
/// triage arms plus the fixed encode/decode bodies. The route and
/// short-header arms are derived from the same `ShortHeader` noun the
/// `short_header` module uses, so the constant names always agree.
struct SignalFrameImplTokens<'enum_source> {
    root_enum: &'enum_source RustEnum,
}

impl<'enum_source> SignalFrameImplTokens<'enum_source> {
    fn new(root_enum: &'enum_source RustEnum) -> Self {
        Self { root_enum }
    }
}

impl ToTokens for SignalFrameImplTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = RustIdentifier::new(self.root_enum.name().as_str()).ident();
        let route_name = RouteName::new(self.root_enum.name()).ident();
        let root_name_literal = self.root_enum.name().as_str();

        let route_arms = self
            .root_enum
            .variants()
            .iter()
            .map(|variant| RouteMatchArm::new(variant, &route_name));

        let short_header_arms = self.root_enum.variants().iter().enumerate().map(
            |(variant_index, variant)| {
                let constant = ShortHeader::new(
                    self.root_enum.name(),
                    variant.name(),
                    0,
                    variant_index,
                )
                .constant_identifier();
                let variant_ident = RustIdentifier::new(variant.name().as_str()).ident();
                if variant.payload().is_some() {
                    quote! { Self::#variant_ident(_) => short_header::#constant, }
                } else {
                    quote! { Self::#variant_ident => short_header::#constant, }
                }
            },
        );

        let route_from_header_arms = self.root_enum.variants().iter().enumerate().map(
            |(variant_index, variant)| {
                let constant = ShortHeader::new(
                    self.root_enum.name(),
                    variant.name(),
                    0,
                    variant_index,
                )
                .constant_identifier();
                let variant_ident = RustIdentifier::new(variant.name().as_str()).ident();
                quote! { short_header::#constant => Ok(#route_name::#variant_ident), }
            },
        );

        quote! {
            impl #name {
                pub fn route(&self) -> #route_name {
                    match self {
                        #(#route_arms)*
                    }
                }
                pub fn short_header(&self) -> u64 {
                    match self {
                        #(#short_header_arms)*
                    }
                }
                pub fn route_from_short_header(header: u64) -> Result<#route_name, SignalFrameError> {
                    match header {
                        #(#route_from_header_arms)*
                        _ => Err(SignalFrameError::UnknownHeader { root_enum: #root_name_literal, header }),
                    }
                }
                pub fn encode_signal_frame(&self) -> Result<Vec<u8>, SignalFrameError> {
                    let archive = rkyv::to_bytes::<rkyv::rancor::Error>(self)
                        .map_err(|_| SignalFrameError::ArchiveEncode)?;
                    let mut frame = Vec::with_capacity(SIGNAL_SHORT_HEADER_BYTE_COUNT + archive.len());
                    frame.extend_from_slice(&self.short_header().to_le_bytes());
                    frame.extend_from_slice(&archive);
                    Ok(frame)
                }
                pub fn decode_signal_frame(frame: &[u8]) -> Result<(#route_name, Self), SignalFrameError> {
                    if frame.len() < SIGNAL_SHORT_HEADER_BYTE_COUNT {
                        return Err(SignalFrameError::FrameTooShort { found: frame.len() });
                    }
                    let mut header_bytes = [0_u8; SIGNAL_SHORT_HEADER_BYTE_COUNT];
                    header_bytes.copy_from_slice(&frame[..SIGNAL_SHORT_HEADER_BYTE_COUNT]);
                    let header = u64::from_le_bytes(header_bytes);
                    let route = Self::route_from_short_header(header)?;
                    let value = rkyv::from_bytes::<Self, rkyv::rancor::Error>(&frame[SIGNAL_SHORT_HEADER_BYTE_COUNT..])
                        .map_err(|_| SignalFrameError::ArchiveDecode)?;
                    let expected = value.short_header();
                    if expected != header {
                        return Err(SignalFrameError::HeaderMismatch { expected, found: header });
                    }
                    Ok((route, value))
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// The `<Name>Route` type identifier derived from a schema enum name.
struct RouteName<'name> {
    name: &'name Name,
}

impl<'name> RouteName<'name> {
    fn new(name: &'name Name) -> Self {
        Self { name }
    }

    fn ident(&self) -> Ident {
        Ident::new(&format!("{}Route", self.name.as_str()), Span::call_site())
    }
}

/// A single short-header constant: the per-root-per-variant route triage
/// key. It owns the root name, the variant name, and the two positional
/// indices, and renders its own SCREAMING_SNAKE constant identifier plus
/// the packed `u64` value. Shared by the `short_header` module emission and
/// the per-root frame codec so the constant name and value are computed in
/// exactly one place.
struct ShortHeader<'enums> {
    root_name: &'enums Name,
    variant_name: &'enums Name,
    root_index: usize,
    variant_index: usize,
}

impl<'enums> ShortHeader<'enums> {
    fn new(
        root_name: &'enums Name,
        variant_name: &'enums Name,
        root_index: usize,
        variant_index: usize,
    ) -> Self {
        Self {
            root_name,
            variant_name,
            root_index,
            variant_index,
        }
    }

    fn constant_identifier(&self) -> Ident {
        let name = format!(
            "{}_{}",
            ScreamingName::new(self.root_name).screaming(),
            ScreamingName::new(self.variant_name).screaming()
        );
        Ident::new(&name, Span::call_site())
    }

    fn value(&self) -> u64 {
        ((self.root_index as u64) << 56) | ((self.variant_index as u64) << 48)
    }

    fn value_literal(&self) -> syn::LitInt {
        syn::LitInt::new(&format!("0x{:016X}", self.value()), Span::call_site())
    }
}

/// Renders the `pub mod short_header { ... }` module: one `pub const NAME:
/// u64 = 0x...;` per root-enum variant across every root, with values packed
/// by declaration order.
struct ShortHeaderModuleTokens<'enums> {
    root_enums: &'enums [RustEnum],
}

impl<'enums> ShortHeaderModuleTokens<'enums> {
    fn new(root_enums: &'enums [RustEnum]) -> Self {
        Self { root_enums }
    }
}

impl ToTokens for ShortHeaderModuleTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let constants = self.root_enums.iter().enumerate().flat_map(
            |(root_index, root_enum)| {
                root_enum
                    .variants()
                    .iter()
                    .enumerate()
                    .map(move |(variant_index, variant)| {
                        let header = ShortHeader::new(
                            root_enum.name(),
                            variant.name(),
                            root_index,
                            variant_index,
                        );
                        let constant = header.constant_identifier();
                        let value = header.value_literal();
                        quote! { pub const #constant: u64 = #value; }
                    })
            },
        );
        quote! {
            pub mod short_header {
                #(#constants)*
            }
        }
        .to_tokens(tokens);
    }
}

/// Projects a schema `Name` into its SCREAMING_SNAKE_CASE constant form:
/// PascalCase word boundaries become underscores and lowercase runs are
/// uppercased. Owns the borrowed name and renders the screaming string.
struct ScreamingName<'name> {
    name: &'name Name,
}

impl<'name> ScreamingName<'name> {
    fn new(name: &'name Name) -> Self {
        Self { name }
    }

    fn screaming(&self) -> String {
        let mut output = String::new();
        for (index, character) in self.name.as_str().chars().enumerate() {
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
}

/// How the generated `to_nota` bridge takes its receiver. A `Copy` noun
/// takes `self` by value and borrows for the encode call; every other noun
/// borrows `&self`.
#[derive(Clone, Copy)]
enum NotaEncodeReceiver {
    Borrowed,
    Owned,
}

impl NotaEncodeReceiver {
    fn to_nota_method(self) -> TokenStream {
        match self {
            Self::Borrowed => quote! {
                pub fn to_nota(&self) -> String {
                    <Self as NotaEncode>::to_nota(self)
                }
            },
            Self::Owned => quote! {
                pub fn to_nota(self) -> String {
                    <Self as NotaEncode>::to_nota(&self)
                }
            },
        }
    }
}

/// Renders the inherent NOTA bridge (`from_nota_block` + `to_nota`) on a
/// generated noun, gated by the context's NOTA feature gate. The noun is
/// named by identity; the receiver mode selects the `to_nota` shape.
struct NotaInherentBridgeTokens<'name, 'context> {
    name: &'name str,
    receiver: NotaEncodeReceiver,
    context: &'context RustRenderContext,
}

impl<'name, 'context> NotaInherentBridgeTokens<'name, 'context> {
    fn borrowed(name: &'name str, context: &'context RustRenderContext) -> Self {
        Self {
            name,
            receiver: NotaEncodeReceiver::Borrowed,
            context,
        }
    }

    fn owned(name: &'name str, context: &'context RustRenderContext) -> Self {
        Self {
            name,
            receiver: NotaEncodeReceiver::Owned,
            context,
        }
    }
}

impl ToTokens for NotaInherentBridgeTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let gate = self.context.nota_feature_gate();
        let name = RustIdentifier::new(self.name);
        let to_nota = self.receiver.to_nota_method();
        quote! {
            #gate
            impl #name {
                pub fn from_nota_block(block: &nota_next::Block) -> Result<Self, NotaDecodeError> {
                    <Self as NotaDecode>::from_nota_block(block)
                }
                #to_nota
            }
        }
        .to_tokens(tokens);
    }
}

/// Renders the root-enum `FromStr` + `Display` NOTA surface, each gated by
/// the context's NOTA feature gate.
struct NotaRootEnumStringSupportTokens<'name, 'context> {
    name: &'name str,
    context: &'context RustRenderContext,
}

impl<'name, 'context> NotaRootEnumStringSupportTokens<'name, 'context> {
    fn new(name: &'name str, context: &'context RustRenderContext) -> Self {
        Self { name, context }
    }
}

impl ToTokens for NotaRootEnumStringSupportTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let gate = self.context.nota_feature_gate();
        let name = RustIdentifier::new(self.name);
        quote! {
            #gate
            impl std::str::FromStr for #name {
                type Err = NotaDecodeError;
                fn from_str(source: &str) -> Result<Self, Self::Err> {
                    NotaSource::new(source).parse::<Self>()
                }
            }
            #gate
            impl std::fmt::Display for #name {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str(&<Self as NotaEncode>::to_nota(self))
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// Renders the inherent `new` / `payload` / `into_payload` accessors plus
/// the `From<Payload>` impl for a tuple newtype. The newtype owns its name
/// and the wrapped payload type, so it renders its own construction surface.
struct NewtypeInherentImplTokens<'newtype> {
    newtype: &'newtype RustNewtype,
}

impl<'newtype> NewtypeInherentImplTokens<'newtype> {
    fn new(newtype: &'newtype RustNewtype) -> Self {
        Self { newtype }
    }
}

impl ToTokens for NewtypeInherentImplTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = RustIdentifier::new(self.newtype.name().as_str());
        let payload_type = RustTypeReferenceTokens::new(self.newtype.reference());
        quote! {
            impl #name {
                pub fn new(payload: #payload_type) -> Self {
                    Self(payload)
                }
                pub fn payload(&self) -> &#payload_type {
                    &self.0
                }
                pub fn into_payload(self) -> #payload_type {
                    self.0
                }
            }
            impl From<#payload_type> for #name {
                fn from(payload: #payload_type) -> Self {
                    Self::new(payload)
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// Renders a cross-crate `use` import as Rust tokens. The dependency crate's
/// `ResolvedImport` already produced the canonical `pub use path as Alias;`
/// item text; this parses it into an item token so it pretty-prints with the
/// rest of the generated module.
struct RustImportTokens<'import> {
    import: &'import RustImport,
}

impl<'import> RustImportTokens<'import> {
    fn new(import: &'import RustImport) -> Self {
        Self { import }
    }
}

impl ToTokens for RustImportTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        syn::parse_str::<syn::ItemUse>(self.import.use_item())
            .expect("generated use item parses")
            .to_tokens(tokens);
    }
}

struct SignalFrameStreamingSupportTokens<'event> {
    event_payload: &'event TypeReference,
}

impl<'event> SignalFrameStreamingSupportTokens<'event> {
    fn new(event_payload: &'event TypeReference) -> Self {
        Self { event_payload }
    }
}

impl ToTokens for SignalFrameStreamingSupportTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let event_type = RustTypeReferenceTokens::new(self.event_payload);
        quote! {
            impl signal_frame::RequestPayload for Input {}

            impl signal_frame::LogVariant for Input {
                fn log_variant(&self) -> u64 {
                    self.short_header()
                }
            }

            pub type Frame = signal_frame::StreamingFrame<Input, Output, #event_type>;
            pub type FrameBody = signal_frame::StreamingFrameBody<Input, Output, #event_type>;
            pub type Request = signal_frame::Request<Input>;
            pub type ReplyEnvelope = signal_frame::Reply<Output>;
            pub type RequestBuilder = signal_frame::RequestBuilder<Input>;

            impl Input {
                pub fn into_frame(self, exchange: signal_frame::ExchangeIdentifier) -> Frame {
                    let short_header = signal_frame::ShortHeader::new(self.short_header());
                    let request = signal_frame::Request::from_payload(self);
                    Frame::with_short_header(
                        short_header,
                        FrameBody::Request { exchange, request },
                    )
                }
            }

            impl Output {
                pub fn into_reply_frame(
                    self,
                    exchange: signal_frame::ExchangeIdentifier,
                ) -> Frame {
                    let short_header = signal_frame::ShortHeader::new(self.short_header());
                    let reply = signal_frame::Reply::committed(signal_frame::NonEmpty::single(
                        signal_frame::SubReply::Ok(self),
                    ));
                    Frame::with_short_header(
                        short_header,
                        FrameBody::Reply { exchange, reply },
                    )
                }
            }

            impl #event_type {
                pub fn into_subscription_frame(
                    self,
                    event_identifier: signal_frame::StreamEventIdentifier,
                    token: signal_frame::SubscriptionTokenInner,
                ) -> Frame {
                    Frame::with_short_header(
                        signal_frame::ShortHeader::new(short_header::OUTPUT_EVENT),
                        FrameBody::SubscriptionEvent {
                            event_identifier,
                            token,
                            event: self,
                        },
                    )
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct NexusRunnerNextStepProjectionTokens<'shape> {
    shape: &'shape NexusRunnerShape,
}

impl<'shape> NexusRunnerNextStepProjectionTokens<'shape> {
    fn new(shape: &'shape NexusRunnerShape) -> Self {
        Self { shape }
    }
}

impl ToTokens for NexusRunnerNextStepProjectionTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let reply_type = RustTypeTokens::new(&self.shape.reply_type);
        let sema_write_input_type = RustTypeTokens::new(self.shape.sema_write_input_type());
        let sema_read_input_type = RustTypeTokens::new(self.shape.sema_read_input_type());
        let effect_command_type = RustTypeTokens::new(self.shape.effect_command_type());
        let sema_write_arm = self.shape.emits_sema_write().then(|| {
            quote! {
                Self::CommandSemaWrite(input) => triad_runtime::NextStep::SemaWrite(input),
            }
        });
        let sema_read_arm = self.shape.emits_sema_read().then(|| {
            quote! {
                Self::CommandSemaRead(input) => triad_runtime::NextStep::SemaRead(input),
            }
        });
        let effect_arm = self.shape.emits_effect().then(|| {
            quote! {
                Self::CommandEffect(effect) => triad_runtime::NextStep::RunEffect(effect),
            }
        });
        let continue_arm = self.shape.has_continue.then(|| {
            quote! {
                Self::Continue(work) => triad_runtime::NextStep::Continue(work),
            }
        });

        quote! {
            pub type NexusRunnerNextStep = triad_runtime::NextStep<
                #reply_type,
                #sema_write_input_type,
                #sema_read_input_type,
                #effect_command_type,
                NexusWork,
            >;

            impl triad_runtime::NexusAction for NexusAction {
                type Reply = #reply_type;
                type SemaWrite = #sema_write_input_type;
                type SemaRead = #sema_read_input_type;
                type Effect = #effect_command_type;
                type Work = NexusWork;

                fn into_next_step(self) -> NexusRunnerNextStep {
                    match self {
                        #sema_write_arm
                        #sema_read_arm
                        Self::ReplyToSignal(output) => triad_runtime::NextStep::Reply(output),
                        #effect_arm
                        #continue_arm
                    }
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct NexusRunnerAdapterTokens<'shape> {
    shape: &'shape NexusRunnerShape,
}

impl<'shape> NexusRunnerAdapterTokens<'shape> {
    fn new(shape: &'shape NexusRunnerShape) -> Self {
        Self { shape }
    }
}

impl ToTokens for NexusRunnerAdapterTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let reply_type = RustTypeTokens::new(&self.shape.reply_type);
        let sema_write_input_type = RustTypeTokens::new(self.shape.sema_write_input_type());
        let sema_read_input_type = RustTypeTokens::new(self.shape.sema_read_input_type());
        let effect_command_type = RustTypeTokens::new(self.shape.effect_command_type());
        let apply_sema_write_body =
            if let Some(output_type) = self.shape.sema_write_output_type.as_deref() {
                let output_type = RustTypeTokens::new(output_type);
                quote! {
                    let output: #output_type = NexusEngine::apply_sema_write(
                        self.engine,
                        self.origin_route,
                        write,
                    );
                    NexusWork::sema_write_completed(output)
                }
            } else {
                quote! {
                    match write {}
                }
            };
        let observe_sema_read_body =
            if let Some(output_type) = self.shape.sema_read_output_type.as_deref() {
                let output_type = RustTypeTokens::new(output_type);
                quote! {
                    let output: #output_type = NexusEngine::observe_sema_read(
                        self.engine,
                        self.origin_route,
                        read,
                    );
                    NexusWork::sema_read_completed(output)
                }
            } else {
                quote! {
                    match read {}
                }
            };
        let run_effect_body = if let Some(output_type) = self.shape.effect_result_type.as_deref() {
            let output_type = RustTypeTokens::new(output_type);
            quote! {
                let output: #output_type = NexusEngine::run_effect(self.engine, effect);
                NexusWork::effect_completed(output)
            }
        } else {
            quote! {
                match effect {}
            }
        };

        quote! {
            struct NexusRunnerAdapter<'engine, Engine> {
                engine: &'engine mut Engine,
                origin_route: OriginRoute,
            }

            impl<'engine, Engine> NexusRunnerAdapter<'engine, Engine> {
                fn new(engine: &'engine mut Engine, origin_route: OriginRoute) -> Self {
                    Self {
                        engine,
                        origin_route,
                    }
                }
            }

            impl<'engine, Engine> triad_runtime::RunnerEngines
                for NexusRunnerAdapter<'engine, Engine>
            where
                Engine: NexusEngine,
            {
                type Reply = #reply_type;
                type SemaWrite = #sema_write_input_type;
                type SemaRead = #sema_read_input_type;
                type Effect = #effect_command_type;
                type Work = NexusWork;

                fn decide_next_step(
                    &mut self,
                    work: Self::Work,
                ) -> triad_runtime::runner::RunnerNextStep<Self> {
                    let action = NexusEngine::decide(
                        self.engine,
                        work.with_origin_route(self.origin_route),
                    )
                    .into_root();
                    triad_runtime::NexusAction::into_next_step(action)
                }

                fn apply_sema_write(&mut self, write: Self::SemaWrite) -> Self::Work {
                    #apply_sema_write_body
                }

                fn observe_sema_read(&self, read: Self::SemaRead) -> Self::Work {
                    #observe_sema_read_body
                }

                fn run_effect(&mut self, effect: Self::Effect) -> Self::Work {
                    #run_effect_body
                }

                fn budget_exhausted_reply(
                    &self,
                    exhausted: triad_runtime::ContinuationExhausted,
                ) -> Self::Reply {
                    NexusEngine::budget_exhausted_reply(self.engine, exhausted)
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct ActorLifecycleSupportTokens;

impl ToTokens for ActorLifecycleSupportTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub enum ActorStartFailure {
                ResourceBusy(String),
                ConfigurationInvalid(String),
            }

            impl std::fmt::Display for ActorStartFailure {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::ResourceBusy(message) => {
                            write!(formatter, "actor resource busy: {message}")
                        }
                        Self::ConfigurationInvalid(message) => {
                            write!(formatter, "actor configuration invalid: {message}")
                        }
                    }
                }
            }

            impl std::error::Error for ActorStartFailure {}

            #[derive(Clone, Debug, PartialEq, Eq)]
            pub enum ActorStopFailure {
                ResourceLocked(String),
                ChildStillRunning(String),
            }

            impl std::fmt::Display for ActorStopFailure {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::ResourceLocked(message) => {
                            write!(formatter, "actor resource locked: {message}")
                        }
                        Self::ChildStillRunning(message) => {
                            write!(formatter, "actor child still running: {message}")
                        }
                    }
                }
            }

            impl std::error::Error for ActorStopFailure {}
        }
        .to_tokens(tokens);
    }
}

struct SignalEngineTraitTokens {
    plane: Plane,
    emits_concrete_signal_engine: bool,
}

impl SignalEngineTraitTokens {
    fn new(emits_concrete_signal_engine: bool) -> Self {
        Self {
            plane: Plane::Signal,
            emits_concrete_signal_engine,
        }
    }
}

impl ToTokens for SignalEngineTraitTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let engine_trait = RustIdentifier::new(self.plane.engine_trait_name());
        let trace_enum = RustIdentifier::new(self.plane.trace_enum_name());
        let trace_activation = RustIdentifier::new(self.plane.trace_activation_method_name());
        let associated_nexus_types = (!self.emits_concrete_signal_engine).then(|| {
            quote! {
                type NexusInput;
                type NexusOutput;
            }
        });
        let triage_inner = if self.emits_concrete_signal_engine {
            quote! {
                fn triage_inner(
                    &self,
                    input: signal::Signal<signal::Input>,
                ) -> nexus::Nexus<nexus::Work>;
            }
        } else {
            quote! {
                fn triage_inner(&self, input: signal::Signal<signal::Input>) -> Self::NexusInput;
            }
        };
        let reply_inner = if self.emits_concrete_signal_engine {
            quote! {
                fn reply_inner(
                    &self,
                    output: nexus::Nexus<nexus::Action>,
                ) -> signal::Signal<signal::Output>;
            }
        } else {
            quote! {
                fn reply_inner(
                    &self,
                    output: Self::NexusOutput,
                ) -> signal::Signal<signal::Output>;
            }
        };
        let triage_output = if self.emits_concrete_signal_engine {
            quote! { nexus::Nexus<nexus::Work> }
        } else {
            quote! { Self::NexusInput }
        };
        let reply_input = if self.emits_concrete_signal_engine {
            quote! { nexus::Nexus<nexus::Action> }
        } else {
            quote! { Self::NexusOutput }
        };

        quote! {
            pub trait #engine_trait {
                #associated_nexus_types

                fn on_start(&mut self) -> Result<(), ActorStartFailure> {
                    Ok(())
                }

                fn on_stop(&mut self) -> Result<(), ActorStopFailure> {
                    Ok(())
                }

                fn #trace_activation(&self, _object_name: #trace_enum) {}

                fn trace_signal_admitted(&self) {
                    self.#trace_activation(#trace_enum::Admitted);
                }

                fn trace_signal_rejected(&self) {
                    self.#trace_activation(#trace_enum::Rejected);
                }

                fn trace_signal_triaged(&self) {
                    self.#trace_activation(#trace_enum::Triaged);
                }

                fn trace_signal_replied(&self) {
                    self.#trace_activation(#trace_enum::Replied);
                }

                #triage_inner

                #reply_inner

                fn triage(&self, input: signal::Signal<signal::Input>) -> #triage_output {
                    let output = self.triage_inner(input);
                    self.trace_signal_triaged();
                    output
                }

                fn reply(&self, output: #reply_input) -> signal::Signal<signal::Output> {
                    let signal_output = self.reply_inner(output);
                    self.trace_signal_replied();
                    signal_output
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct NexusEngineTraitTokens<'shape> {
    plane: Plane,
    runner_shape: Option<&'shape NexusRunnerShape>,
}

impl<'shape> NexusEngineTraitTokens<'shape> {
    fn new(runner_shape: Option<&'shape NexusRunnerShape>) -> Self {
        Self {
            plane: Plane::Nexus,
            runner_shape,
        }
    }
}

impl ToTokens for NexusEngineTraitTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let engine_trait = RustIdentifier::new(self.plane.engine_trait_name());
        let trace_enum = RustIdentifier::new(self.plane.trace_enum_name());
        let trace_activation = RustIdentifier::new(self.plane.trace_activation_method_name());
        let runner_hooks = self.runner_shape.map(|shape| {
            let sema_write_hook = match (
                shape.sema_write_input_type.as_deref(),
                shape.sema_write_output_type.as_deref(),
            ) {
                (Some(input_type), Some(output_type)) => {
                    let input_type = RustTypeTokens::new(input_type);
                    let output_type = RustTypeTokens::new(output_type);
                    quote! {
                        fn apply_sema_write(
                            &mut self,
                            origin_route: OriginRoute,
                            input: #input_type,
                        ) -> #output_type;
                    }
                }
                _ => quote! {},
            };
            let sema_read_hook = match (
                shape.sema_read_input_type.as_deref(),
                shape.sema_read_output_type.as_deref(),
            ) {
                (Some(input_type), Some(output_type)) => {
                    let input_type = RustTypeTokens::new(input_type);
                    let output_type = RustTypeTokens::new(output_type);
                    quote! {
                        fn observe_sema_read(
                            &self,
                            origin_route: OriginRoute,
                            input: #input_type,
                        ) -> #output_type;
                    }
                }
                _ => quote! {},
            };
            let effect_hook = match (
                shape.effect_command_type.as_deref(),
                shape.effect_result_type.as_deref(),
            ) {
                (Some(input_type), Some(output_type)) => {
                    let input_type = RustTypeTokens::new(input_type);
                    let output_type = RustTypeTokens::new(output_type);
                    quote! {
                        fn run_effect(&mut self, input: #input_type) -> #output_type;
                    }
                }
                _ => quote! {},
            };
            let reply_type = RustTypeTokens::new(&shape.reply_type);
            quote! {
                fn continuation_limit(&self) -> triad_runtime::ContinuationLimit {
                    triad_runtime::ContinuationLimit::default()
                }

                #sema_write_hook

                #sema_read_hook

                #effect_hook

                fn budget_exhausted_reply(
                    &self,
                    exhausted: triad_runtime::ContinuationExhausted,
                ) -> #reply_type;
            }
        });
        let sized_where = self.runner_shape.is_some().then(|| {
            quote! {
                where
                    Self: Sized,
            }
        });
        let execute_body = if self.runner_shape.is_some() {
            quote! {
                let origin_route = input.origin_route();
                let first_work = input.into_root();
                let runner = triad_runtime::Runner::new(self.continuation_limit());
                let mut runner_adapter = NexusRunnerAdapter::new(self, origin_route);
                let reply = runner.drive(&mut runner_adapter, first_work);
                let output = NexusAction::reply_to_signal(reply).with_origin_route(origin_route);
            }
        } else {
            quote! {
                let output = self.decide(input);
            }
        };

        quote! {
            pub trait #engine_trait {
                fn on_start(&mut self) -> Result<(), ActorStartFailure> {
                    Ok(())
                }

                fn on_stop(&mut self) -> Result<(), ActorStopFailure> {
                    Ok(())
                }

                fn #trace_activation(&self, _object_name: #trace_enum) {}

                fn trace_nexus_entered(&self) {
                    self.#trace_activation(#trace_enum::Entered);
                }

                fn trace_nexus_decided(&self) {
                    self.#trace_activation(#trace_enum::Decided);
                }

                #runner_hooks

                fn decide(
                    &mut self,
                    input: nexus::Nexus<nexus::Work>,
                ) -> nexus::Nexus<nexus::Action>;

                fn execute(
                    &mut self,
                    input: nexus::Nexus<nexus::Work>,
                ) -> nexus::Nexus<nexus::Action>
                #sized_where
                {
                    self.trace_nexus_entered();
                    #execute_body
                    self.trace_nexus_decided();
                    output
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct SemaEngineTraitTokens {
    plane: Plane,
    emits_apply: bool,
    emits_observe: bool,
}

impl SemaEngineTraitTokens {
    fn new(emits_apply: bool, emits_observe: bool) -> Self {
        Self {
            plane: Plane::Sema,
            emits_apply,
            emits_observe,
        }
    }
}

impl ToTokens for SemaEngineTraitTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let engine_trait = RustIdentifier::new(self.plane.engine_trait_name());
        let trace_enum = RustIdentifier::new(self.plane.trace_enum_name());
        let trace_activation = RustIdentifier::new(self.plane.trace_activation_method_name());
        let write_trace = self.emits_apply.then(|| {
            quote! {
                fn trace_sema_write_applied(&self) {
                    self.#trace_activation(#trace_enum::WriteApplied);
                }
            }
        });
        let read_trace = self.emits_observe.then(|| {
            quote! {
                fn trace_sema_read_observed(&self) {
                    self.#trace_activation(#trace_enum::ReadObserved);
                }
            }
        });
        let apply_inner = self.emits_apply.then(|| {
            quote! {
                fn apply_inner(
                    &mut self,
                    input: sema::Sema<sema::WriteInput>,
                ) -> sema::Sema<sema::WriteOutput>;
            }
        });
        let observe_inner = self.emits_observe.then(|| {
            quote! {
                fn observe_inner(
                    &self,
                    input: sema::Sema<sema::ReadInput>,
                ) -> sema::Sema<sema::ReadOutput>;
            }
        });
        let apply = self.emits_apply.then(|| {
            quote! {
                fn apply(
                    &mut self,
                    input: sema::Sema<sema::WriteInput>,
                ) -> sema::Sema<sema::WriteOutput> {
                    let output = self.apply_inner(input);
                    self.trace_sema_write_applied();
                    output
                }
            }
        });
        let observe = self.emits_observe.then(|| {
            quote! {
                fn observe(&self, input: sema::Sema<sema::ReadInput>) -> sema::Sema<sema::ReadOutput> {
                    let output = self.observe_inner(input);
                    self.trace_sema_read_observed();
                    output
                }
            }
        });

        quote! {
            pub trait #engine_trait {
                fn on_start(&mut self) -> Result<(), ActorStartFailure> {
                    Ok(())
                }

                fn on_stop(&mut self) -> Result<(), ActorStopFailure> {
                    Ok(())
                }

                fn #trace_activation(&self, _object_name: #trace_enum) {}

                #write_trace

                #read_trace

                #apply_inner

                #observe_inner

                #apply

                #observe
            }
        }
        .to_tokens(tokens);
    }
}

struct RuntimeCopyNewtypeTokens<'context> {
    name: &'static str,
    context: &'context RustRenderContext,
}

impl<'context> RuntimeCopyNewtypeTokens<'context> {
    fn new(name: &'static str, context: &'context RustRenderContext) -> Self {
        Self { name, context }
    }
}

impl ToTokens for RuntimeCopyNewtypeTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = RustIdentifier::new(self.name);
        let attributes = self.context.derive_attributes(true, false);
        quote! {
            #( #attributes )*
            pub struct #name(pub Integer);
        }
        .to_tokens(tokens);
    }
}

struct MessageRootTokens<'schema, 'context> {
    root_enums: &'schema [RustEnum],
    context: &'context RustRenderContext,
}

impl<'schema, 'context> MessageRootTokens<'schema, 'context> {
    fn new(root_enums: &'schema [RustEnum], context: &'context RustRenderContext) -> Self {
        Self {
            root_enums,
            context,
        }
    }
}

impl ToTokens for MessageRootTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attributes = self.context.derive_attributes(true, false);
        let variants = self.root_enums.iter().map(|root_enum| {
            let name = RustIdentifier::new(root_enum.name().as_str());
            quote! { #name, }
        });
        quote! {
            #( #attributes )*
            pub enum MessageRoot {
                #( #variants )*
            }
        }
        .to_tokens(tokens);
    }
}

struct SignalMailLifecycleSupportTokens<'schema> {
    root_enums: &'schema [RustEnum],
}

impl<'schema> SignalMailLifecycleSupportTokens<'schema> {
    fn new(root_enums: &'schema [RustEnum]) -> Self {
        Self { root_enums }
    }
}

impl ToTokens for SignalMailLifecycleSupportTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let root_impls = self.root_enums.iter().map(|root_enum| {
            let root_name = RustIdentifier::new(root_enum.name().as_str());
            quote! {
                impl #root_name {
                    pub fn with_origin_route(self, origin_route: OriginRoute) -> Signal<Self> {
                        Signal::new(origin_route, self)
                    }
                }

                impl signal::Signal<#root_name> {
                    pub fn message_sent(&self, identifier: MessageIdentifier) -> MessageSent {
                        MessageSent {
                            identifier,
                            origin_route: self.origin_route(),
                            root: MessageRoot::#root_name,
                            short_header: self.root().short_header(),
                        }
                    }
                }
            }
        });

        quote! {
            #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
            pub struct MessageSent {
                pub identifier: MessageIdentifier,
                pub origin_route: OriginRoute,
                pub root: MessageRoot,
                pub short_header: Integer,
            }

            #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
            pub struct MessageProcessed<Reply> {
                pub identifier: MessageIdentifier,
                pub origin_route: OriginRoute,
                pub reply: Reply,
            }

            pub trait MessageSentHook {
                type Error;

                fn message_sent(&mut self, event: MessageSent) -> Result<(), Self::Error>;
            }

            pub trait MessageProcessedHook<Reply> {
                type Error;

                fn message_processed(
                    &mut self,
                    event: MessageProcessed<Reply>,
                ) -> Result<(), Self::Error>;
            }

            impl MessageSent {
                pub fn origin_route(&self) -> OriginRoute {
                    self.origin_route
                }

                pub fn push_to<Hook>(&self, hook: &mut Hook) -> Result<(), Hook::Error>
                where
                    Hook: MessageSentHook,
                {
                    hook.message_sent(self.clone())
                }
            }

            impl<Reply> MessageProcessed<Reply> {
                pub fn new(
                    identifier: MessageIdentifier,
                    origin_route: OriginRoute,
                    reply: Reply,
                ) -> Self {
                    Self {
                        identifier,
                        origin_route,
                        reply,
                    }
                }

                pub fn identifier(&self) -> MessageIdentifier {
                    self.identifier
                }

                pub fn origin_route(&self) -> OriginRoute {
                    self.origin_route
                }

                pub fn into_reply(self) -> Reply {
                    self.reply
                }

                pub fn push_to<Hook>(&self, hook: &mut Hook) -> Result<(), Hook::Error>
                where
                    Hook: MessageProcessedHook<Reply>,
                    Reply: Clone,
                {
                    hook.message_processed(self.clone())
                }
            }

            #( #root_impls )*
        }
        .to_tokens(tokens);
    }
}

struct SchemaPlaneSupportTokens;

impl ToTokens for SchemaPlaneSupportTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            pub mod schema {
                #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
                pub enum Plane<SignalRoot, NexusRoot, SemaRoot> {
                    Signal(super::Signal<SignalRoot>),
                    Nexus(super::Nexus<NexusRoot>),
                    Sema(super::Sema<SemaRoot>),
                }

                impl<SignalRoot, NexusRoot, SemaRoot> Plane<SignalRoot, NexusRoot, SemaRoot> {
                    pub fn origin_route(&self) -> super::OriginRoute {
                        match self {
                            Self::Signal(message) => message.origin_route(),
                            Self::Nexus(message) => message.origin_route(),
                            Self::Sema(message) => message.origin_route(),
                        }
                    }
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct PlaneEnvelopeTokens<'name> {
    name: &'name str,
}

impl<'name> PlaneEnvelopeTokens<'name> {
    fn new(name: &'name str) -> Self {
        Self { name }
    }
}

impl ToTokens for PlaneEnvelopeTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = RustIdentifier::new(self.name);
        quote! {
            #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
            pub struct #name<Root> {
                pub origin_route: OriginRoute,
                pub root: Root,
            }

            impl<Root> #name<Root> {
                pub fn new(origin_route: OriginRoute, root: Root) -> Self {
                    Self { origin_route, root }
                }

                pub fn origin_route(&self) -> OriginRoute {
                    self.origin_route
                }

                pub fn root(&self) -> &Root {
                    &self.root
                }

                pub fn into_root(self) -> Root {
                    self.root
                }

                pub fn map_root<NextRoot>(self, map: impl FnOnce(Root) -> NextRoot) -> #name<NextRoot> {
                    #name::new(self.origin_route, map(self.root))
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct PlaneNamespaceAlias<'source> {
    export_name: &'static str,
    source_type_name: &'source str,
}

impl<'source> PlaneNamespaceAlias<'source> {
    fn new(export_name: &'static str, source_type_name: &'source str) -> Self {
        Self {
            export_name,
            source_type_name,
        }
    }
}

impl ToTokens for PlaneNamespaceAlias<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let export = RustIdentifier::new(self.export_name);
        let source = RustIdentifier::new(self.source_type_name);
        quote! {
            pub type #export = super::#source;
        }
        .to_tokens(tokens);
    }
}

struct PlaneNamespaceTokens<'source> {
    plane: Plane,
    aliases: Vec<PlaneNamespaceAlias<'source>>,
}

impl<'source> PlaneNamespaceTokens<'source> {
    fn new(plane: Plane, aliases: Vec<PlaneNamespaceAlias<'source>>) -> Self {
        Self { plane, aliases }
    }
}

impl ToTokens for PlaneNamespaceTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let module = RustIdentifier::new(self.plane.module_name());
        let wrapper = RustIdentifier::new(self.plane.wrapper_name());
        let aliases = &self.aliases;
        quote! {
            #[allow(clippy::module_inception)]
            pub mod #module {
                #(#aliases)*
                pub type #wrapper<Root> = super::#wrapper<Root>;
            }
        }
        .to_tokens(tokens);
    }
}

struct PlaneOriginRouteConstructorTokens<'source> {
    plane: Plane,
    type_name: &'source str,
}

impl<'source> PlaneOriginRouteConstructorTokens<'source> {
    fn new(plane: Plane, type_name: &'source str) -> Self {
        Self { plane, type_name }
    }
}

impl ToTokens for PlaneOriginRouteConstructorTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let type_name = RustIdentifier::new(self.type_name);
        let wrapper_path = PlaneWrapperPath::new(self.plane);
        quote! {
            impl #type_name {
                pub fn with_origin_route(self, origin_route: OriginRoute) -> #wrapper_path<Self> {
                    #wrapper_path::new(origin_route, self)
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct PlaneWrapperPath {
    plane: Plane,
}

impl PlaneWrapperPath {
    fn new(plane: Plane) -> Self {
        Self { plane }
    }
}

impl ToTokens for PlaneWrapperPath {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let wrapper = RustIdentifier::new(self.plane.wrapper_name());
        match self.plane {
            Plane::Signal => quote! { #wrapper },
            Plane::Nexus | Plane::Sema => {
                let module = RustIdentifier::new(self.plane.module_name());
                quote! { #module::#wrapper }
            }
        }
        .to_tokens(tokens);
    }
}

struct TraceObjectNameEnumTokens<'schema, 'context> {
    plane: Plane,
    interface_roots: &'schema [TraceInterfaceRoot<'schema>],
    actor_variants: &'schema [&'static str],
    context: &'context RustRenderContext,
}

impl<'schema, 'context> TraceObjectNameEnumTokens<'schema, 'context> {
    fn new(
        plane: Plane,
        interface_roots: &'schema [TraceInterfaceRoot<'schema>],
        actor_variants: &'schema [&'static str],
        context: &'context RustRenderContext,
    ) -> Self {
        Self {
            plane,
            interface_roots,
            actor_variants,
            context,
        }
    }
}

impl ToTokens for TraceObjectNameEnumTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let enum_name = RustIdentifier::new(self.plane.trace_enum_name());
        let attributes = self.context.derive_attributes(true, false);
        let interface_variants = self.interface_roots.iter().map(|root| {
            let object_variant = RustIdentifier::new(root.object_variant);
            let route_name = format!("{}Route", root.type_name.as_str());
            let route_type = RustIdentifier::new(&route_name);
            quote! { #object_variant(#route_type), }
        });
        let actor_variants = self.actor_variants.iter().map(|variant| {
            let variant = RustIdentifier::new(variant);
            quote! { #variant, }
        });
        let interface_match_arms = self.interface_roots.iter().map(|root| {
            let object_variant = RustIdentifier::new(root.object_variant);
            let route_name = format!("{}Route", root.type_name.as_str());
            let route_type = RustIdentifier::new(&route_name);
            let route_arms = root.enum_declaration.variants().iter().map(|variant| {
                let variant_name = RustIdentifier::new(variant.name().as_str());
                let rendered_name = format!("{}{}", root.name_prefix, variant.name());
                let rendered_name = Literal::string(&rendered_name);
                quote! { #route_type::#variant_name => #rendered_name, }
            });
            quote! {
                Self::#object_variant(route) => match route {
                    #( #route_arms )*
                },
            }
        });
        let actor_match_arms = self.actor_variants.iter().map(|variant| {
            let variant_name = RustIdentifier::new(variant);
            let rendered_name = format!("{}{}", self.plane.wrapper_name(), variant);
            let rendered_name = Literal::string(&rendered_name);
            quote! { Self::#variant_name => #rendered_name, }
        });
        quote! {
            #( #attributes )*
            pub enum #enum_name {
                #( #interface_variants )*
                #( #actor_variants )*
            }

            impl #enum_name {
                pub fn name(self) -> &'static str {
                    match self {
                        #( #interface_match_arms )*
                        #( #actor_match_arms )*
                    }
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct TraceSupportTokens<'context> {
    planes: Vec<Plane>,
    context: &'context RustRenderContext,
}

impl<'context> TraceSupportTokens<'context> {
    fn new(planes: Vec<Plane>, context: &'context RustRenderContext) -> Self {
        Self { planes, context }
    }
}

impl ToTokens for TraceSupportTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attributes = self.context.derive_attributes(true, false);
        let variants = self.planes.iter().map(|plane| {
            let variant = RustIdentifier::new(plane.wrapper_name());
            let trace_enum = RustIdentifier::new(plane.trace_enum_name());
            quote! { #variant(#trace_enum), }
        });
        let match_arms = self.planes.iter().map(|plane| {
            let variant = RustIdentifier::new(plane.wrapper_name());
            quote! { Self::#variant(object_name) => object_name.name(), }
        });
        quote! {
            #( #attributes )*
            pub enum ObjectName {
                #( #variants )*
            }

            #( #attributes )*
            pub struct TraceEvent(pub ObjectName);

            impl ObjectName {
                pub fn name(self) -> &'static str {
                    match self {
                        #( #match_arms )*
                    }
                }
            }

            impl TraceEvent {
                pub fn new(object_name: ObjectName) -> Self {
                    Self(object_name)
                }

                pub fn object_name(&self) -> ObjectName {
                    self.0
                }

                pub fn name(&self) -> &'static str {
                    self.0.name()
                }
            }
        }
        .to_tokens(tokens);
    }
}

struct RustDeclarationTokens<'declaration, 'context> {
    declaration: &'declaration RustDeclaration,
    context: &'context RustRenderContext,
}

impl<'declaration, 'context> RustDeclarationTokens<'declaration, 'context> {
    fn new(
        declaration: &'declaration RustDeclaration,
        context: &'context RustRenderContext,
    ) -> Self {
        Self {
            declaration,
            context,
        }
    }
}

impl ToTokens for RustDeclarationTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.declaration.value() {
            RustTypeDeclaration::Alias(value) => {
                RustAliasTokens::new(value, self.declaration.visibility(), self.context)
                    .to_tokens(tokens)
            }
            RustTypeDeclaration::Struct(value) => {
                RustStructTokens::new(value, self.declaration.visibility(), self.context)
                    .to_tokens(tokens)
            }
            RustTypeDeclaration::Enum(value) => {
                RustEnumTokens::new(value, self.declaration.visibility(), self.context)
                    .to_tokens(tokens)
            }
            RustTypeDeclaration::Newtype(value) => {
                RustNewtypeTokens::new(value, self.declaration.visibility(), self.context)
                    .to_tokens(tokens)
            }
        }
    }
}

struct RustAliasTokens<'alias, 'context> {
    alias: &'alias RustAlias,
    visibility: Visibility,
    context: &'context RustRenderContext,
}

impl<'alias, 'context> RustAliasTokens<'alias, 'context> {
    fn new(
        alias: &'alias RustAlias,
        visibility: Visibility,
        context: &'context RustRenderContext,
    ) -> Self {
        Self {
            alias,
            visibility,
            context,
        }
    }
}

impl ToTokens for RustAliasTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let visibility = self.context.visibility_tokens(self.visibility);
        let name = RustIdentifier::new(self.alias.name().as_str());
        let reference = RustTypeReferenceTokens::new(self.alias.reference());
        quote! {
            #visibility type #name = #reference;
        }
        .to_tokens(tokens);
    }
}

struct RustNewtypeTokens<'newtype, 'context> {
    newtype: &'newtype RustNewtype,
    visibility: Visibility,
    context: &'context RustRenderContext,
}

impl<'newtype, 'context> RustNewtypeTokens<'newtype, 'context> {
    fn new(
        newtype: &'newtype RustNewtype,
        visibility: Visibility,
        context: &'context RustRenderContext,
    ) -> Self {
        Self {
            newtype,
            visibility,
            context,
        }
    }
}

impl ToTokens for RustNewtypeTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attributes = self.context.data_type_attributes(self.newtype.name());
        let visibility = self.context.visibility_tokens(self.visibility);
        let name = RustIdentifier::new(self.newtype.name().as_str());
        let reference = RustTypeReferenceTokens::new(self.newtype.reference());
        quote! {
            #(#attributes)*
            #visibility struct #name(#visibility #reference);
        }
        .to_tokens(tokens);
    }
}

struct RustStructTokens<'structure, 'context> {
    structure: &'structure RustStruct,
    visibility: Visibility,
    context: &'context RustRenderContext,
}

impl<'structure, 'context> RustStructTokens<'structure, 'context> {
    fn new(
        structure: &'structure RustStruct,
        visibility: Visibility,
        context: &'context RustRenderContext,
    ) -> Self {
        Self {
            structure,
            visibility,
            context,
        }
    }
}

impl ToTokens for RustStructTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attributes = self.context.data_type_attributes(self.structure.name());
        let visibility = self.context.visibility_tokens(self.visibility);
        let name = RustIdentifier::new(self.structure.name().as_str());
        let fields = self
            .structure
            .fields()
            .iter()
            .map(|field| RustFieldTokens::new(field, self.visibility, self.context))
            .collect::<Vec<_>>();
        quote! {
            #(#attributes)*
            #visibility struct #name {
                #(#fields)*
            }
        }
        .to_tokens(tokens);
    }
}

struct RustFieldTokens<'field, 'context> {
    field: &'field RustField,
    visibility: Visibility,
    context: &'context RustRenderContext,
}

impl<'field, 'context> RustFieldTokens<'field, 'context> {
    fn new(
        field: &'field RustField,
        visibility: Visibility,
        context: &'context RustRenderContext,
    ) -> Self {
        Self {
            field,
            visibility,
            context,
        }
    }
}

impl ToTokens for RustFieldTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let visibility = self
            .context
            .field_visibility_tokens(self.visibility, self.field.reference());
        let name = RustIdentifier::new(self.field.name().as_str());
        let reference = RustTypeReferenceTokens::new(self.field.reference());
        quote! {
            #visibility #name: #reference,
        }
        .to_tokens(tokens);
    }
}

struct RustEnumTokens<'enumeration, 'context> {
    enumeration: &'enumeration RustEnum,
    visibility: Visibility,
    context: &'context RustRenderContext,
    root: bool,
}

impl<'enumeration, 'context> RustEnumTokens<'enumeration, 'context> {
    fn new(
        enumeration: &'enumeration RustEnum,
        visibility: Visibility,
        context: &'context RustRenderContext,
    ) -> Self {
        Self {
            enumeration,
            visibility,
            context,
            root: false,
        }
    }

    fn root(enumeration: &'enumeration RustEnum, context: &'context RustRenderContext) -> Self {
        Self {
            enumeration,
            visibility: Visibility::Public,
            context,
            root: true,
        }
    }

    fn attributes(&self) -> Vec<TokenStream> {
        if self.root {
            self.context.root_data_type_attributes()
        } else {
            self.context.data_type_attributes(self.enumeration.name())
        }
    }
}

impl ToTokens for RustEnumTokens<'_, '_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attributes = self.attributes();
        let visibility = self.context.visibility_tokens(self.visibility);
        let name = RustIdentifier::new(self.enumeration.name().as_str());
        let variants = self
            .enumeration
            .variants()
            .iter()
            .map(RustEnumVariantTokens::new)
            .collect::<Vec<_>>();
        quote! {
            #(#attributes)*
            #visibility enum #name {
                #(#variants)*
            }
        }
        .to_tokens(tokens);
    }
}

struct RustEnumVariantTokens<'variant> {
    variant: &'variant RustEnumVariant,
}

impl<'variant> RustEnumVariantTokens<'variant> {
    fn new(variant: &'variant RustEnumVariant) -> Self {
        Self { variant }
    }
}

impl ToTokens for RustEnumVariantTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = RustIdentifier::new(self.variant.name().as_str());
        match self.variant.payload() {
            Some(reference) => {
                let reference = RustTypeReferenceTokens::new(reference);
                quote! { #name(#reference), }.to_tokens(tokens);
            }
            None => quote! { #name, }.to_tokens(tokens),
        }
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

/// The render driver for one generated module. It owns the emission
/// context (the map-key and private-type name sets, the NOTA surface, and
/// the emission target) and accumulates the pretty-printed source text as
/// each section's token-wrapper noun renders itself through
/// [`RustModuleRenderer::emit_item_tokens`]. It builds **no** Rust as
/// strings — the only direct text it writes is the leading `// @generated`
/// header comment, which cannot pass through `prettyplease` because that
/// drops non-doc comments. Its remaining `emits_*` / `*_root` / `trace_*`
/// methods are schema-analysis predicates that decide which sections emit;
/// the syntax of every section lives on its owning `*Tokens` noun.
struct RustModuleRenderer {
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

/// Renders `impl From<Payload> for Enum { fn from(payload) { Self::Variant(payload) } }`
/// for an enum variant whose payload type is unique within the enum. Owns
/// the enum name, the variant name, and the payload type name.
struct EnumPayloadFromImplTokens<'name> {
    enum_name: &'name Name,
    variant_name: &'name Name,
    payload: &'name str,
}

impl<'name> EnumPayloadFromImplTokens<'name> {
    fn new(enum_name: &'name Name, variant_name: &'name Name, payload: &'name str) -> Self {
        Self {
            enum_name,
            variant_name,
            payload,
        }
    }
}

impl ToTokens for EnumPayloadFromImplTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let enum_name = RustIdentifier::new(self.enum_name.as_str());
        let variant = RustIdentifier::new(self.variant_name.as_str());
        let payload = RustIdentifier::new(self.payload);
        quote! {
            impl From<#payload> for #enum_name {
                fn from(payload: #payload) -> Self {
                    Self::#variant(payload)
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// One generated variant constructor: the method name, the source variant
/// name, and the resolved payload (argument type + wrap expression).
struct EnumVariantConstructor {
    method_name: String,
    variant_name: String,
    payload: EnumConstructorPayload,
}

impl EnumVariantConstructor {
    fn new(method_name: String, variant_name: String, payload: EnumConstructorPayload) -> Self {
        Self {
            method_name,
            variant_name,
            payload,
        }
    }

    fn method_tokens(&self) -> TokenStream {
        let method_name = RustIdentifier::new(&self.method_name).ident();
        let variant = RustIdentifier::new(&self.variant_name);
        let argument_type = RustTypeTokens::new(self.payload.argument_type());
        let expression = syn::parse_str::<syn::Expr>(self.payload.expression())
            .expect("generated constructor expression parses");
        quote! {
            pub fn #method_name(payload: #argument_type) -> Self {
                Self::#variant(#expression)
            }
        }
    }
}

/// Renders the `impl Enum { pub fn variant(payload) -> Self { ... } ... }`
/// block carrying one associated constructor per payload-bearing variant.
struct EnumVariantConstructorsTokens<'data> {
    enum_name: &'data Name,
    constructors: &'data [EnumVariantConstructor],
}

impl<'data> EnumVariantConstructorsTokens<'data> {
    fn new(enum_name: &'data Name, constructors: &'data [EnumVariantConstructor]) -> Self {
        Self {
            enum_name,
            constructors,
        }
    }
}

impl ToTokens for EnumVariantConstructorsTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let enum_name = RustIdentifier::new(self.enum_name.as_str());
        let methods = self
            .constructors
            .iter()
            .map(EnumVariantConstructor::method_tokens);
        quote! {
            impl #enum_name {
                #(#methods)*
            }
        }
        .to_tokens(tokens);
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

struct RuntimeRoleTraitImpl {
    type_name: String,
    trait_name: &'static str,
    canonical_type_name: String,
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

impl RuntimeRoleTraitImpl {
    fn new(type_name: String, trait_name: &'static str, canonical_type_name: String) -> Self {
        Self {
            type_name,
            trait_name,
            canonical_type_name,
        }
    }

    fn matches(&self, type_name: &str, trait_name: &'static str) -> bool {
        self.canonical_type_name == type_name && self.trait_name == trait_name
    }
}

struct TraceInterfaceRoot<'schema> {
    object_variant: &'static str,
    name_prefix: &'static str,
    type_name: &'schema Name,
    enum_declaration: &'schema RustEnum,
}

impl RustModuleRenderer {
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

    fn emits_signal(&self) -> bool {
        self.runtime_planes().emits_signal()
    }

    fn emits_wire_frame(&self) -> bool {
        self.target.emits_wire_frame()
    }

    fn emits_short_headers(&self) -> bool {
        self.emits_wire_frame()
    }

    fn runtime_planes(&self) -> RuntimePlaneSet {
        self.target.runtime_planes()
    }

    fn render_context(&self) -> RustRenderContext {
        RustRenderContext::new(
            self.map_key_types.clone(),
            self.private_type_names.clone(),
            self.nota_surface.clone(),
        )
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

    fn emit_item_tokens(&mut self, tokens: TokenStream) {
        let file = syn::parse2::<syn::File>(tokens).expect("generated Rust item tokens parse");
        let source = prettyplease::unparse(&file);
        self.output.push_str(source.trim_end());
        self.output.push('\n');
    }

    /// Emit a `pub use` alias for each cross-crate import.
    ///
    /// The dependency crate emits its own definition of the type; the
    /// consumer references that type through the local alias instead of
    /// re-declaring it. Later fields or variants that name the imported
    /// type therefore use the dependency crate's type identity.
    fn emit_scalar_alias(&mut self, alias: &RustScalarAlias) {
        self.emit_item_tokens(RustScalarAliasTokens::new(alias).into_token_stream());
    }

    fn emit_imports(&mut self, imports: &[RustImport]) {
        if imports.is_empty() {
            return;
        }
        for import in imports {
            self.emit_item_tokens(RustImportTokens::new(import).into_token_stream());
        }
        self.blank();
    }

    fn emit_type(&mut self, declaration: &RustDeclaration) {
        let context = self.render_context();
        self.emit_item_tokens(
            RustDeclarationTokens::new(declaration, &context).into_token_stream(),
        );
    }

    fn emit_nota_support(&mut self) {
        if !self.nota_surface.emits_nota() {
            return;
        }
        let context = self.render_context();
        let gate = context.nota_feature_gate();
        self.emit_item_tokens(quote! {
            #gate
            pub use nota_next::{
                NotaDecode, NotaDecodeError, NotaEncode, NotaSource,
            };
        });
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
        self.emit_item_tokens(NewtypeInherentImplTokens::new(declaration).into_token_stream());
    }

    fn emit_root_enum(&mut self, root_enum: &RustEnum) {
        let context = self.render_context();
        self.emit_item_tokens(RustEnumTokens::root(root_enum, &context).into_token_stream());
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
            self.emit_item_tokens(
                EnumPayloadFromImplTokens::new(declaration.name(), variant.name(), payload)
                    .into_token_stream(),
            );
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
        let constructors: Vec<EnumVariantConstructor> = payload_variants
            .iter()
            .filter_map(|variant| {
                let payload = variant.payload()?;
                let constructor = self.enum_variant_constructor_payload(payload, newtypes);
                Some(EnumVariantConstructor::new(
                    variant.name().field_name(),
                    variant.name().as_str().to_owned(),
                    constructor,
                ))
            })
            .collect();
        self.emit_item_tokens(
            EnumVariantConstructorsTokens::new(declaration.name(), &constructors)
                .into_token_stream(),
        );
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
        let context = self.render_context();
        self.emit_item_tokens(
            NotaInherentBridgeTokens::borrowed(name, &context).into_token_stream(),
        );
    }

    fn emit_nota_copy_inherent_bridge(&mut self, name: &str) {
        let context = self.render_context();
        self.emit_item_tokens(NotaInherentBridgeTokens::owned(name, &context).into_token_stream());
    }

    fn emit_nota_root_enum_support(&mut self, root_enum: &RustEnum) {
        if !self.nota_surface.emits_nota() {
            return;
        }
        let context = self.render_context();
        self.emit_item_tokens(
            NotaInherentBridgeTokens::borrowed(root_enum.name().as_str(), &context)
                .into_token_stream(),
        );
        self.blank();
        self.emit_item_tokens(
            NotaRootEnumStringSupportTokens::new(root_enum.name().as_str(), &context)
                .into_token_stream(),
        );
    }

    fn emit_short_headers(&mut self, root_enums: &[RustEnum]) {
        self.emit_item_tokens(ShortHeaderModuleTokens::new(root_enums).into_token_stream());
    }

    /// Emit the basic signal-frame codec: the short-header byte count,
    /// the [`SignalFrameError`] type, the per-root route enums, and the
    /// per-root frame impls (`route` / `short_header` /
    /// `route_from_short_header` / `encode_signal_frame` /
    /// `decode_signal_frame`).
    ///
    /// This is the wire framing every wire-facing target needs — a
    /// separately-generated `WireContract` crate IS the framing that peers
    /// and the owning daemon import and call the codec on. It is gated by
    /// [`RustModuleRenderer::emits_wire_frame`] in [`RustModule::render`], so it
    /// reaches `WireContract`, `SignalRuntime`, and `ComponentRuntime` but
    /// never the internal `NexusRuntime` / `SemaRuntime` planes. The
    /// streaming / observable surface is gated separately by
    /// [`RustModuleRenderer::emits_signal`] plus a declared stream — see
    /// [`RustModuleRenderer::emit_signal_frame_streaming_support`].
    fn emit_signal_frame_codec(&mut self, root_enums: &[RustEnum]) {
        self.emit_item_tokens(quote! {
            const SIGNAL_SHORT_HEADER_BYTE_COUNT: usize = 8;
            #[derive(Clone, Debug, PartialEq, Eq)]
            pub enum SignalFrameError {
                ArchiveEncode,
                ArchiveDecode,
                FrameTooShort { found: usize },
                UnknownHeader { root_enum: &'static str, header: u64 },
                HeaderMismatch { expected: u64, found: u64 },
            }
            impl std::fmt::Display for SignalFrameError {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::ArchiveEncode => formatter.write_str("failed to encode rkyv archive"),
                        Self::ArchiveDecode => formatter.write_str("failed to decode rkyv archive"),
                        Self::FrameTooShort { found } => write!(formatter, "signal frame too short: {found} bytes"),
                        Self::UnknownHeader { root_enum, header } => write!(formatter, "unknown {root_enum} short header 0x{header:016X}"),
                        Self::HeaderMismatch { expected, found } => write!(formatter, "decoded payload header mismatch: expected 0x{expected:016X}, found 0x{found:016X}"),
                    }
                }
            }
            impl std::error::Error for SignalFrameError {}
        });
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
        let context = self.render_context();
        self.emit_item_tokens(RouteEnumTokens::new(root_enum, &context).into_token_stream());
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
        self.emit_item_tokens(RouteImplTokens::new(declaration).into_token_stream());
    }

    fn emit_signal_frame_impl(&mut self, root_enum: &RustEnum) {
        self.emit_item_tokens(SignalFrameImplTokens::new(root_enum).into_token_stream());
    }

    fn streaming_event_payload<'schema>(
        &self,
        root_enums: &'schema [RustEnum],
        streams: &'schema [StreamDeclaration],
    ) -> Option<&'schema TypeReference> {
        let stream = streams.first()?;
        let output = self.root_enum_named(root_enums, "Output")?;
        let output_event_payload = output
            .variants()
            .iter()
            .find(|variant| variant.name().as_str() == "Event")
            .and_then(RustEnumVariant::payload)?;
        if &stream.event == output_event_payload {
            Some(output_event_payload)
        } else {
            None
        }
    }

    fn emit_signal_frame_streaming_support(&mut self, event_payload: &TypeReference) {
        self.emit_item_tokens(
            SignalFrameStreamingSupportTokens::new(event_payload).into_token_stream(),
        );
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
        let mut planes = Vec::new();
        let context = self.render_context();
        if has_signal {
            planes.push(Plane::Signal);
        }
        self.emit_object_name_enum(
            Plane::Signal,
            &signal_roots,
            &signal_actor_variants,
            &context,
        );
        if has_nexus {
            planes.push(Plane::Nexus);
        }
        self.emit_object_name_enum(Plane::Nexus, &nexus_roots, &nexus_actor_variants, &context);
        if has_sema {
            planes.push(Plane::Sema);
        }
        self.emit_object_name_enum(Plane::Sema, &sema_roots, &sema_actor_variants, &context);
        self.emit_item_tokens(TraceSupportTokens::new(planes, &context).into_token_stream());
        self.blank();
    }

    fn emit_object_name_enum(
        &mut self,
        plane: Plane,
        interface_roots: &[TraceInterfaceRoot<'_>],
        actor_variants: &[&'static str],
        context: &RustRenderContext,
    ) {
        if interface_roots.is_empty() && actor_variants.is_empty() {
            return;
        }
        self.emit_item_tokens(
            TraceObjectNameEnumTokens::new(plane, interface_roots, actor_variants, context)
                .into_token_stream(),
        );
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
        let context = self.render_context();
        if self.runtime_planes().emits_signal() {
            self.emit_item_tokens(
                RuntimeCopyNewtypeTokens::new("MessageIdentifier", &context).into_token_stream(),
            );
            if self.nota_surface.emits_nota() {
                self.emit_nota_copy_inherent_bridge("MessageIdentifier");
            }
            self.blank();
        }
        self.emit_item_tokens(
            RuntimeCopyNewtypeTokens::new("OriginRoute", &context).into_token_stream(),
        );
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
        let context = self.render_context();
        self.emit_item_tokens(MessageRootTokens::new(root_enums, &context).into_token_stream());
    }

    fn emit_signal_mail_lifecycle_support(&mut self, root_enums: &[RustEnum]) {
        self.emit_item_tokens(
            SignalMailLifecycleSupportTokens::new(root_enums).into_token_stream(),
        );
        self.blank();
    }

    fn emit_schema_plane_support(&mut self) {
        self.emit_item_tokens(SchemaPlaneSupportTokens.into_token_stream());
    }

    fn emit_plane_envelope(&mut self, name: &str) {
        self.emit_item_tokens(PlaneEnvelopeTokens::new(name).into_token_stream());
    }

    fn emit_plane_namespaces(&mut self, declarations: &[RustDeclaration], root_enums: &[RustEnum]) {
        let active_planes = self.runtime_planes().active_planes();
        for plane in &active_planes {
            let aliases = self.plane_namespace_aliases(*plane, declarations, root_enums);
            if aliases.is_empty() {
                continue;
            }
            self.emit_item_tokens(PlaneNamespaceTokens::new(*plane, aliases).into_token_stream());
            self.blank();
        }
        for plane in &active_planes {
            let source_type_names =
                self.plane_origin_route_source_type_names(*plane, declarations, root_enums);
            for source_type_name in source_type_names {
                self.emit_item_tokens(
                    PlaneOriginRouteConstructorTokens::new(*plane, source_type_name)
                        .into_token_stream(),
                );
                self.blank();
            }
        }
    }

    fn plane_namespace_aliases<'declaration>(
        &self,
        plane: Plane,
        declarations: &'declaration [RustDeclaration],
        root_enums: &'declaration [RustEnum],
    ) -> Vec<PlaneNamespaceAlias<'declaration>> {
        match plane {
            Plane::Signal => plane
                .alias_names()
                .iter()
                .zip(plane.canonical_source_type_names())
                .filter(|(_, source)| self.has_root_enum(root_enums, source))
                .map(|(export, source)| PlaneNamespaceAlias::new(export, source))
                .collect(),
            Plane::Nexus => plane
                .alias_names()
                .iter()
                .zip(plane.canonical_source_type_names())
                .filter(|(_, source)| self.has_type(declarations, source))
                .map(|(export, source)| PlaneNamespaceAlias::new(export, source))
                .collect(),
            Plane::Sema => plane
                .alias_names()
                .iter()
                .zip(self.sema_source_type_names(declarations, root_enums))
                .filter_map(|(export, source)| {
                    source.map(|source| PlaneNamespaceAlias::new(export, source))
                })
                .collect(),
        }
    }

    fn plane_origin_route_source_type_names<'declaration>(
        &self,
        plane: Plane,
        declarations: &'declaration [RustDeclaration],
        root_enums: &'declaration [RustEnum],
    ) -> Vec<&'declaration str> {
        match plane {
            Plane::Signal => Vec::new(),
            Plane::Nexus => plane
                .canonical_source_type_names()
                .iter()
                .copied()
                .filter(|source| self.has_type(declarations, source))
                .collect(),
            Plane::Sema => self
                .sema_source_type_names(declarations, root_enums)
                .into_iter()
                .flatten()
                .collect(),
        }
    }

    fn sema_source_type_names(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) -> [Option<&'static str>; 4] {
        [
            self.sema_write_input_type_name(declarations, root_enums),
            self.sema_write_output_type_name(declarations, root_enums),
            self.sema_read_input_type_name(declarations, root_enums),
            self.sema_read_output_type_name(declarations, root_enums),
        ]
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

    fn emit_runtime_role_trait_impls(
        &mut self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
    ) {
        let mut role_impls = Vec::<RuntimeRoleTraitImpl>::new();

        if self.runtime_planes().emits_nexus() {
            self.push_role_trait_impl_if_local_role_type(
                &mut role_impls,
                declarations,
                root_enums,
                "NexusWork",
                "triad_runtime::NexusWork",
            );
            if let Some(shape) = self.nexus_runner_shape(declarations) {
                self.push_role_trait_impl_if_local_role_type(
                    &mut role_impls,
                    declarations,
                    root_enums,
                    shape.sema_write_input_type(),
                    "triad_runtime::SemaWriteInput",
                );
                self.push_role_trait_impl_if_local_role_type(
                    &mut role_impls,
                    declarations,
                    root_enums,
                    shape.sema_read_input_type(),
                    "triad_runtime::SemaReadInput",
                );
                self.push_role_trait_impl_if_local_role_type(
                    &mut role_impls,
                    declarations,
                    root_enums,
                    shape.effect_command_type(),
                    "triad_runtime::NexusEffectCommand",
                );
                if let Some(effect_result_type) = shape.effect_result_type.as_deref() {
                    self.push_role_trait_impl_if_local_role_type(
                        &mut role_impls,
                        declarations,
                        root_enums,
                        effect_result_type,
                        "triad_runtime::NexusEffectResult",
                    );
                }
            }
        }

        if self.runtime_planes().emits_sema() {
            if let Some(root) = self.sema_write_input_root(declarations, root_enums) {
                self.push_role_trait_impl(
                    declarations,
                    &mut role_impls,
                    root.name().as_str(),
                    "triad_runtime::SemaWriteInput",
                );
            }
            if let Some(root) = self.sema_write_output_root(declarations, root_enums) {
                self.push_role_trait_impl(
                    declarations,
                    &mut role_impls,
                    root.name().as_str(),
                    "triad_runtime::SemaWriteOutput",
                );
            }
            if let Some(root) = self.sema_read_input_root(declarations, root_enums) {
                self.push_role_trait_impl(
                    declarations,
                    &mut role_impls,
                    root.name().as_str(),
                    "triad_runtime::SemaReadInput",
                );
            }
            if let Some(root) = self.sema_read_output_root(declarations, root_enums) {
                self.push_role_trait_impl(
                    declarations,
                    &mut role_impls,
                    root.name().as_str(),
                    "triad_runtime::SemaReadOutput",
                );
            }
        }

        for role_impl in role_impls {
            let type_name = RustTypeTokens::new(&role_impl.type_name);
            let trait_name = RustTypeTokens::new(role_impl.trait_name);
            self.emit_item_tokens(quote! {
                impl #trait_name for #type_name {}
            });
            self.blank();
        }
    }

    fn push_role_trait_impl_if_local_role_type(
        &self,
        role_impls: &mut Vec<RuntimeRoleTraitImpl>,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
        type_name: &str,
        trait_name: &'static str,
    ) {
        if type_name == "std::convert::Infallible" {
            return;
        }
        if self.local_runtime_role_type_exists(declarations, root_enums, type_name) {
            self.push_role_trait_impl(declarations, role_impls, type_name, trait_name);
        }
    }

    fn push_role_trait_impl(
        &self,
        declarations: &[RustDeclaration],
        role_impls: &mut Vec<RuntimeRoleTraitImpl>,
        type_name: &str,
        trait_name: &'static str,
    ) {
        let canonical_type_name = self
            .declaration_alias_target(declarations, type_name)
            .unwrap_or(type_name)
            .to_owned();
        if !role_impls
            .iter()
            .any(|role_impl| role_impl.matches(&canonical_type_name, trait_name))
        {
            role_impls.push(RuntimeRoleTraitImpl::new(
                type_name.to_owned(),
                trait_name,
                canonical_type_name,
            ));
        }
    }

    fn declaration_alias_target<'schema>(
        &self,
        declarations: &'schema [RustDeclaration],
        type_name: &str,
    ) -> Option<&'schema str> {
        declarations
            .iter()
            .find(|declaration| declaration.name().as_str() == type_name)
            .and_then(|declaration| match declaration.value() {
                RustTypeDeclaration::Alias(alias) => match alias.reference() {
                    TypeReference::Plain(target) => Some(target.as_str()),
                    _ => None,
                },
                _ => None,
            })
    }

    fn local_runtime_role_type_exists(
        &self,
        declarations: &[RustDeclaration],
        root_enums: &[RustEnum],
        type_name: &str,
    ) -> bool {
        if self
            .declaration_enum_named(declarations, type_name)
            .is_some()
            || self.root_enum_named(root_enums, type_name).is_some()
        {
            return true;
        }

        self.declaration_alias_target(declarations, type_name)
            .is_some_and(|target| {
                self.declaration_enum_named(declarations, target).is_some()
                    || self.root_enum_named(root_enums, target).is_some()
            })
    }

    fn emit_nexus_runner_next_step_projection(&mut self, shape: &NexusRunnerShape) {
        self.emit_item_tokens(NexusRunnerNextStepProjectionTokens::new(shape).into_token_stream());
        self.blank();
    }

    fn emit_nexus_runner_adapter(&mut self, shape: &NexusRunnerShape) {
        self.emit_item_tokens(NexusRunnerAdapterTokens::new(shape).into_token_stream());
        self.blank();
    }

    fn emit_split_nexus_work_projection(&mut self, projection: &SplitSemaProjection<'_>) {
        let signal_arrived_arms = self.split_signal_arrived_arms(projection);
        let sema_write_arms =
            self.split_output_arms(projection.sema_write_output, projection.signal_output, "SemaWriteOutput");
        let sema_read_arms =
            self.split_output_arms(projection.sema_read_output, projection.signal_output, "SemaReadOutput");
        self.emit_item_tokens(quote! {
            impl nexus::Nexus<nexus::Work> {
                pub fn into_nexus_action(self) -> nexus::Nexus<nexus::Action> {
                    let origin_route = self.origin_route();
                    match self.into_root() {
                        NexusWork::SignalArrived(input) => match input {
                            #(#signal_arrived_arms)*
                        },
                        NexusWork::SemaWriteCompleted(output) => match output {
                            #(#sema_write_arms)*
                        },
                        NexusWork::SemaReadCompleted(output) => match output {
                            #(#sema_read_arms)*
                        },
                        _ => panic!("nexus work cannot project to a generated nexus action"),
                    }
                    .with_origin_route(origin_route)
                }
            }
        });
        self.blank();
    }

    /// The `Input::Variant(payload) => NexusAction::from(...)` arms for the
    /// `SignalArrived` leg: each signal input variant routes to a SEMA write
    /// or read input target, preferring an exact name match before a unique
    /// payload-type fallback.
    fn split_signal_arrived_arms(
        &self,
        projection: &SplitSemaProjection<'_>,
    ) -> Vec<TokenStream> {
        let mut arms = Vec::new();
        for variant in projection.signal_input.variants() {
            let source = RustIdentifier::new(variant.name().as_str());
            if let Some(target_variant) =
                self.exact_target_variant_for_source(variant, projection.sema_write_input)
            {
                let target = RustIdentifier::new(target_variant.name().as_str());
                arms.push(quote! {
                    Input::#source(payload) => NexusAction::from(SemaWriteInput::#target(payload)),
                });
                continue;
            }
            if let Some(target_variant) =
                self.exact_target_variant_for_source(variant, projection.sema_read_input)
            {
                let target = RustIdentifier::new(target_variant.name().as_str());
                arms.push(quote! {
                    Input::#source(payload) => NexusAction::from(SemaReadInput::#target(payload)),
                });
                continue;
            }
            let write_fallback =
                self.fallback_target_variant_for_source(variant, projection.sema_write_input);
            let read_fallback =
                self.fallback_target_variant_for_source(variant, projection.sema_read_input);
            match (write_fallback, read_fallback) {
                (Some(target_variant), None) => {
                    let target = RustIdentifier::new(target_variant.name().as_str());
                    arms.push(quote! {
                        Input::#source(payload) => NexusAction::from(SemaWriteInput::#target(payload)),
                    });
                }
                (None, Some(target_variant)) => {
                    let target = RustIdentifier::new(target_variant.name().as_str());
                    arms.push(quote! {
                        Input::#source(payload) => NexusAction::from(SemaReadInput::#target(payload)),
                    });
                }
                (Some(_), Some(_)) | (None, None) => {}
            }
        }
        arms
    }

    /// The `<SemaOutput>::Variant(payload) => NexusAction::from(Output::Target(payload))`
    /// arms for a SEMA completion leg, keyed by the source-output enum name.
    fn split_output_arms(
        &self,
        sema_output: &RustEnum,
        signal_output: &RustEnum,
        source_enum: &str,
    ) -> Vec<TokenStream> {
        let source_enum = RustIdentifier::new(source_enum);
        let mut arms = Vec::new();
        for variant in sema_output.variants() {
            if let Some(target_variant) = self.target_variant_for_source(variant, signal_output) {
                let source = RustIdentifier::new(variant.name().as_str());
                let target = RustIdentifier::new(target_variant.name().as_str());
                arms.push(quote! {
                    #source_enum::#source(payload) => NexusAction::from(Output::#target(payload)),
                });
            }
        }
        arms
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
        let mut methods = Vec::<TokenStream>::new();
        if has_sema_write {
            methods.push(quote! {
                pub fn into_sema_write_input(self) -> sema::Sema<sema::WriteInput> {
                    let origin_route = self.origin_route();
                    match self.into_root() {
                        NexusAction::CommandSemaWrite(input) => input.with_origin_route(origin_route),
                        _ => panic!("nexus action is not a SEMA write input"),
                    }
                }
            });
        }
        if has_sema_read {
            methods.push(quote! {
                pub fn into_sema_read_input(self) -> sema::Sema<sema::ReadInput> {
                    let origin_route = self.origin_route();
                    match self.into_root() {
                        NexusAction::CommandSemaRead(input) => input.with_origin_route(origin_route),
                        _ => panic!("nexus action is not a SEMA read input"),
                    }
                }
            });
        }
        if has_signal {
            methods.push(quote! {
                pub fn into_signal_output(self) -> signal::Signal<signal::Output> {
                    let origin_route = self.origin_route();
                    match self.into_root() {
                        NexusAction::ReplyToSignal(output) => output.with_origin_route(origin_route),
                        _ => panic!("nexus action is not a signal reply"),
                    }
                }
            });
        }
        self.emit_item_tokens(quote! {
            impl nexus::Nexus<nexus::Action> {
                #(#methods)*
            }
        });
        self.blank();
    }

    fn emit_split_sema_output_projection(&mut self, plane_alias: &str, type_name: &str) {
        let plane_alias = RustIdentifier::new(plane_alias);
        self.emit_item_tokens(quote! {
            impl sema::Sema<sema::#plane_alias> {
                pub fn into_nexus_work(self) -> nexus::Nexus<nexus::Work> {
                    let origin_route = self.origin_route();
                    NexusWork::from(self.into_root()).with_origin_route(origin_route)
                }
            }
        });
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
        self.emit_item_tokens(quote! {
            pub trait UpgradeFrom<Previous>: Sized {
                type Error;
                fn upgrade_from(previous: Previous) -> Result<Self, Self::Error>;
            }
            pub trait AcceptPrevious<Previous>: UpgradeFrom<Previous> {
                fn accept_previous(previous: Previous) -> Result<Self, Self::Error> {
                    Self::upgrade_from(previous)
                }
            }
            impl<Current, Previous> AcceptPrevious<Previous> for Current where
                Current: UpgradeFrom<Previous>
            {}
        });
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
            self.emit_item_tokens(
                SignalEngineTraitTokens::new(emits_concrete_signal_engine).into_token_stream(),
            );
            self.blank();
        }
        if emits_nexus_engine {
            self.emit_item_tokens(
                NexusEngineTraitTokens::new(nexus_runner_shape.as_ref()).into_token_stream(),
            );
            self.blank();
            if let Some(shape) = nexus_runner_shape.as_ref() {
                self.emit_nexus_runner_adapter(shape);
            }
        }
        if emits_sema_engine {
            self.emit_item_tokens(
                SemaEngineTraitTokens::new(emits_sema_apply, emits_sema_observe)
                    .into_token_stream(),
            );
            self.blank();
        }
    }

    fn emit_actor_lifecycle_support(&mut self) {
        self.emit_item_tokens(ActorLifecycleSupportTokens.into_token_stream());
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
