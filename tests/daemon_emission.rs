use schema_rust_next::{
    DaemonModule, MetaListenerTier, NexusDaemonShape, SocketModeBits, WorkingListenerTier,
};

mod support;

use support::FixtureSchema;

const OWNER_ONLY_SOCKET_MODE: u32 = 0o600;

fn assert_code_contains(code: &str, expected: &str) {
    let compact = |text: &str| {
        text.chars()
            .filter(|character| !character.is_whitespace() && *character != ',')
            .collect::<String>()
    };
    assert!(
        compact(code).contains(&compact(expected)),
        "generated daemon code must contain {expected:?}\n--- generated ---\n{code}"
    );
}

fn assert_code_excludes(code: &str, unexpected: &str) {
    assert!(
        !code.contains(unexpected),
        "generated daemon code must NOT contain {unexpected:?}"
    );
}

fn single_listener_shape() -> NexusDaemonShape {
    NexusDaemonShape::new("test-daemon", WorkingListenerTier::new("signal"))
}

fn multi_listener_shape() -> NexusDaemonShape {
    NexusDaemonShape::new("test-daemon", WorkingListenerTier::new("signal"))
        .with_meta_tier(MetaListenerTier::new(SocketModeBits::new(OWNER_ONLY_SOCKET_MODE)))
}

#[test]
fn daemon_module_emits_the_component_daemon_hook_trait() {
    let schema = FixtureSchema::new("daemon-stream.schema").lower("test:signal");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();

    assert_eq!(generated.path, "src/schema/daemon.rs");
    let code = generated.code.as_str();
    assert_code_contains(code, "pub trait ComponentDaemon");
    assert_code_contains(code, "type Configuration: DaemonConfiguration");
    assert_code_contains(code, "type Engine;");
    assert_code_contains(code, "type Error:");
    assert_code_contains(code, "const PROCESS_NAME: &'static str;");
    assert_code_contains(
        code,
        "fn build_runtime(configuration: &Self::Configuration) -> Result<Self::Engine, Self::Error>;",
    );
    assert_code_contains(
        code,
        "fn handle_working_input(engine: &Self::Engine, input: Input, connection: &triad_runtime::ConnectionContext) -> Result<Output, Self::Error>;",
    );
}

#[test]
fn daemon_module_emits_the_command_and_exit_entry() {
    let schema = FixtureSchema::new("daemon-stream.schema").lower("test:signal");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(code, "pub struct DaemonCommand<Daemon: ComponentDaemon>");
    assert_code_contains(code, "self.command.signal_file_argument()?");
    assert_code_contains(code, "Daemon::load_configuration(file.as_path())");
    assert_code_contains(code, "pub trait DaemonEntry: ComponentDaemon");
    assert_code_contains(code, "fn run_to_exit_code() -> std::process::ExitCode");
    assert_code_contains(code, "ExitReport::new(Self::PROCESS_NAME)");
}

#[test]
fn single_listener_daemon_emits_the_single_listener_spine() {
    let schema = FixtureSchema::new("daemon-stream.schema").lower("test:signal");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(code, "SingleListenerDaemon::new(");
    assert_code_contains(code, "impl<Daemon: ComponentDaemon> DaemonRuntime for GeneratedDaemonRuntime<Daemon>");
    assert_code_contains(code, "fn handle_stream(&mut self, stream: UnixStream)");
    assert_code_contains(code, "self.handle_working_stream(stream)");
    // The spine reads the accepted stream's peer credentials before moving the
    // stream into the transport, then threads them into the working-input hook so
    // the component can mint an origin from the operating-system trust boundary.
    assert_code_contains(
        code,
        "let connection = ConnectionContext::from_stream(&stream).map_err(FrameError::Io)?;",
    );
    assert_code_contains(
        code,
        "Daemon::handle_working_input(&self.engine, input, &connection)?",
    );
    // The single-listener daemon has no meta tier and no listener-tier enum.
    assert_code_excludes(code, "MultiListenerDaemon");
    assert_code_excludes(code, "pub enum ListenerTier");
    assert_code_excludes(code, "handle_meta_stream");
}

#[test]
fn multi_listener_daemon_emits_the_listener_tier_routing() {
    let schema = FixtureSchema::new("daemon-stream.schema").lower("test:signal");
    let generated =
        DaemonModule::new(multi_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(code, "pub enum ListenerTier");
    assert_code_contains(code, "ListenerTier::Working");
    assert_code_contains(code, "ListenerTier::Meta");
    assert_code_contains(code, "MultiListenerDaemon::new(");
    assert_code_contains(code, "SocketMode::new(0o600)");
    assert_code_contains(
        code,
        "fn handle_meta_stream(engine: &Self::Engine, stream: UnixStream) -> Result<(), Self::Error>;",
    );
    assert_code_contains(code, "ListenerTier::Meta => Daemon::handle_meta_stream(&self.engine, stream)");
    assert_code_contains(code, "impl<Daemon: ComponentDaemon> MultiListenerRuntime for GeneratedDaemonRuntime<Daemon>");
}

#[test]
fn declared_stream_emits_the_option_b_subscription_plumbing() {
    let schema = FixtureSchema::new("daemon-stream.schema").lower("test:signal");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    // The component declares only filter + event policy; the registry/publish
    // plumbing is emitted.
    assert_code_contains(code, "pub struct EmittedSubscriptions<Daemon: ComponentDaemon>");
    assert_code_contains(code, "SubscriptionRegistry<Daemon::SubscriptionToken, Daemon::SubscriptionFilter>");
    assert_code_contains(code, "SubscriptionEventPublisher<Input, Output, Daemon::StreamEvent>");
    assert_code_contains(code, "fn subscription_filter(input: &Input) -> Option<Self::SubscriptionFilter>;");
    assert_code_contains(code, "fn published_event(engine: &Self::Engine, output: &Output)");
    assert_code_contains(code, "self.subscriptions.register(token, filter, subscription_writer);");
    assert_code_contains(code, "self.subscriptions.publish(event)?;");

    // The stream event must carry the rkyv Archive + high-level Serialize
    // bounds, or the emitted publisher cannot encode the subscription-event
    // frame (StreamingFrame::encode requires them).
    assert_code_contains(code, "type StreamEvent: Clone");
    assert_code_contains(code, "+ rkyv::Archive");
    assert_code_contains(code, "for<'archive> rkyv::Serialize");
    assert_code_contains(code, "rkyv::api::high::HighSerializer");

    // The emitted publish must reborrow the mutex guard once into the owned
    // state so the disjoint registry/publisher field borrows split cleanly;
    // borrowing through the guard's Deref directly would not compile.
    assert_code_contains(code, "let state = &mut *guard;");
}

#[test]
fn schema_without_a_stream_emits_no_subscription_plumbing() {
    let schema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_excludes(code, "EmittedSubscriptions");
    assert_code_excludes(code, "subscription_filter");
    assert_code_excludes(code, "SubscriptionRegistry");
    // The hook trait and spine are still emitted.
    assert_code_contains(code, "pub trait ComponentDaemon");
    assert_code_contains(code, "fn handle_working_input");
}
