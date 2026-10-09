// `#[async_trait]` expands each trait method with a generated `#[must_use]`, which clippy's
// `double_must_use` then flags because the async return type is already `#[must_use]`. The
// attribute is produced by the macro and cannot be removed at the source, so allow it crate-wide.
#![allow(clippy::double_must_use)]

#[cfg(feature = "geoblock")]
use wc::geoip::block::{middleware::GeoBlockLayer, BlockingPolicy};
#[cfg(any(feature = "analytics", feature = "geoblock"))]
use {
    crate::error::Error,
    aws_config::meta::region::RegionProviderChain,
    aws_sdk_s3::{config::Region, Client as S3Client},
    wc::geoip::MaxMindResolver,
};
use {
    crate::{log::prelude::*, state::TenantStoreArc},
    axum::{
        extract::Request,
        routing::{delete, get, post},
        Router,
    },
    axum_client_ip::SecureClientIpSource,
    config::Config,
    hyper::http::Method,
    middleware::rate_limit::rate_limit_middleware,
    sqlx::{
        postgres::{PgConnectOptions, PgPoolOptions},
        ConnectOptions,
    },
    std::{future::IntoFuture, net::SocketAddr, str::FromStr, sync::Arc, time::Duration},
    tokio::{net::TcpListener, select, sync::broadcast},
    tower::ServiceBuilder,
    tower_http::{
        catch_panic::CatchPanicLayer,
        cors::{AllowOrigin, CorsLayer},
        request_id::MakeRequestUuid,
        trace::TraceLayer,
        ServiceBuilderExt,
    },
    tracing::{info, log::LevelFilter},
};

#[cfg(not(feature = "multitenant"))]
use crate::stores::tenant::DefaultTenantStore;

#[cfg(feature = "analytics")]
pub mod analytics;

#[cfg(not(feature = "analytics"))]
pub mod analytics {
    pub mod message_info {
        #[derive(Debug, Clone, serde::Serialize)]
        pub struct MessageInfo;
    }
}

pub mod blob;
pub mod config;
pub mod error;
pub mod handlers;
pub mod jwt_validation;
pub mod log;
pub mod macros;
pub mod metrics;
pub mod middleware;
pub mod networking;
pub mod providers;
pub mod relay;
pub mod state;
pub mod stores;

const PG_CONNECTION_POOL_SIZE: u32 = 100;

pub async fn bootstap(mut shutdown: broadcast::Receiver<()>, config: Config) -> error::Result<()> {
    // Check config is valid and then throw the error if its not
    config.is_valid()?;

    let pg_options = PgConnectOptions::from_str(&config.database_url)?
        .log_statements(LevelFilter::Trace)
        .log_slow_statements(LevelFilter::Info, Duration::from_millis(250))
        .clone();

    let store = PgPoolOptions::new()
        .max_connections(PG_CONNECTION_POOL_SIZE)
        .connect_with(pg_options)
        .await?;

    // Run database migrations. `./migrations` is the path to migrations, relative
    // to the root dir (the directory containing `Cargo.toml`).
    sqlx::migrate!("./migrations").run(&store).await?;

    #[cfg(not(feature = "multitenant"))]
    let tenant_store: TenantStoreArc = Arc::new(DefaultTenantStore::new(Arc::new(config.clone()))?);

    #[cfg(feature = "multitenant")]
    let tenant_store: TenantStoreArc = {
        let tenant_pg_options = PgConnectOptions::from_str(&config.tenant_database_url)?
            .log_statements(LevelFilter::Trace)
            .log_slow_statements(LevelFilter::Info, Duration::from_millis(250))
            .clone();

        let tenant_database = PgPoolOptions::new()
            .max_connections(PG_CONNECTION_POOL_SIZE)
            .connect_with(tenant_pg_options)
            .await?;

        // Run database migrations. `./tenant_migrations` is the path to migrations,
        // relative to the root dir (the directory containing `Cargo.toml`).
        sqlx::migrate!("./tenant_migrations")
            .run(&tenant_database)
            .await?;

        Arc::new(tenant_database)
    };

    let mut state = state::new_state(
        config,
        Arc::new(store.clone()),
        Arc::new(store.clone()),
        tenant_store,
    )?;

    #[cfg(any(feature = "analytics", feature = "geoblock"))]
    {
        let s3_client = get_s3_client(&state.config).await;
        let geoip_resolver = get_geoip_resolver(&state.config, &s3_client).await;

        #[cfg(feature = "analytics")]
        {
            if let Some(ip) = state.public_ip {
                let analytics =
                    analytics::initialize(&state.config, s3_client, ip, geoip_resolver.clone())
                        .await;
                state.analytics = Some(analytics);
            }
        }

        #[cfg(feature = "geoblock")]
        {
            state.geoblock = geoip_resolver.map(|resolver| {
                GeoBlockLayer::new(
                    resolver.clone(),
                    state.config.blocked_countries.clone(),
                    BlockingPolicy::AllowAll,
                )
            });
        }
    }

    #[cfg(feature = "multitenant")]
    let supported_providers_string = "multi-tenant".to_string();

    #[cfg(not(feature = "multitenant"))]
    let supported_providers_string = state
        .config
        .single_tenant_supported_providers()
        .into_iter()
        .map(Into::into)
        .collect::<Vec<&str>>()
        .join(", ");

    if state.config.telemetry_prometheus_port.is_some() {
        state.set_metrics(metrics::Metrics::new());
    }

    let port = state.config.port;
    let private_port = state.config.telemetry_prometheus_port.unwrap_or(3001);
    let build_version = state.build_info.crate_info.version.clone();
    let build_commit = match state.build_info.version_control.clone() {
        Some(v) => v.git().unwrap().commit_short_id.clone(),
        None => "unknown-commit".to_string(),
    };
    let build_rustc_version = state.build_info.compiler.version.clone();
    let show_header = !state.config.disable_header;
    // TODO use value again
    let _allowed_origins = state.config.cors_allowed_origins.clone();

    let state_arc = Arc::new(state);

    let global_middleware = ServiceBuilder::new()
        .set_x_request_id(MakeRequestUuid)
        .layer(
            TraceLayer::new_for_http()
            .make_span_with(|request: &Request| {
                let request_id = match request.headers().get("x-request-id") {
                    Some(value) => value.to_str().unwrap_or_default().to_string(),
                    None => {
                        // If this warning is triggered, it means that the `x-request-id` was not
                        // propagated to headers properly. This is a bug in the middleware chain.
                        warn!("Missing x-request-id header in a middleware");
                        String::new()
                    }
                };
                tracing::info_span!("http-request", "method" = ?request.method(), "request_id" = ?request_id, "uri" = ?request.uri())
            })
        )
        .layer(CatchPanicLayer::new())
        .layer(
            CorsLayer::new()
                .allow_methods([Method::POST, Method::DELETE])
                .allow_origin(AllowOrigin::any())
                .allow_headers([
                    hyper::http::header::CONTENT_TYPE,
                    hyper::http::header::AUTHORIZATION,
                ]),
        )
        .layer(SecureClientIpSource::RightmostXForwardedFor.into_extension())
        .propagate_x_request_id();

    #[cfg(feature = "multitenant")]
    let app = {
        let tenancy_routes = Router::new()
            .route("/", post(handlers::create_tenant::handler))
            .route(
                "/:id",
                get(handlers::get_tenant::handler).delete(handlers::delete_tenant::handler),
            )
            .route("/:id/fcm", post(handlers::update_fcm::handler))
            .route("/:id/fcm", delete(handlers::delete_fcm::handler))
            .route("/:id/fcm_v1", post(handlers::update_fcm_v1::handler))
            .route("/:id/fcm_v1", delete(handlers::delete_fcm_v1::handler))
            .route("/:id/apns", post(handlers::update_apns::handler))
            .route("/:id/apns", delete(handlers::delete_apns::handler))
            .layer(
                global_middleware.clone().layer(
                    CorsLayer::new()
                        .allow_methods([Method::GET, Method::POST, Method::DELETE])
                        /*
                            TODO: switch back to using the real configuration
                            allowed_origins
                                .iter()
                                .map(|v| v.parse::<HeaderValue>().unwrap())
                                .collect::<Vec<HeaderValue>>(), */
                        .allow_origin(AllowOrigin::any())
                        .allow_headers([hyper::http::header::CONTENT_TYPE, hyper::http::header::AUTHORIZATION]),
                ),
            )
            .layer(axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware));

        Router::new()
            .route("/health", get(handlers::health::handler))
            .route("/rate_limit_test", get(handlers::rate_limit_test::handler).layer(
                axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware),
            ))
            .nest("/tenants", tenancy_routes.layer(
                axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware),
            ))
            .route(
                "/:tenant_id/clients",
                post(handlers::register_client::handler).layer(
                    axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware),
                ),
            )
            .route(
                "/:tenant_id/clients/:id",
                delete(handlers::delete_client::handler).layer(
                    axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware),
                ),
            )
            // Rate limiting middleware is not applying to push_handler because it is used by the relay
            .route(
                "/:tenant_id/clients/:id",
                post(handlers::push_message::handler),
            )
            .layer(global_middleware)
    };

    #[cfg(not(feature = "multitenant"))]
    let app = Router::new()
        .route("/health", get(handlers::health::handler))
        .route("/rate_limit_test", get(handlers::rate_limit_test::handler).layer(
            axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware),
        ))
        .route(
            "/clients",
            post(handlers::single_tenant_wrappers::register_handler).layer(
                axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware),
            ),
        )
        .route(
            "/clients/:id",
            delete(handlers::single_tenant_wrappers::delete_handler).layer(
                axum::middleware::from_fn_with_state(state_arc.clone(), rate_limit_middleware),
            ),
        )
        // Rate limiting middleware is not applying to push_handler because it is used by the relay
        .route(
            "/clients/:id",
            post(handlers::single_tenant_wrappers::push_handler),
        )
        .layer(global_middleware);

    // If geoblock is enabled, add the geoblock middleware to the app
    let app = if let Some(geoblock) = state_arc.geoblock.clone() {
        app.layer(geoblock)
    } else {
        app
    };

    let app = app.with_state(state_arc.clone());
    let private_app = Router::new()
        .route("/metrics", get(handlers::metrics::handler))
        .with_state(state_arc);

    if show_header {
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
            "0.0.0.0",
            port.clone(),
            supported_providers_string
        );

        println!("{header}");
    } else {
        debug!("Online and listening at http://0.0.0.0:{}", port.clone())
    }

    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
    let private_listener =
        TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], private_port))).await?;

    select! {
        _ = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).into_future() => info!("Server terminating"),
        _ = axum::serve(private_listener, private_app.into_make_service()).into_future() => info!("Internal Server terminating"),
        _ = shutdown.recv() => info!("Shutdown signal received, killing servers"),
    }

    Ok(())
}

#[cfg(any(feature = "analytics", feature = "geoblock"))]
async fn get_geoip_resolver(config: &Config, s3_client: &S3Client) -> Option<Arc<MaxMindResolver>> {
    match (&config.geoip_db_bucket, &config.geoip_db_key) {
        (Some(bucket), Some(key)) => {
            info!(%bucket, %key, "initializing geoip database from aws s3");

            MaxMindResolver::from_aws_s3(s3_client, bucket, key)
                .await
                .map_err(|err| {
                    info!(?err, "failed to load geoip resolver");
                    Error::GeoIpS3Failed
                })
                .ok()
                .map(Arc::new)
        }
        _ => {
            info!("analytics geoip lookup is disabled");

            None
        }
    }
}

#[cfg(any(feature = "analytics", feature = "geoblock"))]
async fn get_s3_client(config: &Config) -> S3Client {
    let region_provider = RegionProviderChain::first_try(Region::new("eu-central-1"));
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let aws_config = match &config.s3_endpoint {
        Some(s3_endpoint) => {
            info!(%s3_endpoint, "initializing analytics with custom s3 endpoint");

            aws_sdk_s3::config::Builder::from(&shared_config)
                .endpoint_url(s3_endpoint)
                .build()
        }
        _ => aws_sdk_s3::config::Builder::from(&shared_config).build(),
    };

    S3Client::from_conf(aws_config)
}
