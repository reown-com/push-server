mod env;
mod env_test;
mod error;
mod handlers;
mod providers;
mod state;
mod store;
mod store_test;

use crate::env::Config;
use build_info::BuildInfo;
use dotenv::dotenv;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::state::{AppState, Metrics};

use opentelemetry::sdk::metrics::selectors;
use opentelemetry::sdk::{
    trace::{self, IdGenerator, Sampler},
    Resource,
};
use opentelemetry::util::tokio_interval_stream;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use std::time::Duration;

use tracing_subscriber::fmt::format::FmtSpan;

use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::store::Client;

#[tokio::main]
async fn main() -> error::Result<()> {
    dotenv().ok();
    let config =
        env::get_config().expect("Failed to load config, please ensure all env vars are defined.");

    let supported_providers = config.supported_providers();
    if supported_providers.is_empty() {
        panic!("You must enable at least one provider.");
    }

    let store: HashMap<String, Client> = HashMap::new();
    let mut state = state::new_state(config, store)?;

    if state.config.telemetry_enabled.unwrap_or(false) {
        let grpc_url = state
            .config
            .telemetry_grpc_url
            .clone()
            .unwrap_or_else(|| "http://localhost:4317".to_string());

        let tracing_exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(grpc_url.clone())
            .with_timeout(Duration::from_secs(5))
            .with_protocol(Protocol::Grpc);

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(tracing_exporter)
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_id_generator(IdGenerator::default())
                    .with_max_events_per_span(64)
                    .with_max_attributes_per_span(16)
                    .with_max_events_per_span(16)
                    .with_resource(Resource::new(vec![
                        KeyValue::new("service.name", "echo-server"),
                        KeyValue::new(
                            "service.version",
                            state.build_info.crate_info.version.clone().to_string(),
                        ),
                    ])),
            )
            .install_batch(opentelemetry::runtime::Tokio)?;

        let metrics_exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(grpc_url)
            .with_timeout(Duration::from_secs(5))
            .with_protocol(Protocol::Grpc);

        let meter_provider = opentelemetry_otlp::new_pipeline()
            .metrics(tokio::spawn, tokio_interval_stream)
            .with_exporter(metrics_exporter)
            .with_period(Duration::from_secs(3))
            .with_timeout(Duration::from_secs(10))
            .with_aggregator_selector(selectors::simple::Selector::Exact)
            .build()?;

        opentelemetry::global::set_meter_provider(meter_provider.provider());

        let meter = opentelemetry::global::meter("echo-server");
        let hooks_counter = meter
            .i64_up_down_counter("registered_webhooks")
            .with_description("The number of currently registered webhooks")
            .init();

        let notification_counter = meter
            .u64_counter("received_notifications")
            .with_description("The number of notification received")
            .init();

        state.set_telemetry(
            tracer,
            Metrics {
                registered_webhooks: hooks_counter,
                received_notifications: notification_counter,
            },
        )
    } else {
        // Only log to console if telemetry disabled
        tracing_subscriber::fmt()
            .with_max_level(state.config.log_level())
            .with_span_events(FmtSpan::CLOSE)
            .init();
    }

    let port = state.config.port;
    let build_version = state.build_info.crate_info.version.clone();
    let build_commit = state
        .build_info
        .version_control
        .clone()
        .unwrap()
        .git()
        .unwrap()
        .commit_short_id
        .clone();
    let build_rustc_version = state.build_info.compiler.version.clone();

    let state_arc = Arc::new(state);

    let app = Router::with_state(state_arc)
        .route("/health", get(handlers::health::handler))
        .route("/clients", post(handlers::register_client::handler))
        .route(
            "/clients/:id",
            delete(handlers::delete_client::handler).post(handlers::push_message::handler),
        );

    let header = format!(
        "
 ______       _               _____
|  ____|     | |             / ____|
| |__    ___ | |__    ___   | (___    ___  _ __ __   __ ___  _ __
|  __|  / __|| '_ \\  / _ \\   \\___ \\  / _ \\| '__|\\ \\ / // _ \\| '__|
| |____| (__ | | | || (_) |  ____) ||  __/| |    \\ V /|  __/| |
|______|\\___||_| |_| \\___/  |_____/  \\___||_|     \\_/  \\___||_|
\nversion: {}, commit: {}, rustc: {},
web-host: {}, web-port: {},
providers: [{}]
",
        build_version,
        build_commit,
        build_rustc_version,
        "127.0.0.1",
        port.clone(),
        supported_providers.join(", ")
    );
    println!("{}", header);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
