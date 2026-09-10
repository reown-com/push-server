use {
    crate::{
        error,
        error::{
            Error,
            Error::{InvalidConfiguration, NoApnsConfigured},
        },
        stores::tenant::ApnsType,
    },
    serde::Deserialize,
};

#[cfg(not(feature = "multitenant"))]
use crate::providers::ProviderKind;

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    pub public_url: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_level_otel")]
    pub log_level_otel: String,
    #[serde(default = "default_disable_header")]
    pub disable_header: bool,
    pub relay_public_key: String,
    #[serde(default = "default_validate_signatures")]
    pub validate_signatures: bool,
    pub database_url: String,
    #[serde(default = "default_is_test", skip)]
    /// This is an internal flag to disable logging, cannot be defined by user
    pub is_test: bool,

    // CORS
    #[serde(default = "default_cors_allowed_origins")]
    pub cors_allowed_origins: Vec<String>,

    // TELEMETRY
    pub otel_exporter_otlp_endpoint: Option<String>,
    pub telemetry_prometheus_port: Option<u16>,

    // APNS
    #[cfg(not(feature = "multitenant"))]
    pub apns_type: Option<ApnsType>,
    #[cfg(not(feature = "multitenant"))]
    pub apns_topic: Option<String>,

    #[cfg(not(feature = "multitenant"))]
    pub apns_certificate: Option<String>,
    #[cfg(not(feature = "multitenant"))]
    pub apns_certificate_password: Option<String>,

    #[cfg(not(feature = "multitenant"))]
    pub apns_pkcs8_pem: Option<String>,
    #[cfg(not(feature = "multitenant"))]
    pub apns_key_id: Option<String>,
    #[cfg(not(feature = "multitenant"))]
    pub apns_team_id: Option<String>,

    // FCM
    // No legacy `fcm_api_key`: the legacy FCM HTTP API is decommissioned, so a
    // single-tenant deployment configures FCM_V1_CREDENTIALS instead.
    #[cfg(not(feature = "multitenant"))]
    pub fcm_v1_credentials: Option<String>,

    // Multi-tenancy
    pub tenant_database_url: String,
    #[cfg(feature = "multitenant")]
    pub jwt_secret: String,

    // Analytics
    #[cfg(any(feature = "analytics", feature = "geoblock"))]
    pub s3_endpoint: Option<String>,

    #[cfg(any(feature = "analytics", feature = "geoblock"))]
    pub geoip_db_bucket: Option<String>,
    #[cfg(any(feature = "analytics", feature = "geoblock"))]
    pub geoip_db_key: Option<String>,

    #[cfg(feature = "analytics")]
    pub analytics_export_bucket: String,

    #[cfg(feature = "geoblock")]
    pub blocked_countries: Vec<String>,
}

impl Config {
    /// Run validations against config and throw error
    pub fn is_valid(&self) -> error::Result<()> {
        #[cfg(feature = "multitenant")]
        {
            if self.tenant_database_url == self.database_url {
                return Err(InvalidConfiguration(
                    "`TENANT_DATABASE_URL` is equal to `DATABASE_URL`, this is not allowed"
                        .to_string(),
                ));
            }
        }

        // Check that APNS config is valid when it has been configured
        match self.get_apns_type() {
            Ok(_) => Ok(()),
            Err(NoApnsConfigured) => Ok(()),
            Err(e) => Err(e),
        }?;

        // Empty Relay public key is not allowed
        if self.relay_public_key.is_empty() {
            return Err(InvalidConfiguration(
                "`RELAY_PUBLIC_KEY` cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    #[cfg(not(feature = "multitenant"))]
    pub fn single_tenant_supported_providers(&self) -> Vec<ProviderKind> {
        let mut supported = vec![];

        if self.get_apns_type().is_ok() {
            supported.push(ProviderKind::Apns);
            supported.push(ProviderKind::ApnsSandbox);
        }

        if self.fcm_v1_credentials.is_some() {
            supported.push(ProviderKind::Fcm);
        }

        // Only available in debug/testing
        #[cfg(any(debug_assertions, test))]
        supported.push(ProviderKind::Noop);

        supported
    }

    pub fn get_apns_type(&self) -> Result<ApnsType, Error> {
        #[cfg(not(feature = "multitenant"))]
        if let Some(apns_type) = &self.apns_type {
            // Check if APNS config is correct
            let _ = match apns_type {
                ApnsType::Certificate => match (
                    &self.apns_topic,
                    &self.apns_certificate,
                    &self.apns_certificate_password,
                ) {
                    (Some(_), Some(_), Some(_)) => Ok(ApnsType::Certificate),
                    _ => Err(InvalidConfiguration(
                        "APNS_TYPE of Certificate requires specific variables, please check the \
                         documentation."
                            .to_string(),
                    )),
                },
                ApnsType::Token => match (
                    &self.apns_topic,
                    &self.apns_pkcs8_pem,
                    &self.apns_key_id,
                    &self.apns_team_id,
                ) {
                    (Some(_), Some(_), Some(_), Some(_)) => Ok(ApnsType::Token),
                    _ => Err(InvalidConfiguration(
                        "APNS_TYPE of Certificate requires specific variables, please check the \
                         documentation."
                            .to_string(),
                    )),
                },
            }?;
        }

        Err(NoApnsConfigured)
    }
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info,echo-server=info".to_string()
}

fn default_log_level_otel() -> String {
    "info,echo-server=trace".to_string()
}

fn default_disable_header() -> bool {
    false
}

fn default_validate_signatures() -> bool {
    true
}

fn default_is_test() -> bool {
    false
}

fn default_cors_allowed_origins() -> Vec<String> {
    vec!["*".to_string()]
}

pub fn get_config() -> error::Result<Config> {
    let config = envy::from_env::<Config>()?;
    Ok(config)
}
