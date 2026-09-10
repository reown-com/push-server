pub mod apns;
pub mod fcm_v1;
#[cfg(any(debug_assertions, test))]
pub mod noop;

use {
    self::fcm_v1::FcmV1Provider,
    crate::{blob::ENCRYPTED_FLAG, error, providers::apns::ApnsProvider},
    async_trait::async_trait,
    relay_rpc::rpc::msg_id::get_message_id,
    serde::{Deserialize, Serialize},
    std::{
        fmt::{Display, Formatter},
        sync::Arc,
    },
    tracing::instrument,
};

#[cfg(any(debug_assertions, test))]
use crate::providers::noop::NoopProvider;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PushMessage {
    LegacyPushMessage(LegacyPushMessage),
    RawPushMessage(RawPushMessage),
}

impl PushMessage {
    pub fn message_id(&self) -> Arc<str> {
        match self {
            Self::RawPushMessage(msg) => get_message_id(&msg.message).into(),
            Self::LegacyPushMessage(msg) => msg.id.clone(),
        }
    }

    pub fn topic(&self) -> Arc<str> {
        match self {
            Self::RawPushMessage(msg) => msg.topic.clone(),
            Self::LegacyPushMessage(msg) => msg.payload.topic.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct LegacyPushMessage {
    pub id: Arc<str>,
    pub payload: MessagePayload,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct MessagePayload {
    pub topic: Arc<str>,
    pub flags: u32,
    pub blob: Arc<str>,
}

impl MessagePayload {
    pub fn is_encrypted(&self) -> bool {
        (self.flags & ENCRYPTED_FLAG) == ENCRYPTED_FLAG
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct RawPushMessage {
    /// Topic is used by the SDKs to decrypt
    /// encrypted payloads on the client side
    pub topic: Arc<str>,
    /// Filtering tag
    pub tag: u32,
    /// The payload message
    pub message: Arc<str>,
}

#[async_trait]
pub trait PushProvider {
    async fn send_notification(&self, token: String, body: PushMessage) -> error::Result<()>;
}

pub const PROVIDER_APNS: &str = "apns";
pub const PROVIDER_APNS_SANDBOX: &str = "apns-sandbox";
pub const PROVIDER_FCM: &str = "fcm";
pub const PROVIDER_FCM_V1: &str = "fcm_v1";
#[cfg(any(debug_assertions, test))]
pub const PROVIDER_NOOP: &str = "noop";

#[derive(Debug, Copy, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "provider")]
#[sqlx(rename_all = "lowercase")]
pub enum ProviderKind {
    Apns,
    ApnsSandbox,
    Fcm,
    // Intentionally no FcmV1 variant because ProviderKind is also used to determine token type (of which FCM and FCM V1 are the same)
    #[cfg(any(debug_assertions, test))]
    Noop,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Apns => PROVIDER_APNS,
            Self::ApnsSandbox => PROVIDER_APNS_SANDBOX,
            Self::Fcm => PROVIDER_FCM,
            #[cfg(any(debug_assertions, test))]
            Self::Noop => PROVIDER_NOOP,
        }
    }
}

impl Display for ProviderKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&ProviderKind> for String {
    fn from(val: &ProviderKind) -> Self {
        val.as_str().to_string()
    }
}

impl From<ProviderKind> for String {
    fn from(val: ProviderKind) -> Self {
        val.as_str().to_string()
    }
}

impl From<ProviderKind> for &str {
    fn from(val: ProviderKind) -> Self {
        val.as_str()
    }
}

impl TryFrom<&str> for ProviderKind {
    type Error = error::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            PROVIDER_APNS => Ok(Self::Apns),
            PROVIDER_APNS_SANDBOX => Ok(Self::ApnsSandbox),
            PROVIDER_FCM => Ok(Self::Fcm),
            #[cfg(any(debug_assertions, test))]
            PROVIDER_NOOP => Ok(Self::Noop),
            _ => Err(error::Error::ProviderNotFound(value.to_owned())),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Provider {
    FcmV1(FcmV1Provider),
    Apns(ApnsProvider),
    #[cfg(any(debug_assertions, test))]
    Noop(NoopProvider),
}

#[async_trait]
impl PushProvider for Provider {
    #[instrument(name = "send_notification")]
    async fn send_notification(&self, token: String, body: PushMessage) -> error::Result<()> {
        match self {
            Provider::FcmV1(p) => p.send_notification(token, body).await,
            Provider::Apns(p) => p.send_notification(token, body).await,
            #[cfg(any(debug_assertions, test))]
            Provider::Noop(p) => p.send_notification(token, body).await,
        }
    }
}
