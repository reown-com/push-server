#[cfg(feature = "functional_tests")]
use echo_server::state::{ClientStoreArc, NotificationStoreArc, TenantStoreArc};
use {
    self::server::EchoServer,
    async_trait::async_trait,
    echo_server::config::Config,
    sqlx::{Pool, Postgres},
    std::{env, sync::Arc},
    test_context::{AsyncTestContext, TestContext},
};

mod server;
mod stores;

pub struct ConfigContext {
    pub config: Config,
}

pub struct EchoServerContext {
    pub server: EchoServer,
    #[cfg(all(
        feature = "multitenant",
        feature = "apns_tests",
        feature = "fcm_tests",
        feature = "fcmv1_tests"
    ))]
    pub config: Config,
}

pub struct StoreContext {
    pub pool: Arc<Pool<Postgres>>,
    pub tenant_pool: Arc<Pool<Postgres>>,

    #[cfg(feature = "functional_tests")]
    pub clients: ClientStoreArc,
    #[cfg(feature = "functional_tests")]
    pub notifications: NotificationStoreArc,
    #[cfg(feature = "functional_tests")]
    pub tenants: TenantStoreArc,
}

impl TestContext for ConfigContext {
    fn setup() -> Self {
        let public_port = self::server::get_random_port();
        let config = Config {
            port: public_port,
            public_url: format!("http://127.0.0.1:{public_port}"),
            log_level: "info,echo-server=info".into(),
            log_level_otel: "info,echo-server=trace".into(),
            disable_header: true,
            validate_signatures: false,
            relay_public_key: env::var("RELAY_PUBLIC_KEY").unwrap_or(
                // Default relay public key if env not set
                // TODO I don't think this is used in the tests, so this should be refactored/removed
                "ff469faa970df23c23a6542765ce8dba2a907538522833b2327a153e365d138e".to_string(),
            ),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL environment variable is not set"),
            tenant_database_url: env::var("TENANT_DATABASE_URL")
                .expect("TENANT_DATABASE_URL environment variable is not set"),
            #[cfg(feature = "multitenant")]
            jwt_secret: "n/a".to_string(),
            otel_exporter_otlp_endpoint: None,
            telemetry_prometheus_port: Some(self::server::get_random_port()),
            #[cfg(not(feature = "multitenant"))]
            apns_type: None,
            #[cfg(not(feature = "multitenant"))]
            apns_certificate: None,
            #[cfg(not(feature = "multitenant"))]
            apns_certificate_password: None,
            #[cfg(not(feature = "multitenant"))]
            apns_pkcs8_pem: None,
            #[cfg(not(feature = "multitenant"))]
            apns_team_id: None,
            #[cfg(not(feature = "multitenant"))]
            apns_key_id: None,
            #[cfg(not(feature = "multitenant"))]
            apns_topic: None,
            #[cfg(not(feature = "multitenant"))]
            fcm_v1_credentials: None,
            #[cfg(any(feature = "analytics", feature = "geoblock"))]
            s3_endpoint: None,
            #[cfg(any(feature = "analytics", feature = "geoblock"))]
            geoip_db_bucket: None,
            #[cfg(any(feature = "analytics", feature = "geoblock"))]
            geoip_db_key: None,
            #[cfg(feature = "analytics")]
            analytics_export_bucket: "example-bucket".to_string(),
            is_test: true,
            cors_allowed_origins: vec!["*".to_string()],
            #[cfg(feature = "geoblock")]
            blocked_countries: vec![],
        };
        Self { config }
    }
}

#[async_trait]
impl AsyncTestContext for EchoServerContext {
    async fn setup() -> Self {
        Self {
            server: EchoServer::start(ConfigContext::setup().config).await,
            #[cfg(all(
                feature = "multitenant",
                feature = "apns_tests",
                feature = "fcm_tests",
                feature = "fcmv1_tests"
            ))]
            config: ConfigContext::setup().config,
        }
    }

    async fn teardown(mut self) {
        self.server.shutdown().await;
    }
}

#[async_trait]
impl AsyncTestContext for StoreContext {
    async fn setup() -> Self {
        let config = ConfigContext::setup().config;
        let (db, tenant_db) =
            stores::open_pg_connections(&config.database_url, &config.tenant_database_url).await;

        let db_arc = Arc::new(db);
        let tenant_db_arc = Arc::new(tenant_db);

        Self {
            pool: db_arc.clone(),
            tenant_pool: tenant_db_arc.clone(),
            #[cfg(feature = "functional_tests")]
            clients: db_arc.clone(),
            #[cfg(feature = "functional_tests")]
            notifications: db_arc.clone(),
            #[cfg(feature = "functional_tests")]
            tenants: tenant_db_arc.clone(),
        }
    }

    async fn teardown(self) {
        self.pool.close().await;
        self.tenant_pool.close().await;
    }
}
