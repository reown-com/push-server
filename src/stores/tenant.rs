#[cfg(not(feature = "multitenant"))]
use {crate::config::Config, std::sync::Arc};
use {
    crate::{
        error::{
            self,
            Error::{self, InvalidTenantId, ProviderNotAvailable},
            Result,
        },
        providers::{
            apns::ApnsProvider,
            fcm_v1::FcmV1Provider,
            Provider::{self, Apns, FcmV1},
            ProviderKind,
        },
    },
    async_trait::async_trait,
    base64::Engine as _,
    chrono::{DateTime, Utc},
    moka::future::Cache,
    reqwest::Client,
    serde::{Deserialize, Serialize},
    sqlx::{Executor, PgPool},
    tracing::{debug, instrument},
};

#[cfg(any(debug_assertions, test))]
use crate::providers::{noop::NoopProvider, Provider::Noop};

const APNS_TYPE_CERTIFICATE: &str = "certificate";
const APNS_TYPE_TOKEN: &str = "token";

pub const DEFAULT_TENANT_ID: &str = "0000-0000-0000-0000";

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "apns_type")]
#[sqlx(rename_all = "lowercase")]
pub enum ApnsType {
    Certificate,
    Token,
}

impl ApnsType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Certificate => APNS_TYPE_CERTIFICATE,
            Self::Token => APNS_TYPE_TOKEN,
        }
    }
}

impl From<&ApnsType> for String {
    fn from(val: &ApnsType) -> Self {
        val.as_str().to_string()
    }
}

impl From<ApnsType> for String {
    fn from(val: ApnsType) -> Self {
        val.as_str().to_string()
    }
}

impl From<ApnsType> for &str {
    fn from(val: ApnsType) -> Self {
        val.as_str()
    }
}

impl TryFrom<&str> for ApnsType {
    type Error = error::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value.to_lowercase().as_str() {
            APNS_TYPE_CERTIFICATE => Ok(Self::Certificate),
            APNS_TYPE_TOKEN => Ok(Self::Token),
            _ => Err(error::Error::InvalidApnsType(value.to_owned())),
        }
    }
}

#[derive(sqlx::FromRow, Debug, Eq, PartialEq, Clone)]
pub struct Tenant {
    pub id: String,

    pub fcm_api_key: Option<String>,
    pub fcm_v1_credentials: Option<String>,

    pub apns_type: Option<ApnsType>,
    pub apns_topic: Option<String>,

    // Certificate Based
    pub apns_certificate: Option<String>,
    pub apns_certificate_password: Option<String>,

    // Token Based
    pub apns_pkcs8_pem: Option<String>,
    pub apns_key_id: Option<String>,
    pub apns_team_id: Option<String>,

    // Suspension
    pub suspended: bool,
    pub suspended_reason: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TenantUpdateParams {
    pub id: String,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TenantFcmUpdateParams {
    pub fcm_api_key: String,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TenantFcmV1UpdateParams {
    pub fcm_v1_credentials: String,
}

#[derive(Deserialize, Clone, Debug)]
pub enum TenantApnsUpdateAuth {
    Certificate {
        apns_certificate: String,
        apns_certificate_password: String,
    },
    Token {
        apns_pkcs8_pem: String,
        apns_key_id: String,
        apns_team_id: String,
    },
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TenantApnsUpdateParams {
    pub apns_topic: String,
}

impl Tenant {
    pub fn providers(&self) -> Vec<ProviderKind> {
        let mut supported = vec![];

        if self.get_apns_type().is_some() {
            supported.push(ProviderKind::Apns);
            supported.push(ProviderKind::ApnsSandbox);
        }

        // Deliberately not self.fcm_api_key: a legacy-only tenant cannot receive FCM
        // notifications any more, so advertising the provider would be a lie.
        if self.fcm_v1_credentials.is_some() {
            supported.push(ProviderKind::Fcm);
        }

        // Only available in debug/testing
        #[cfg(any(debug_assertions, test))]
        supported.push(ProviderKind::Noop);

        supported
    }

    pub fn get_apns_type(&self) -> Option<ApnsType> {
        if let Some(apns_type) = &self.apns_type {
            // Check if APNS config is correct
            match apns_type {
                ApnsType::Certificate => match (
                    &self.apns_topic,
                    &self.apns_certificate,
                    &self.apns_certificate_password,
                ) {
                    (Some(_), Some(_), Some(_)) => Some(ApnsType::Certificate),
                    _ => None,
                },
                ApnsType::Token => match (
                    &self.apns_topic,
                    &self.apns_pkcs8_pem,
                    &self.apns_key_id,
                    &self.apns_team_id,
                ) {
                    (Some(_), Some(_), Some(_), Some(_)) => Some(ApnsType::Token),
                    _ => None,
                },
            }
        } else {
            None
        }
    }

    #[instrument(skip_all, fields(tenant_id = %self.id, provider = %provider.as_str()))]
    pub async fn provider(
        &self,
        provider: &ProviderKind,
        http_client: Client,
        provider_cache: &Cache<String, Provider>,
    ) -> Result<Provider> {
        if !self.providers().contains(provider) {
            return Err(ProviderNotAvailable(provider.into()));
        }

        match provider {
            ProviderKind::ApnsSandbox | ProviderKind::Apns => {
                let endpoint = match provider {
                    ProviderKind::ApnsSandbox => a2::Endpoint::Sandbox,
                    _ => a2::Endpoint::Production,
                };
                match self.get_apns_type() {
                    Some(ApnsType::Certificate) => match (
                        &self.apns_certificate,
                        &self.apns_certificate_password,
                        &self.apns_topic,
                    ) {
                        (Some(certificate), Some(password), Some(topic)) => {
                            debug!("apns certificate (p12) provider is matched");
                            let decoded =
                                base64::engine::general_purpose::STANDARD.decode(certificate)?;
                            let apns_client = ApnsProvider::new_cert(
                                &mut &mut std::io::Cursor::new(decoded),
                                password.clone(),
                                endpoint,
                                topic.clone(),
                            )?;

                            Ok(Apns(apns_client))
                        }
                        _ => Err(ProviderNotAvailable(provider.into())),
                    },
                    Some(ApnsType::Token) => match (
                        &self.apns_topic,
                        &self.apns_pkcs8_pem,
                        &self.apns_key_id,
                        &self.apns_team_id,
                    ) {
                        (Some(topic), Some(pkcs8_pem), Some(key_id), Some(team_id)) => {
                            debug!("apns token (p8) provider is matched");
                            let p8_token =
                                base64::engine::general_purpose::STANDARD.decode(pkcs8_pem)?;
                            let apns_client = ApnsProvider::new_token(
                                &mut std::io::Cursor::new(p8_token),
                                key_id.clone(),
                                team_id.clone(),
                                endpoint,
                                topic.clone(),
                            )?;

                            Ok(Apns(apns_client))
                        }
                        _ => Err(ProviderNotAvailable(provider.into())),
                    },
                    None => Err(ProviderNotAvailable(provider.into())),
                }
            }
            ProviderKind::Fcm => match self.fcm_v1_credentials.clone() {
                Some(fcm_v1_credentials) => {
                    debug!("fcm v1 provider is matched");
                    if let Some(provider) = provider_cache.get(&fcm_v1_credentials).await {
                        return Ok(provider);
                    }
                    #[allow(clippy::match_single_binding)]
                    let fcm = FcmV1(
                        FcmV1Provider::new(
                            serde_json::from_str(&fcm_v1_credentials)
                                .map_err(Error::InternalFcmV1InvalidServiceAccountKey)?,
                            http_client,
                        )
                        .await
                        .map_err(|e| match e {
                            // TODO check specific error
                            // ServiceAccountError::Unauthorized => Error::BadFcmV1Credentials,
                            _ => Error::BadFcmV1Credentials,
                        })?,
                    );
                    provider_cache
                        .insert(fcm_v1_credentials.clone(), fcm.clone())
                        .await;
                    Ok(fcm)
                }
                // A tenant holding only a legacy server key has no working FCM path:
                // the legacy HTTP API it would send through is decommissioned. Report
                // that instead of building a provider whose every send answers 404.
                None => match self.fcm_api_key {
                    Some(_) => Err(Error::LegacyFcmApiRetired),
                    None => Err(ProviderNotAvailable(provider.into())),
                },
            },
            #[cfg(any(debug_assertions, test))]
            ProviderKind::Noop => {
                debug!("noop provider is matched");
                Ok(Noop(NoopProvider::new()))
            }
        }
    }
}

#[async_trait]
pub trait TenantStore {
    async fn get_tenant(&self, id: &str) -> Result<Tenant>;
    async fn delete_tenant(&self, id: &str) -> Result<()>;
    async fn create_tenant(&self, params: TenantUpdateParams) -> Result<Tenant>;
    async fn update_tenant_fcm(&self, id: &str, params: TenantFcmUpdateParams) -> Result<Tenant>;
    async fn update_tenant_delete_fcm(&self, id: &str) -> Result<Tenant>;
    async fn update_tenant_fcm_v1(
        &self,
        id: &str,
        params: TenantFcmV1UpdateParams,
    ) -> Result<Tenant>;
    async fn update_tenant_delete_fcm_v1(&self, id: &str) -> Result<Tenant>;
    async fn update_tenant_apns(&self, id: &str, params: TenantApnsUpdateParams) -> Result<Tenant>;
    async fn update_tenant_apns_auth(
        &self,
        id: &str,
        params: TenantApnsUpdateAuth,
    ) -> Result<Tenant>;
    async fn update_tenant_delete_apns(&self, id: &str) -> Result<Tenant>;
    async fn suspend_tenant(&self, id: &str, reason: &str) -> Result<()>;
    async fn unsuspend_tenant(&self, id: &str) -> Result<()>;
}

#[async_trait]
impl TenantStore for PgPool {
    #[instrument(skip(self))]
    async fn get_tenant(&self, id: &str) -> Result<Tenant> {
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(
            "SELECT * FROM public.tenants WHERE id = $1",
        )
        .bind(id)
        .fetch_one(self)
        .await;

        match res {
            Err(sqlx::Error::RowNotFound) => Err(InvalidTenantId(id.into())),
            Err(e) => Err(e.into()),
            Ok(row) => Ok(row),
        }
    }

    #[instrument(skip(self))]
    async fn delete_tenant(&self, id: &str) -> Result<()> {
        let mut query_builder = sqlx::QueryBuilder::new("DELETE FROM public.tenants WHERE id = ");
        query_builder.push_bind(id);
        let query = query_builder.build();

        self.execute(query).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn create_tenant(&self, params: TenantUpdateParams) -> Result<Tenant> {
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(
            "INSERT INTO public.tenants (id)
            VALUES ($1)
            ON CONFLICT (id)
            DO UPDATE SET updated_at = NOW()
            RETURNING *",
        )
        .bind(params.id)
        .fetch_one(self)
        .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn update_tenant_fcm(&self, id: &str, params: TenantFcmUpdateParams) -> Result<Tenant> {
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(
            "UPDATE public.tenants SET fcm_api_key = $2, updated_at = NOW() WHERE id = $1 \
             RETURNING *;",
        )
        .bind(id)
        .bind(params.fcm_api_key)
        .fetch_one(self)
        .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn update_tenant_delete_fcm(&self, id: &str) -> Result<Tenant> {
        let query = "
            UPDATE public.tenants
            SET updated_at = NOW(),
                fcm_api_key = NULL
            WHERE id = $1
            RETURNING *
        ";
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(query)
            .bind(id)
            .fetch_one(self)
            .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn update_tenant_fcm_v1(
        &self,
        id: &str,
        params: TenantFcmV1UpdateParams,
    ) -> Result<Tenant> {
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(
            "UPDATE public.tenants SET fcm_v1_credentials = $2, updated_at = NOW() WHERE id = $1 \
             RETURNING *;",
        )
        .bind(id)
        .bind(params.fcm_v1_credentials)
        .fetch_one(self)
        .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn update_tenant_delete_fcm_v1(&self, id: &str) -> Result<Tenant> {
        let query = "
            UPDATE public.tenants
            SET updated_at = NOW(),
                fcm_v1_credentials = NULL
            WHERE id = $1
            RETURNING *
        ";
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(query)
            .bind(id)
            .fetch_one(self)
            .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn update_tenant_apns(&self, id: &str, params: TenantApnsUpdateParams) -> Result<Tenant> {
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(
            "UPDATE public.tenants SET apns_topic = $2, updated_at = NOW() WHERE id = $1 \
             RETURNING *;",
        )
        .bind(id)
        .bind(params.apns_topic)
        .fetch_one(self)
        .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn update_tenant_apns_auth(
        &self,
        id: &str,
        params: TenantApnsUpdateAuth,
    ) -> Result<Tenant> {
        let res = match params {
            TenantApnsUpdateAuth::Certificate {
                apns_certificate,
                apns_certificate_password,
            } => sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(
                "UPDATE public.tenants SET apns_type = 'certificate'::apns_type, apns_certificate \
                 = $2, apns_certificate_password = $3, apns_pkcs8_pem = null, apns_team_id = \
                 null, apns_key_id = null, updated_at = NOW() WHERE id = $1 RETURNING *;",
            )
            .bind(id)
            .bind(apns_certificate)
            .bind(apns_certificate_password),
            TenantApnsUpdateAuth::Token {
                apns_pkcs8_pem,
                apns_team_id,
                apns_key_id,
            } => sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(
                "UPDATE public.tenants SET apns_type = 'token'::apns_type, apns_pkcs8_pem = $2, \
                 apns_team_id = $3, apns_key_id = $4, apns_certificate = null, \
                 apns_certificate_password = null, updated_at = NOW() WHERE id = $1 RETURNING *;",
            )
            .bind(id)
            .bind(apns_pkcs8_pem)
            .bind(apns_team_id)
            .bind(apns_key_id),
        }
        .fetch_one(self)
        .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn update_tenant_delete_apns(&self, id: &str) -> Result<Tenant> {
        let query = "
            UPDATE public.tenants
            SET updated_at = NOW(),
                apns_topic = NULL,
                apns_type = NULL,
                apns_certificate = NULL,
                apns_certificate_password = NULL,
                apns_pkcs8_pem = NULL,
                apns_team_id = NULL,
                apns_key_id = NULL
            WHERE id = $1
            RETURNING *
        ";
        let res = sqlx::query_as::<sqlx::postgres::Postgres, Tenant>(query)
            .bind(id)
            .fetch_one(self)
            .await?;

        Ok(res)
    }

    #[instrument(skip(self))]
    async fn suspend_tenant(&self, id: &str, reason: &str) -> Result<()> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "UPDATE public.tenants SET suspended = true, updated_at = NOW(), suspended_reason =",
        );
        query_builder.push_bind(reason);
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);
        let query = query_builder.build();

        self.execute(query).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn unsuspend_tenant(&self, id: &str) -> Result<()> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "UPDATE public.tenants SET suspended = false, updated_at = NOW(), suspended_reason = \
             null WHERE id = ",
        );
        query_builder.push_bind(id);
        let query = query_builder.build();

        self.execute(query).await?;

        Ok(())
    }
}

#[cfg(not(feature = "multitenant"))]
pub struct DefaultTenantStore(Tenant);

#[cfg(not(feature = "multitenant"))]
impl DefaultTenantStore {
    pub fn new(config: Arc<Config>) -> Result<DefaultTenantStore> {
        Ok(DefaultTenantStore(Tenant {
            id: DEFAULT_TENANT_ID.to_string(),
            // Legacy FCM is retired; single-tenant deployments configure FCM v1.
            fcm_api_key: None,
            fcm_v1_credentials: config.fcm_v1_credentials.clone(),
            apns_type: config.apns_type,
            apns_topic: config.apns_topic.clone(),
            apns_certificate: config.apns_certificate.clone(),
            apns_certificate_password: config.apns_certificate_password.clone(),
            apns_pkcs8_pem: config.apns_pkcs8_pem.clone(),
            apns_key_id: config.apns_key_id.clone(),
            apns_team_id: config.apns_team_id.clone(),
            suspended: false,
            suspended_reason: None,
            created_at: Default::default(),
            updated_at: Default::default(),
        }))
    }
}

#[async_trait]
#[cfg(not(feature = "multitenant"))]
impl TenantStore for DefaultTenantStore {
    async fn get_tenant(&self, _id: &str) -> Result<Tenant> {
        Ok(self.0.clone())
    }

    async fn delete_tenant(&self, _id: &str) -> Result<()> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn create_tenant(&self, _params: TenantUpdateParams) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn update_tenant_fcm(&self, _id: &str, _params: TenantFcmUpdateParams) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn update_tenant_delete_fcm(&self, _id: &str) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn update_tenant_fcm_v1(
        &self,
        _id: &str,
        _params: TenantFcmV1UpdateParams,
    ) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn update_tenant_delete_fcm_v1(&self, _id: &str) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn update_tenant_apns(
        &self,
        _id: &str,
        _params: TenantApnsUpdateParams,
    ) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn update_tenant_apns_auth(
        &self,
        _id: &str,
        _params: TenantApnsUpdateAuth,
    ) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn update_tenant_delete_apns(&self, _id: &str) -> Result<Tenant> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn suspend_tenant(&self, _id: &str, _reason: &str) -> Result<()> {
        panic!("Shouldn't have run in single tenant mode")
    }

    async fn unsuspend_tenant(&self, _id: &str) -> Result<()> {
        panic!("Shouldn't have run in single tenant mode")
    }
}
