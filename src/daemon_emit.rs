//! Daemon-module emission — the source-visible `src/schema/daemon.rs`.
//!
//! This is the `triad_main!` emitter from designer report 542: instead of a
//! literal macro, schema-rust-next emits a per-component, source-visible
//! `src/schema/daemon.rs` carrying the uniform daemon skeleton (the
//! `ComponentDaemon` hook trait, `DaemonCommand` argv parsing, the generated
//! runtime struct + its decode -> execute -> encode `handle_stream` spine, the
//! single/multi listener selection, and the `ExitReport`-based exit body). The
//! component hand-writes only `impl ComponentDaemon` (the `1488` escape
//! hatches: `Configuration` / `Engine` / `Error` / `PROCESS_NAME` + the
//! required `build_runtime`, plus the typed working-input handler and the
//! residual streaming/meta hooks) and a schema-side [`NexusDaemonShape`].
//!
//! Streaming follows option B: when the schema declares a stream, the emitter
//! generates the daemon-side subscription registry + publish wiring from the
//! stream metadata (reusing `triad_runtime`'s `SubscriptionRegistry` +
//! `SubscriptionEventPublisher`), so a declared stream becomes emitted daemon
//! plumbing rather than a hand-written subscription hub.
//!
//! Rust syntax is built as `proc_macro2` token streams through `quote!` and
//! pretty-printed once at the boundary, matching the token-first discipline of
//! the main emitter (`lib.rs`) and `migration.rs`. Each emitted section is its
//! own data-bearing `ToTokens` noun; the daemon emitter builds no Rust as
//! strings. The `// @generated` header is prepended as text because
//! `prettyplease` does not preserve non-doc comments through a parse/unparse
//! round-trip.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use schema_next::Schema;

use crate::{GeneratedFile, RustCode};

/// The schema-side declaration that turns the daemon emitter ON for a
/// component, sibling to the in-emitter `NexusRunnerShape`.
///
/// It carries the data the design says is *not* derivable from the wire
/// contract (fork 2 of report 542): the OS process name, the working listener
/// tier's contract module, and the optional owner-only meta tier with its
/// socket file mode. The streaming wiring is derived from the schema's declared
/// streams, not from this shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NexusDaemonShape {
    process_name: String,
    working_tier: WorkingListenerTier,
    meta_tier: Option<MetaListenerTier>,
}

impl NexusDaemonShape {
    pub fn new(process_name: impl Into<String>, working_tier: WorkingListenerTier) -> Self {
        Self {
            process_name: process_name.into(),
            working_tier,
            meta_tier: None,
        }
    }

    pub fn with_meta_tier(mut self, meta_tier: MetaListenerTier) -> Self {
        self.meta_tier = Some(meta_tier);
        self
    }

    pub fn process_name(&self) -> &str {
        &self.process_name
    }

    pub fn working_tier(&self) -> &WorkingListenerTier {
        &self.working_tier
    }

    pub fn meta_tier(&self) -> Option<&MetaListenerTier> {
        self.meta_tier.as_ref()
    }

    fn is_multi_listener(&self) -> bool {
        self.meta_tier.is_some()
    }
}

/// The peer-callable working listener tier: the signal contract module whose
/// emitted `Input` / `Output` roots the decode -> execute -> encode spine drives.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkingListenerTier {
    contract_module: String,
}

impl WorkingListenerTier {
    pub fn new(contract_module: impl Into<String>) -> Self {
        Self {
            contract_module: contract_module.into(),
        }
    }

    pub fn contract_module(&self) -> &str {
        &self.contract_module
    }
}

/// The owner-only meta listener tier: the owner-only socket file mode applied
/// at bind time. The meta wire codec is the component's escape hatch — the
/// emitter routes the meta socket to a single component-provided
/// `handle_meta_stream` method rather than emitting a second frame spine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetaListenerTier {
    socket_mode: SocketModeBits,
}

impl MetaListenerTier {
    pub fn new(socket_mode: SocketModeBits) -> Self {
        Self { socket_mode }
    }

    pub fn socket_mode(&self) -> SocketModeBits {
        self.socket_mode
    }
}

/// A Unix socket file mode in octal-equivalent bits, e.g. `0o600` owner-only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocketModeBits {
    bits: u32,
}

impl SocketModeBits {
    pub const fn new(bits: u32) -> Self {
        Self { bits }
    }

    pub fn bits(self) -> u32 {
        self.bits
    }

    /// The `0o600`-form octal literal token the emitted `SocketMode::new`
    /// default uses. Kept as the octal text the source review expects, not
    /// the decimal a plain integer literal would print.
    fn octal_literal(self) -> syn::LitInt {
        syn::LitInt::new(&format!("0o{:o}", self.bits), Span::call_site())
    }
}

/// Renders the full `src/schema/daemon.rs` source for a component from its
/// [`NexusDaemonShape`] plus whether the schema declares a stream.
pub struct DaemonModule {
    shape: NexusDaemonShape,
    emits_stream: bool,
    generator_name: String,
}

impl DaemonModule {
    pub fn new(
        shape: NexusDaemonShape,
        schema: &Schema,
        generator_name: impl Into<String>,
    ) -> Self {
        let emits_stream = !schema.streams().is_empty();
        Self {
            shape,
            emits_stream,
            generator_name: generator_name.into(),
        }
    }

    pub fn to_generated_file(&self) -> GeneratedFile {
        GeneratedFile {
            path: "src/schema/daemon.rs".to_owned(),
            code: RustCode(self.render()),
        }
    }

    /// The single `// @generated` header line, kept as text because
    /// `prettyplease` drops non-doc comments through its parse/unparse pass.
    fn header(&self) -> String {
        format!("// @generated by {}\n", self.generator_name)
    }

    /// Build the whole module as one `TokenStream`, then route it through the
    /// `syn::parse2` + `prettyplease` seam exactly like the main emitter's
    /// `emit_item_tokens` and `migration.rs`. Malformed emitted Rust fails
    /// here, at emission time, rather than in the consumer build (fix M2).
    fn render(&self) -> String {
        let body = DaemonModuleBody::new(&self.shape, self.emits_stream);
        let file = syn::parse2::<syn::File>(body.into_token_stream())
            .expect("generated daemon tokens parse");
        let mut source = self.header();
        source.push_str(prettyplease::unparse(&file).trim_end());
        source.push('\n');
        source
    }
}

/// The whole daemon-module body as a composition of per-section `ToTokens`
/// nouns. Owns the daemon shape it is rendering against and whether the schema
/// declared a stream (option B).
struct DaemonModuleBody<'shape> {
    shape: &'shape NexusDaemonShape,
    emits_stream: bool,
}

impl<'shape> DaemonModuleBody<'shape> {
    fn new(shape: &'shape NexusDaemonShape, emits_stream: bool) -> Self {
        Self {
            shape,
            emits_stream,
        }
    }
}

impl ToTokens for DaemonModuleBody<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let imports = DaemonImportsTokens::new(self.shape, self.emits_stream);
        let hook_trait = ComponentDaemonTraitTokens::new(self.shape, self.emits_stream);
        let command = DaemonCommandTokens;
        let binder = DaemonBinderTokens::new(self.shape);
        let transport = WorkingTransportTokens;
        let subscriptions = self
            .emits_stream
            .then_some(EmittedSubscriptionsTokens);
        let runtime = GeneratedDaemonRuntimeTokens::new(self.shape, self.emits_stream);
        let error = DaemonErrorTokens::new(self.shape);
        let exit = DaemonEntryTokens;
        quote! {
            #imports
            #hook_trait
            #command
            #binder
            #transport
            #subscriptions
            #runtime
            #error
            #exit
        }
        .to_tokens(tokens);
    }
}

/// The `use` preamble: the always-present `std`/`thiserror` imports, the
/// single- vs multi-listener `triad_runtime` set, the working contract
/// `Input`/`Output`/`SignalFrameError`, and the streaming imports when the
/// schema declares a stream.
struct DaemonImportsTokens<'shape> {
    shape: &'shape NexusDaemonShape,
    emits_stream: bool,
}

impl<'shape> DaemonImportsTokens<'shape> {
    fn new(shape: &'shape NexusDaemonShape, emits_stream: bool) -> Self {
        Self {
            shape,
            emits_stream,
        }
    }
}

impl ToTokens for DaemonImportsTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let working = syn::Ident::new(
            self.shape.working_tier().contract_module(),
            Span::call_site(),
        );
        let runtime_imports = if self.shape.is_multi_listener() {
            quote! {
                use triad_runtime::{
                    ArgumentError, ComponentArgument, ComponentCommand, DaemonConfiguration,
                    ExitReport, FrameBody, FrameError, LengthPrefixedCodec, ListenerError,
                    ListenerSocket, MultiListenerDaemon, MultiListenerDaemonError,
                    MultiListenerRuntime, RequestErrorLog, SocketMode,
                };
            }
        } else {
            quote! {
                use triad_runtime::{
                    ArgumentError, ComponentArgument, ComponentCommand, DaemonConfiguration,
                    DaemonRuntime, ExitReport, FrameBody, FrameError, LengthPrefixedCodec,
                    ListenerError, RequestErrorLog, SingleListenerDaemon,
                    SingleListenerDaemonError,
                };
            }
        };
        let streaming_imports = self.emits_stream.then(|| {
            quote! {
                use triad_runtime::{
                    SubscriptionEventPublisher, SubscriptionRegistry, SubscriptionToken,
                };
                use signal_frame::SubscriptionTokenInner;
            }
        });
        quote! {
            use std::os::unix::net::UnixStream;

            use thiserror::Error;
            #runtime_imports

            use crate::schema::#working::{Input, Output, SignalFrameError};
            #streaming_imports
        }
        .to_tokens(tokens);
    }
}

/// The `ComponentDaemon` hook trait — the only daemon code the component
/// hand-writes (record 1488 escape hatches). Owns whether the meta tier and
/// the streaming hooks are emitted.
struct ComponentDaemonTraitTokens<'shape> {
    shape: &'shape NexusDaemonShape,
    emits_stream: bool,
}

impl<'shape> ComponentDaemonTraitTokens<'shape> {
    fn new(shape: &'shape NexusDaemonShape, emits_stream: bool) -> Self {
        Self {
            shape,
            emits_stream,
        }
    }
}

impl ToTokens for ComponentDaemonTraitTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let streaming_associated_types = self.emits_stream.then(|| {
            quote! {
                type SubscriptionToken: SubscriptionToken;
                type SubscriptionFilter: Clone;
                /// The stream event payload. The rkyv `Archive` + high-level
                /// `Serialize` bounds are what the emitted publisher needs to
                /// encode the subscription-event frame; they mirror
                /// `signal_frame`'s own `StreamingFrame::encode` bounds so the
                /// event rides the wire.
                type StreamEvent: Clone
                    + rkyv::Archive
                    + for<'archive> rkyv::Serialize<
                        rkyv::api::high::HighSerializer<
                            rkyv::util::AlignedVec,
                            rkyv::ser::allocator::ArenaHandle<'archive>,
                            rkyv::rancor::Error,
                        >,
                    >;
            }
        });
        let meta_hook = self.shape.meta_tier().is_some().then(|| {
            quote! {
                /// Serve one owner-only meta stream end to end (read, handle, write).
                /// The meta wire codec is component-owned, so this is a full escape hatch.
                fn handle_meta_stream(engine: &Self::Engine, stream: UnixStream) -> Result<(), Self::Error>;
            }
        });
        let streaming_hooks = self.emits_stream.then(|| {
            quote! {
                /// The subscription filter an `Input` opens, if any. `None` means the
                /// input does not open a stream.
                fn subscription_filter(input: &Input) -> Option<Self::SubscriptionFilter>;

                /// The stream token an `Output` carries when it acknowledges a new
                /// subscription, if any.
                fn subscription_token(output: &Output) -> Option<Self::SubscriptionToken>;

                /// The stream event a committed `Output` publishes, if any.
                fn published_event(engine: &Self::Engine, output: &Output) -> Result<Option<Self::StreamEvent>, Self::Error>;

                /// Whether a stream event matches a registered subscription filter.
                fn event_matches_filter(filter: &Self::SubscriptionFilter, event: &Self::StreamEvent) -> bool;

                /// The short header constant for stream subscription-event frames, so
                /// the emitted publisher stamps the same header the contract codec uses.
                fn subscription_event_short_header() -> u64;
            }
        });
        quote! {
            /// The component hook surface for the emitted daemon — the only daemon
            /// code the component hand-writes (record 1488 escape hatches).
            ///
            /// The component declares its `Configuration` / `Engine` / `Error` types
            /// and `PROCESS_NAME`, and provides the REQUIRED `build_runtime` (the
            /// emitter cannot know how to open the component's Store/Engine) plus the
            /// typed working-input handler. Streaming hooks are residual: when the
            /// schema declares a stream the registry/publish plumbing is emitted, and
            /// the component supplies only filter + event policy. The meta tier is the
            /// owner-only escape hatch — the component owns its full read/handle/write.
            pub trait ComponentDaemon: Sized + 'static {
                type Configuration: DaemonConfiguration;
                type ConfigurationError: std::error::Error;
                type Engine;
                type Error: std::fmt::Display + From<FrameError> + From<SignalFrameError> + From<ListenerError>;
                #streaming_associated_types

                const PROCESS_NAME: &'static str;

                /// Load the binary rkyv `Configuration` from the daemon's single argument.
                fn load_configuration(path: &std::path::Path) -> Result<Self::Configuration, Self::ConfigurationError>;

                /// Open the component's durable Store and construct its Engine.
                fn build_runtime(configuration: &Self::Configuration) -> Result<Self::Engine, Self::Error>;

                /// Lifecycle: called once before the listener serves, once after it stops.
                fn start(engine: &mut Self::Engine) -> Result<(), Self::Error> {
                    let _ = engine;
                    Ok(())
                }

                fn stop(engine: &mut Self::Engine) -> Result<(), Self::Error> {
                    let _ = engine;
                    Ok(())
                }

                /// Run one decoded working `Input` through the engine and return the
                /// `Output` root to encode back to the caller.
                fn handle_working_input(engine: &Self::Engine, input: Input) -> Result<Output, Self::Error>;
                #meta_hook
                #streaming_hooks
            }
        }
        .to_tokens(tokens);
    }
}

/// `DaemonCommand`: argv -> binary `Configuration` -> the bound daemon. The
/// single-argument rule: exactly one argument, a signal-encoded (rkyv)
/// configuration file. The section carries no per-component data.
struct DaemonCommandTokens;

impl ToTokens for DaemonCommandTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            /// argv -> binary `Configuration` -> the bound daemon. The single-argument
            /// rule: exactly one argument, a signal-encoded (rkyv) configuration file.
            pub struct DaemonCommand<Daemon: ComponentDaemon> {
                command: ComponentCommand,
                daemon: std::marker::PhantomData<fn() -> Daemon>,
            }

            impl<Daemon: ComponentDaemon> DaemonCommand<Daemon> {
                pub fn from_environment() -> Self {
                    Self {
                        command: ComponentCommand::from_environment(),
                        daemon: std::marker::PhantomData,
                    }
                }

                pub fn from_arguments<Arguments, Argument>(arguments: Arguments) -> Self
                where
                    Arguments: IntoIterator<Item = Argument>,
                    Argument: Into<String>,
                {
                    Self {
                        command: ComponentCommand::from_arguments(arguments),
                        daemon: std::marker::PhantomData,
                    }
                }

                pub fn configuration(&self) -> Result<Daemon::Configuration, DaemonError<Daemon>> {
                    match self.command.signal_file_argument()? {
                        ComponentArgument::SignalFile(file) => {
                            Daemon::load_configuration(file.as_path()).map_err(DaemonError::Configuration)
                        }
                        ComponentArgument::InlineNota(_) | ComponentArgument::NotaFile(_) => {
                            Err(DaemonError::Argument(ArgumentError::ExpectedSignalFile))
                        }
                    }
                }

                pub fn run(&self) -> Result<(), DaemonError<Daemon>> {
                    Daemon::bind(self.configuration()?)?.run().map_err(DaemonError::from)
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// The `DaemonBinder` default-method trait: builds the engine and selects the
/// listener tiers (single vs multi from the schema shape), returning a runner
/// the `DaemonCommand` drives. Owns the daemon shape (multi vs single, and the
/// meta socket mode for the multi bind body).
struct DaemonBinderTokens<'shape> {
    shape: &'shape NexusDaemonShape,
}

impl<'shape> DaemonBinderTokens<'shape> {
    fn new(shape: &'shape NexusDaemonShape) -> Self {
        Self { shape }
    }
}

impl ToTokens for DaemonBinderTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let bind_method = if self.shape.is_multi_listener() {
            let socket_mode = self
                .shape
                .meta_tier()
                .expect("multi-listener daemon has a meta tier")
                .socket_mode()
                .octal_literal();
            quote! {
                fn bind(
                    configuration: Self::Configuration,
                ) -> Result<MultiListenerDaemon<GeneratedDaemonRuntime<Self>>, DaemonError<Self>> {
                    let engine = Self::build_runtime(&configuration).map_err(DaemonError::Component)?;
                    let runtime = GeneratedDaemonRuntime::<Self>::new(engine);
                    let mut sockets = vec![ListenerSocket::new(
                        ListenerTier::Working,
                        configuration.socket_path().to_path_buf(),
                    )];
                    if let Some(meta_socket_path) = configuration.meta_socket_path() {
                        let socket_mode = configuration
                            .meta_socket_mode()
                            .unwrap_or_else(|| SocketMode::new(#socket_mode));
                        sockets.push(
                            ListenerSocket::new(ListenerTier::Meta, meta_socket_path.to_path_buf())
                                .with_socket_mode(socket_mode),
                        );
                    }
                    Ok(MultiListenerDaemon::new(
                        sockets,
                        runtime,
                        RequestErrorLog::new(Self::PROCESS_NAME),
                    ))
                }
            }
        } else {
            quote! {
                fn bind(
                    configuration: Self::Configuration,
                ) -> Result<SingleListenerDaemon<GeneratedDaemonRuntime<Self>>, DaemonError<Self>> {
                    let engine = Self::build_runtime(&configuration).map_err(DaemonError::Component)?;
                    let runtime = GeneratedDaemonRuntime::<Self>::new(engine);
                    Ok(SingleListenerDaemon::new(
                        configuration.socket_path().to_path_buf(),
                        runtime,
                        RequestErrorLog::new(Self::PROCESS_NAME),
                    ))
                }
            }
        };
        quote! {
            /// The bound daemon constructor on the component trait: builds the engine,
            /// selects the listener tiers (single vs multi from the schema shape), and
            /// returns a runner the `DaemonCommand` drives. The component never writes
            /// this by hand — it is emitted as a default method on `ComponentDaemon`.
            pub trait DaemonBinder: ComponentDaemon {
                #bind_method
            }

            impl<Daemon: ComponentDaemon> DaemonBinder for Daemon {}
        }
        .to_tokens(tokens);
    }
}

/// The working-tier wire transport over one accepted stream: a length-prefixed
/// envelope around the schema-emitted signal frame codec. Emitted (not imported
/// from a hand-written `transport.rs`) so the daemon spine is self-contained.
/// The section carries no per-component data.
struct WorkingTransportTokens;

impl ToTokens for WorkingTransportTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            /// The working-tier wire transport over one accepted stream: a
            /// length-prefixed envelope around the schema-emitted signal frame codec.
            struct WorkingTransport {
                stream: UnixStream,
            }

            impl WorkingTransport {
                fn new(stream: UnixStream) -> Self {
                    Self { stream }
                }

                fn read_frame(&mut self) -> Result<Vec<u8>, FrameError> {
                    Ok(LengthPrefixedCodec::default()
                        .read_body(&mut self.stream)?
                        .into_bytes())
                }

                fn write_frame(&mut self, frame: Vec<u8>) -> Result<(), FrameError> {
                    use std::io::Write;
                    LengthPrefixedCodec::default()
                        .write_body(&mut self.stream, &FrameBody::new(frame))?;
                    self.stream.flush()?;
                    Ok(())
                }

                fn try_clone_stream(&self) -> Result<UnixStream, FrameError> {
                    self.stream.try_clone().map_err(FrameError::Io)
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// The emitted option-B subscription plumbing: the `EmittedSubscriptions`
/// registry + `SubscriptionState` + publish wiring, reusing `triad_runtime`'s
/// subscription primitives. The section carries no per-component data (the
/// generic `Daemon` parameter threads the component types through).
struct EmittedSubscriptionsTokens;

impl ToTokens for EmittedSubscriptionsTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            /// The emitted option-B subscription plumbing. It reuses `triad_runtime`'s
            /// `SubscriptionRegistry` + `SubscriptionEventPublisher` (the runtime owns
            /// token registries and frame construction) and adds the per-subscriber
            /// writer map + delivery. This replaces a hand-written `SubscriptionHub`;
            /// the component supplies only filter + event policy through `ComponentDaemon`.
            pub struct EmittedSubscriptions<Daemon: ComponentDaemon> {
                state: std::sync::Mutex<SubscriptionState<Daemon>>,
            }

            struct SubscriptionState<Daemon: ComponentDaemon> {
                registry: SubscriptionRegistry<Daemon::SubscriptionToken, Daemon::SubscriptionFilter>,
                writers: std::collections::HashMap<SubscriptionTokenInner, UnixStream>,
                publisher: SubscriptionEventPublisher<Input, Output, Daemon::StreamEvent>,
            }

            impl<Daemon: ComponentDaemon> Default for EmittedSubscriptions<Daemon> {
                fn default() -> Self {
                    Self {
                        state: std::sync::Mutex::new(SubscriptionState {
                            registry: SubscriptionRegistry::new(),
                            writers: std::collections::HashMap::new(),
                            publisher: SubscriptionEventPublisher::acceptor(
                                signal_frame::ShortHeader::new(Daemon::subscription_event_short_header()),
                                signal_frame::SessionEpoch::new(1),
                            ),
                        }),
                    }
                }
            }

            impl<Daemon: ComponentDaemon> EmittedSubscriptions<Daemon> {
                fn register(
                    &self,
                    token: Daemon::SubscriptionToken,
                    filter: Daemon::SubscriptionFilter,
                    writer: UnixStream,
                ) {
                    let mut state = self.state.lock().expect("subscription state lock");
                    state.registry.register_token(token, filter);
                    state.writers.insert(token.into_inner(), writer);
                }

                /// Publish a committed stream event to every matching subscriber.
                ///
                /// The body reborrows the `MutexGuard` once into the owned state
                /// (`let state = &mut *guard;`) so the disjoint field borrows below
                /// (`registry` shared, `publisher` exclusive) split cleanly; going
                /// through the `MutexGuard` `Deref` directly would conflict because
                /// deref yields the whole struct. (The rationale is a method doc
                /// comment, not an inner-statement comment, so it survives the
                /// `prettyplease` pass without tripping `unused_doc_comments` in the
                /// consumer crate.)
                fn publish(&self, event: Daemon::StreamEvent) -> Result<usize, FrameError> {
                    let mut guard = self.state.lock().expect("subscription state lock");
                    let state = &mut *guard;
                    let mut frames = Vec::new();
                    let publisher = &mut state.publisher;
                    let registry = &state.registry;
                    registry.publish_matching(
                        &event,
                        |filter, event| Daemon::event_matches_filter(filter, event),
                        |token, event| {
                            frames.push((token.into_inner(), publisher.publish(token, event.clone())));
                        },
                    );
                    let mut delivered = 0;
                    let mut stale = Vec::new();
                    for (token, frame) in frames {
                        match SubscriptionWriters::<Daemon>::deliver(&mut state.writers, token, frame) {
                            Ok(true) => delivered += 1,
                            Ok(false) => {}
                            Err(_error) => stale.push(token),
                        }
                    }
                    for token in stale {
                        state.writers.remove(&token);
                        state.registry.unregister(Daemon::SubscriptionToken::from_inner(token));
                    }
                    Ok(delivered)
                }
            }

            /// The per-subscriber writer map. Delivery is a method on the map noun
            /// so the verb lives on the data it reads and writes.
            trait SubscriptionWriters<Daemon: ComponentDaemon> {
                fn deliver(
                    &mut self,
                    token: SubscriptionTokenInner,
                    frame: signal_frame::StreamingFrame<Input, Output, Daemon::StreamEvent>,
                ) -> Result<bool, FrameError>;
            }

            impl<Daemon: ComponentDaemon> SubscriptionWriters<Daemon>
                for std::collections::HashMap<SubscriptionTokenInner, UnixStream>
            {
                fn deliver(
                    &mut self,
                    token: SubscriptionTokenInner,
                    frame: signal_frame::StreamingFrame<Input, Output, Daemon::StreamEvent>,
                ) -> Result<bool, FrameError> {
                    use std::io::Write;
                    let Some(writer) = self.get_mut(&token) else {
                        return Ok(false);
                    };
                    let bytes = frame
                        .encode()
                        .map_err(|_| FrameError::Io(std::io::Error::other("subscription frame encode")))?;
                    LengthPrefixedCodec::default().write_body(writer, &FrameBody::new(bytes))?;
                    writer.flush().map_err(FrameError::Io)?;
                    Ok(true)
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// The generated runtime struct that owns the engine and (option B) the emitted
/// subscription registry. Its `handle_stream` IS the decode -> execute ->
/// encode spine. Owns the daemon shape (multi vs single) and whether the schema
/// declared a stream.
struct GeneratedDaemonRuntimeTokens<'shape> {
    shape: &'shape NexusDaemonShape,
    emits_stream: bool,
}

impl<'shape> GeneratedDaemonRuntimeTokens<'shape> {
    fn new(shape: &'shape NexusDaemonShape, emits_stream: bool) -> Self {
        Self {
            shape,
            emits_stream,
        }
    }
}

impl ToTokens for GeneratedDaemonRuntimeTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let subscriptions_field = self
            .emits_stream
            .then(|| quote! { subscriptions: EmittedSubscriptions<Daemon>, });
        let subscriptions_init = self
            .emits_stream
            .then(|| quote! { subscriptions: EmittedSubscriptions::default(), });
        let subscription_writer = self
            .emits_stream
            .then(|| quote! { let subscription_writer = transport.try_clone_stream()?; });
        let subscription_filter = self
            .emits_stream
            .then(|| quote! { let subscription_filter = Daemon::subscription_filter(&input); });
        let subscription_publish = self.emits_stream.then(|| {
            quote! {
                if let (Some(filter), Some(token)) =
                    (subscription_filter, Daemon::subscription_token(&output))
                {
                    self.subscriptions.register(token, filter, subscription_writer);
                }
                if let Some(event) = Daemon::published_event(&self.engine, &output)? {
                    self.subscriptions.publish(event)?;
                }
            }
        });
        let runtime_impl = if self.shape.is_multi_listener() {
            quote! {
                impl<Daemon: ComponentDaemon> MultiListenerRuntime for GeneratedDaemonRuntime<Daemon> {
                    type Listener = ListenerTier;
                    type StartError = Daemon::Error;
                    type StopError = Daemon::Error;
                    type RequestError = Daemon::Error;

                    fn start(&mut self) -> Result<(), Self::StartError> {
                        Daemon::start(&mut self.engine)
                    }

                    fn stop(&mut self) -> Result<(), Self::StopError> {
                        Daemon::stop(&mut self.engine)
                    }

                    fn handle_stream(
                        &mut self,
                        listener: ListenerTier,
                        stream: UnixStream,
                    ) -> Result<(), Self::RequestError> {
                        match listener {
                            ListenerTier::Working => self.handle_working_stream(stream),
                            ListenerTier::Meta => Daemon::handle_meta_stream(&self.engine, stream),
                        }
                    }
                }
            }
        } else {
            quote! {
                impl<Daemon: ComponentDaemon> DaemonRuntime for GeneratedDaemonRuntime<Daemon> {
                    type StartError = Daemon::Error;
                    type StopError = Daemon::Error;
                    type RequestError = Daemon::Error;

                    fn start(&mut self) -> Result<(), Self::StartError> {
                        Daemon::start(&mut self.engine)
                    }

                    fn stop(&mut self) -> Result<(), Self::StopError> {
                        Daemon::stop(&mut self.engine)
                    }

                    fn handle_stream(&mut self, stream: UnixStream) -> Result<(), Self::RequestError> {
                        self.handle_working_stream(stream)
                    }
                }
            }
        };
        let listener_tier_enum = ListenerTierEnumTokens;
        let runtime_impl = if self.shape.is_multi_listener() {
            quote! {
                #listener_tier_enum
                #runtime_impl
            }
        } else {
            runtime_impl
        };
        quote! {
            /// The generated runtime struct that owns the engine and (option B) the
            /// emitted subscription registry. Its `handle_stream` IS the decode ->
            /// execute -> encode spine.
            pub struct GeneratedDaemonRuntime<Daemon: ComponentDaemon> {
                engine: Daemon::Engine,
                #subscriptions_field
            }

            impl<Daemon: ComponentDaemon> GeneratedDaemonRuntime<Daemon> {
                fn new(engine: Daemon::Engine) -> Self {
                    Self {
                        engine,
                        #subscriptions_init
                    }
                }

                fn handle_working_stream(&self, stream: UnixStream) -> Result<(), Daemon::Error> {
                    let mut transport = WorkingTransport::new(stream);
                    #subscription_writer
                    let frame = transport.read_frame()?;
                    let (_route, input) = Input::decode_signal_frame(&frame)?;
                    #subscription_filter
                    let output = Daemon::handle_working_input(&self.engine, input)?;
                    transport.write_frame(output.encode_signal_frame()?)?;
                    #subscription_publish
                    Ok(())
                }
            }

            #runtime_impl
        }
        .to_tokens(tokens);
    }
}

/// The `ListenerTier` enum: which authority-tiered socket an arriving stream
/// belongs to. Emitted only on the multi-listener path. Carries no
/// per-component data.
struct ListenerTierEnumTokens;

impl ToTokens for ListenerTierEnumTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            /// Which authority-tiered socket an arriving stream belongs to.
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub enum ListenerTier {
                Working,
                Meta,
            }

            impl std::fmt::Display for ListenerTier {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::Working => formatter.write_str("working"),
                        Self::Meta => formatter.write_str("meta"),
                    }
                }
            }
        }
        .to_tokens(tokens);
    }
}

/// The emitted `DaemonError`: argv, configuration, listener, and the component
/// error, plus the `From` conversions. Owns the daemon shape, which selects the
/// single- vs multi-listener `From` impl.
struct DaemonErrorTokens<'shape> {
    shape: &'shape NexusDaemonShape,
}

impl<'shape> DaemonErrorTokens<'shape> {
    fn new(shape: &'shape NexusDaemonShape) -> Self {
        Self { shape }
    }
}

impl ToTokens for DaemonErrorTokens<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let listener_from_impl = if self.shape.is_multi_listener() {
            quote! {
                impl<Daemon: ComponentDaemon> From<MultiListenerDaemonError<Daemon::Error, Daemon::Error>>
                    for DaemonError<Daemon>
                {
                    fn from(error: MultiListenerDaemonError<Daemon::Error, Daemon::Error>) -> Self {
                        match error {
                            MultiListenerDaemonError::Listener(error) => Self::Listener(error),
                            MultiListenerDaemonError::Start(error) | MultiListenerDaemonError::Stop(error) => {
                                Self::Component(error)
                            }
                        }
                    }
                }
            }
        } else {
            quote! {
                impl<Daemon: ComponentDaemon> From<SingleListenerDaemonError<Daemon::Error, Daemon::Error>>
                    for DaemonError<Daemon>
                {
                    fn from(error: SingleListenerDaemonError<Daemon::Error, Daemon::Error>) -> Self {
                        match error {
                            SingleListenerDaemonError::Listener(error) => Self::Listener(error),
                            SingleListenerDaemonError::Start(error) | SingleListenerDaemonError::Stop(error) => {
                                Self::Component(error)
                            }
                        }
                    }
                }
            }
        };
        quote! {
            /// The emitted daemon error: argv, configuration, listener, and the
            /// component error. The component's own error rides the `Component` arm.
            #[derive(Debug, Error)]
            pub enum DaemonError<Daemon: ComponentDaemon> {
                #[error("daemon argument error: {0}")]
                Argument(ArgumentError),

                #[error("daemon configuration error: {0}")]
                Configuration(Daemon::ConfigurationError),

                #[error("daemon listener error: {0}")]
                Listener(ListenerError),

                #[error("component error: {0}")]
                Component(Daemon::Error),
            }

            impl<Daemon: ComponentDaemon> From<ArgumentError> for DaemonError<Daemon> {
                fn from(error: ArgumentError) -> Self {
                    Self::Argument(error)
                }
            }

            #listener_from_impl
        }
        .to_tokens(tokens);
    }
}

/// The component-agnostic exit body: `DaemonEntry::run_to_exit_code`, called
/// from the component binary's `fn main`. Carries no per-component data.
struct DaemonEntryTokens;

impl ToTokens for DaemonEntryTokens {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            /// The component-agnostic exit body. The component's binary calls
            /// `<SpiritDaemon as DaemonEntry>::run_to_exit_code()` from `fn main`.
            pub trait DaemonEntry: ComponentDaemon {
                fn run_to_exit_code() -> std::process::ExitCode {
                    ExitReport::new(Self::PROCESS_NAME)
                        .from_result(DaemonCommand::<Self>::from_environment().run())
                }
            }

            impl<Daemon: ComponentDaemon> DaemonEntry for Daemon {}
        }
        .to_tokens(tokens);
    }
}
