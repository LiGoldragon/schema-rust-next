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
    NexusDaemonShape::new("test-daemon", WorkingListenerTier::new("signal")).with_meta_tier(
        MetaListenerTier::new(SocketModeBits::new(OWNER_ONLY_SOCKET_MODE)),
    )
}

#[test]
fn daemon_module_emits_the_component_daemon_hook_trait() {
    let schema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();

    assert_eq!(generated.path, "src/schema/daemon.rs");
    let code = generated.code.as_str();
    assert_code_contains(code, "#[rustfmt::skip]");
    assert_code_contains(code, "pub trait ComponentDaemon");
    assert_code_contains(code, "type Configuration: DaemonConfiguration");
    assert_code_contains(code, "type Engine: Send + Sync + 'static;");
    assert_code_contains(code, "type Error:");
    assert_code_contains(code, "const PROCESS_NAME: &'static str;");
    assert_code_contains(
        code,
        "fn build_runtime(configuration: &Self::Configuration) -> Result<Self::Engine, Self::Error>;",
    );
    assert_code_contains(
        code,
        "fn handle_working_input<'connection>(engine: &'connection Self::Engine, input: Input, connection: &'connection triad_runtime::ConnectionContext) -> impl std::future::Future<Output = Result<Output, Self::Error>> + Send + 'connection;",
    );
}

#[test]
fn daemon_module_emits_the_command_and_exit_entry() {
    let schema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(code, "pub struct DaemonCommand<Daemon: ComponentDaemon>");
    assert_code_contains(code, "self.command.signal_file_argument()?");
    assert_code_contains(code, "Daemon::load_configuration(file.as_path())");
    assert_code_contains(code, "tokio::runtime::Runtime::new()");
    assert_code_contains(code, "pub trait DaemonEntry: ComponentDaemon");
    assert_code_contains(code, "fn run_to_exit_code() -> std::process::ExitCode");
    assert_code_contains(code, "ExitReport::new(Self::PROCESS_NAME)");
}

#[test]
fn single_listener_daemon_emits_the_actor_native_single_listener_spine() {
    let schema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(code, "ActorSingleListenerDaemon::new(");
    assert_code_contains(
        code,
        "impl<Daemon: ComponentDaemon> ActorConnectionRuntime for GeneratedDaemonRuntime<Daemon>",
    );
    assert_code_contains(code, "async fn handle_connection(");
    assert_code_contains(code, "self.handle_working_connection(connection).await");
    assert_code_contains(
        code,
        "Daemon::handle_working_input(&self.engine, input, transport.context()).await?",
    );
    assert_code_contains(code, "read_body_async(self.connection.stream_mut())");
    assert_code_contains(code, "write_body_async(");
    // The single-listener actor daemon has no sync listener, no meta tier, and
    // no listener-tier enum.
    assert_code_excludes(
        code,
        "impl<Daemon: ComponentDaemon> DaemonRuntime for GeneratedDaemonRuntime<Daemon>",
    );
    assert_code_excludes(code, "UnixStream");
    assert_code_excludes(code, "MultiListenerRuntime");
    assert_code_excludes(code, "pub enum ListenerTier");
    assert_code_excludes(code, "handle_meta_connection");
}

#[test]
fn meta_listener_tier_emits_the_actor_native_multi_listener_spine() {
    let schema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated =
        DaemonModule::new(multi_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(code, "pub enum ListenerTier");
    assert_code_contains(code, "Working");
    assert_code_contains(code, "Meta");
    assert_code_contains(code, "ActorMultiListenerDaemon::new(");
    assert_code_contains(code, "ActorListenerSocket::new(");
    assert_code_contains(code, "SocketMode::new(0o600)");
    assert_code_contains(
        code,
        "impl<Daemon: ComponentDaemon> ActorMultiConnectionRuntime for GeneratedDaemonRuntime<Daemon>",
    );
    assert_code_contains(code, "type Listener = ListenerTier;");
    assert_code_contains(
        code,
        "ListenerTier::Working => self.handle_working_connection(connection).await",
    );
    assert_code_contains(
        code,
        "ListenerTier::Meta => { Daemon::handle_meta_connection(&self.engine, connection).await }",
    );
    assert_code_contains(code, "fn handle_meta_connection(");
    assert_code_contains(code, "MissingMetaSocket");
    assert_code_contains(code, "From<ActorMultiListenerDaemonError<Daemon::Error>>");
    assert_code_excludes(code, "MultiListenerRuntime");
    assert_code_excludes(code, "ActorSingleListenerDaemon::new(");
    assert_code_excludes(code, "ActorConnectionRuntime for GeneratedDaemonRuntime");
    assert_code_excludes(code, "handle_meta_stream");
    assert_code_excludes(code, "UnixStream");
}

#[test]
fn declared_stream_emits_actor_native_subscription_support() {
    let schema = FixtureSchema::new("daemon-stream.schema").lower("test:signal");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(code, "type SubscriptionToken:");
    assert_code_contains(code, "type SubscriptionFilter:");
    assert_code_contains(code, "type StreamEvent:");
    assert_code_contains(code, "fn subscription_filter(input: &Input)");
    assert_code_contains(code, "fn subscription_token(output: &Output)");
    assert_code_contains(
        code,
        "fn published_event<'event>(engine: &'event Self::Engine, output: &'event Output)",
    );
    assert_code_contains(code, "fn event_matches_filter(");
    assert_code_contains(code, "fn subscription_event_short_header() -> u64");
    assert_code_contains(
        code,
        "pub struct EmittedSubscriptions<Daemon: ComponentDaemon>",
    );
    assert_code_contains(code, "tokio::sync::Mutex");
    assert_code_contains(code, "SubscriptionEventPublisher::acceptor(");
    assert_code_contains(code, "let (stream, context) = connection.into_parts();");
    assert_code_contains(code, "let (reader, writer) = stream.into_split();");
    assert_code_contains(code, "transport.into_writer()");
    assert_code_contains(
        code,
        "Daemon::published_event(&self.engine, &output).await?",
    );
    assert_code_contains(code, "self.subscriptions.publish(event).await?");
    assert_code_contains(code, "frame.encode()?");
    assert_code_contains(code, "write_body_async(writer, &FrameBody::new(bytes))");
    assert_code_excludes(code, "compile_error!");
    assert_code_excludes(code, "std::sync::Mutex");
    assert_code_excludes(code, "try_clone_stream");
    assert_code_excludes(code, "UnixStream");
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
