use {
    crate::{
        config::Config,
        metrics::Metrics,
        middleware::rate_limit,
        networking,
        providers::Provider,
        relay::RelayClient,
        stores::{client::ClientStore, notification::NotificationStore, tenant::TenantStore},
    },
    build_info::BuildInfo,
    moka::future::Cache,
    std::{net::IpAddr, sync::Arc},
    tokio::time::Duration,
    wc::geoip::{block::middleware::GeoBlockLayer, MaxMindResolver},
};

#[cfg(feature = "analytics")]
use crate::analytics::PushAnalytics;
#[cfg(feature = "multitenant")]
use crate::jwt_validation::JwtValidationClient;

pub type ClientStoreArc = Arc<dyn ClientStore + Send + Sync + 'static>;
pub type NotificationStoreArc = Arc<dyn NotificationStore + Send + Sync + 'static>;
pub type TenantStoreArc = Arc<dyn TenantStore + Send + Sync + 'static>;

pub trait State {
    fn config(&self) -> Config;
    fn build_info(&self) -> BuildInfo;
    fn client_store(&self) -> ClientStoreArc;
    fn notification_store(&self) -> NotificationStoreArc;
    fn tenant_store(&self) -> TenantStoreArc;
    fn relay_client(&self) -> RelayClient;
    fn is_multitenant(&self) -> bool;
    fn validate_signatures(&self) -> bool;
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub build_info: BuildInfo,
    pub metrics: Option<Metrics>,
    #[cfg(feature = "analytics")]
    pub analytics: Option<PushAnalytics>,
    pub client_store: ClientStoreArc,
    pub notification_store: NotificationStoreArc,
    pub tenant_store: TenantStoreArc,
    pub relay_client: RelayClient,
    #[cfg(feature = "multitenant")]
    pub jwt_validation_client: JwtValidationClient,
    pub public_ip: Option<IpAddr>,
    is_multitenant: bool,
    pub geoblock: Option<GeoBlockLayer<Arc<MaxMindResolver>>>,
    /// Service instance identifier
    pub instance_id: uuid::Uuid,
    /// Service instance uptime measurement
    pub uptime: std::time::Instant,
    pub http_client: reqwest::Client,
    pub provider_cache: Cache<String, Provider>,
    pub rate_limit: rate_limit::RateLimiter,
}

build_info::build_info!(fn build_info);

pub fn new_state(
    config: Config,
    client_store: ClientStoreArc,
    notification_store: NotificationStoreArc,
    tenant_store: TenantStoreArc,
) -> crate::error::Result<AppState> {
    let build_info: &BuildInfo = build_info();

    #[cfg(feature = "multitenant")]
    let is_multitenant = true;

    #[cfg(not(feature = "multitenant"))]
    let is_multitenant = false;

    #[cfg(feature = "multitenant")]
    let jwt_secret = config.jwt_secret.clone();

    let public_ip = networking::find_public_ip_addr().ok();

    Ok(AppState {
        config: config.clone(),
        build_info: build_info.clone(),
        metrics: None,
        #[cfg(feature = "analytics")]
        analytics: None,
        client_store,
        notification_store,
        tenant_store,
        relay_client: RelayClient::new(config.relay_public_key)?,
        #[cfg(feature = "multitenant")]
        jwt_validation_client: JwtValidationClient::new(jwt_secret),
        public_ip,
        is_multitenant,
        geoblock: None,
        instance_id: uuid::Uuid::new_v4(),
        uptime: std::time::Instant::now(),
        http_client: reqwest::Client::new(),
        provider_cache: Cache::new(100),
        rate_limit: rate_limit::RateLimiter::new(100, Duration::from_secs(60)),
    })
}

impl AppState {
    pub fn set_metrics(&mut self, metrics: Metrics) {
        self.metrics = Some(metrics);
    }
}

impl State for Arc<AppState> {
    fn config(&self) -> Config {
        self.config.clone()
    }

    fn build_info(&self) -> BuildInfo {
        self.build_info.clone()
    }

    fn client_store(&self) -> ClientStoreArc {
        self.client_store.clone()
    }

    fn notification_store(&self) -> NotificationStoreArc {
        self.notification_store.clone()
    }

    fn tenant_store(&self) -> TenantStoreArc {
        self.tenant_store.clone()
    }

    fn relay_client(&self) -> RelayClient {
        self.relay_client.clone()
    }

    fn is_multitenant(&self) -> bool {
        self.is_multitenant
    }

    fn validate_signatures(&self) -> bool {
        self.config.validate_signatures
    }
}
