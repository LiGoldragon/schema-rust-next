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
        "fn handle_working_input(engine: &Self::Engine, input: Input, connection: &triad_runtime::ConnectionContext) -> Result<Output, Self::Error>;",
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
        "Daemon::handle_working_input(&self.engine, input, transport.context())?",
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
    assert_code_excludes(code, "MultiListenerDaemon");
    assert_code_excludes(code, "pub enum ListenerTier");
    assert_code_excludes(code, "handle_meta_stream");
}

#[test]
fn meta_listener_tier_is_rejected_until_actor_native_meta_support_lands() {
    let schema = FixtureSchema::new("spirit-min.schema").lower("spirit:lib");
    let generated =
        DaemonModule::new(multi_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(
        code,
        "compile_error!(\"actor-native daemon emission does not yet support the meta listener tier\");",
    );
    assert_code_excludes(code, "MultiListenerDaemon");
    assert_code_excludes(code, "handle_meta_stream");
    assert_code_excludes(code, "UnixStream");
}

#[test]
fn declared_stream_is_rejected_until_actor_native_subscription_support_lands() {
    let schema = FixtureSchema::new("daemon-stream.schema").lower("test:signal");
    let generated =
        DaemonModule::new(single_listener_shape(), &schema, "schema-rust-next").to_generated_file();
    let code = generated.code.as_str();

    assert_code_contains(
        code,
        "compile_error!(\"actor-native daemon emission does not yet support declared streams\");",
    );
    assert_code_excludes(code, "EmittedSubscriptions");
    assert_code_excludes(code, "std::sync::Mutex");
    assert_code_excludes(code, "SubscriptionRegistry");
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
