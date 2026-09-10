use {
    crate::{
        handlers::{ErrorField, ErrorLocation, ResponseError},
        log::prelude::*,
        middleware::validate_signature::{SIGNATURE_HEADER_NAME, TIMESTAMP_HEADER_NAME},
        stores::StoreError,
    },
    axum::response::{IntoResponse, Response},
    hyper::StatusCode,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(is_variant_derive::IsVariant, Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Envy(#[from] envy::Error),

    #[error("Bad device token error: {0}")]
    BadDeviceToken(String),

    #[error(transparent)]
    Apns(#[from] a2::Error),

    #[error("APNS Responded with error, {0}")]
    ApnsResponse(a2::ErrorReason),

    #[error(transparent)]
    FcmV1(#[from] fcm_v1::SendError),

    #[error("FCM v1 Responded with an error")]
    FcmV1Response(fcm_v1::ErrorReason),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Hex(hex::FromHexError),

    #[error(transparent)]
    Ed25519(#[from] ed25519_dalek::ed25519::Error),

    #[error(transparent)]
    HttpRequest(#[from] reqwest::Error),

    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),

    #[error("Failed to decode legacy decrypted message: {0}")]
    DecryptedNotificationDecode(base64::DecodeError),

    #[error("Failed to parse legacy decrypted message as JSON: {0}")]
    DecryptedNotificationParse(serde_json::Error),

    #[error("Invalid ServiceServiceAccount key: {0}")]
    FcmV1InvalidServiceAccountKey(serde_json::Error),

    #[error("Internal: Invalid ServiceServiceAccount key: {0}")]
    InternalFcmV1InvalidServiceAccountKey(serde_json::Error),

    #[error("Failed to perform internal serialization: {0}")]
    InternalSerializationError(serde_json::Error),

    #[error(transparent)]
    Store(#[from] StoreError),

    #[error(transparent)]
    ToStr(#[from] axum::http::header::ToStrError),

    #[error("database migration failed: {0}")]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),

    #[error("{0} is an invalid push provider as it cannot be not found")]
    ProviderNotFound(String),

    #[error("{0} is an invalid push provider as it has not been enabled")]
    ProviderNotAvailable(String),

    #[error("the `{0}` field must not be empty")]
    EmptyField(String),

    #[error("a required environment variable cannot be found")]
    RequiredEnvNotFound,

    #[error("timestamp header cannot not found")]
    MissingTimestampHeader,

    #[error("signature header cannot not found")]
    MissingSignatureHeader,

    #[error("middleware T::from_request failed")]
    FromRequestError,

    #[error("middleware failed to parse body")]
    ToBytesError,

    #[error("neither signature or timestamp header cannot not found")]
    MissingAllSignatureHeader,

    #[error("single-tenant request made while echo server in multi-tenant mode")]
    MissingTenantId,

    #[error("multi-tenant request made while echo server in single-tenant mode")]
    IncludedTenantIdWhenNotNeeded,

    #[error("invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("invalid tenant id: {0}")]
    InvalidTenantId(String),

    #[error("invalid options provided for {0}")]
    InvalidOptionsProvided(String),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    MultipartError(#[from] axum::extract::multipart::MultipartError),

    #[error("The provided multi-part body did not satisfy the requirements")]
    InvalidMultipartBody,

    #[error("invalid apns type: {0}")]
    InvalidApnsType(String),

    #[error("cannot get type when APNS is not configured")]
    NoApnsConfigured,

    #[error(
        "Encrypted push notification received without a topic, please ensure all required \
         parameters set"
    )]
    MissingTopic,

    #[error("client cannot be found")]
    ClientNotFound,

    #[error("this should not have occurred; used when case has been handled before")]
    InternalServerError,

    #[error(transparent)]
    JwtError(#[from] relay_rpc::jwt::JwtError),

    #[error("the provided authentication does not authenticate the request")]
    InvalidAuthentication,

    #[error("GeoIpReader Error: {0}")]
    GeoIpReader(String),

    #[error("BatchCollector Error: {0}")]
    BatchCollector(String),

    #[error("Invalid Project ID: {0}")]
    InvalidProjectId(String),

    #[error(transparent)]
    JWT(#[from] jsonwebtoken::errors::Error),

    #[error("failed to load geoip database from s3")]
    GeoIpS3Failed,

    #[error("tenant id and client's registered tenant didn't match")]
    MissmatchedTenantId,

    /// Legacy FCM ("server key") credentials: Google decommissioned the legacy FCM
    /// HTTP API in June 2024, so such a key can neither be validated nor used to
    /// deliver. Returned when one is submitted, and when a tenant carrying only a
    /// legacy key needs a provider built.
    #[error("legacy FCM server keys are no longer supported, use FCM v1")]
    LegacyFcmApiRetired,

    #[error("Invalid FCM v1 credentials")]
    BadFcmV1Credentials,

    #[error("Invalid APNs creds")]
    BadApnsCredentials,

    #[error("Expired APNs certificate")]
    ApnsCertificateExpired,

    #[error("Unknown CA of APNs certificate")]
    ApnsCertificateUnknownCA,

    #[error("Invalid APNs provider token")]
    ApnsInvalidProviderToken,

    #[error("client deleted due to invalid device token")]
    ClientDeleted,

    #[error("tenant suspended due to invalid configuration")]
    TenantSuspended,

    #[error("Payload is too large")]
    PayloadTooLarge,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let response = match &self {
            Error::BadDeviceToken(e) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "invalid_token".to_string(),
                    message: e.to_string(),
                }
            ], vec![
                ErrorField {
                    field: "token".to_string(),
                    description: "Invalid device token".to_string(),
                    location: ErrorLocation::Body,
                }
            ]),
            Error::Apns(e) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "apns".to_string(),
                    message: e.to_string(),
                }
            ], vec![]),
            Error::ApnsResponse(e) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "apns_response".to_string(),
                    message: e.to_string(),
                }
            ], vec![]),
            Error::BadApnsCredentials => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "bad_apns_credentials".to_string(),
                    message: "Failed to validate the provided Certificate or Token".to_string(),
                }
            ], vec![]),
            Error::LegacyFcmApiRetired => crate::handlers::Response::new_failure(StatusCode::GONE, vec![
                ResponseError {
                    name: "legacy_fcm_api_retired".to_string(),
                    message: "Legacy FCM server keys are no longer supported: Google decommissioned the legacy FCM HTTP API. Register a service account key with POST /:id/fcm_v1 instead".to_string(),
                }
            ], vec![
                ErrorField {
                    field: "api_key".to_string(),
                    description: "Legacy FCM server keys can no longer be used to send notifications; use fcm_v1_credentials".to_string(),
                    location: ErrorLocation::Body,
                }
            ]),
            Error::FcmV1InvalidServiceAccountKey(e) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "fcm_v1_invalid_service_account_key".to_string(),
                    message: format!("The provided Service Account Key was not valid: {e}"),
                }
            ], vec![
                ErrorField {
                    field: "fcm_v1_credentials".to_string(),
                    description: "FCM V1 credentials".to_string(),
                    location: ErrorLocation::Body,
                }
            ]),
            Error::BadFcmV1Credentials => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "bad_fcm_v1_credentials".to_string(),
                    message: "The provided credentials were not valid".to_string(),
                }
            ], vec![
                ErrorField {
                    field: "fcm_v1_credentials".to_string(),
                    description: "FCM V1 credentials".to_string(),
                    location: ErrorLocation::Body,
                }
            ]),
            Error::Database(e) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "sqlx".to_string(),
                    message: e.to_string(),
                }
            ], vec![]),
            Error::Hex(e) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "from_hex".to_string(),
                    message: e.to_string(),
                }
            ], vec![]),
            Error::Ed25519(e) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "ed25519".to_string(),
                    message: e.to_string(),
                }
            ], vec![]),
            Error::HttpRequest(e) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "http_request".to_string(),
                    message: e.to_string(),
                }
            ], vec![]),
            Error::Base64Decode(e) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "base64_decode".to_string(),
                    message: e.to_string(),
                }
            ], vec![]),
            Error::Store(e) => match e {
                StoreError::Database(e) => crate::handlers::Response::new_failure(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    vec![ResponseError {
                        name: "sqlx".to_string(),
                        message: e.to_string(),
                    }],
                    vec![],
                ),
                StoreError::NotFound(entity, id) => crate::handlers::Response::new_failure(
                    StatusCode::NOT_FOUND,
                    vec![],
                    vec![ErrorField {
                        field: format!("{}.id", entity),
                        description: format!("Cannot find {entity} with specified identifier {id}"),
                        location: ErrorLocation::Body,
                    }],
                ),
            },
            Error::ProviderNotFound(p) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "provider_not_available".to_string(),
                    message: format!("The requested provider ({}) is not a valid provider", p),
                }
            ], vec![
                ErrorField {
                    field: "provider".to_string(),
                    description: format!("The requested provider ({}) is not a valid provider", p),
                    location: ErrorLocation::Body,
                }
            ]),
            Error::ProviderNotAvailable(p) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "provider_not_available".to_string(),
                    message: format!("The requested provider ({}) is not currently available", p),
                }
            ], vec![
                ErrorField {
                    field: "provider".to_string(),
                    description: format!("The requested provider ({}) is not currently available", p),
                    location: ErrorLocation::Body,
                }
            ]),
            Error::MissingAllSignatureHeader => crate::handlers::Response::new_failure(StatusCode::UNAUTHORIZED, vec![
                ResponseError {
                    name: "webhook_validation_failed".to_string(),
                    message: "Failed to validate webhook, please ensure that all required headers are provided.".to_string(),
                }
            ], vec![
                ErrorField {
                    field: SIGNATURE_HEADER_NAME.to_string(),
                    description: "Missing signature".to_string(),
                    location: ErrorLocation::Header,
                },
                ErrorField {
                    field: TIMESTAMP_HEADER_NAME.to_string(),
                    description: "Missing timestamp".to_string(),
                    location: ErrorLocation::Header,
                },
            ]),
            Error::MissingSignatureHeader => crate::handlers::Response::new_failure(StatusCode::UNAUTHORIZED, vec![
                ResponseError {
                    name: "webhook_validation_failed".to_string(),
                    message: "Failed to validate webhook, please ensure that all required headers are provided.".to_string(),
                }
            ], vec![
                ErrorField {
                    field: SIGNATURE_HEADER_NAME.to_string(),
                    description: "Missing signature".to_string(),
                    location: ErrorLocation::Header,
                }
            ]),
            Error::MissingTimestampHeader => crate::handlers::Response::new_failure(StatusCode::UNAUTHORIZED, vec![
                ResponseError {
                    name: "webhook_validation_failed".to_string(),
                    message: "Failed to validate webhook, please ensure that all required headers are provided.".to_string(),
                }
            ], vec![
                ErrorField {
                    field: TIMESTAMP_HEADER_NAME.to_string(),
                    description: "Missing timestamp".to_string(),
                    location: ErrorLocation::Header,
                }
            ]),
            Error::InvalidTenantId(id) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "tenant".to_string(),
                    message: format!("The provided Tenant ID, {}, is invalid. Please ensure it's valid and the url is in the format /:tenant_id/...path", id),
                }
            ], vec![
                ErrorField {
                    field: "tenant_id".to_string(),
                    description: format!("Invalid Tenant ID, {}", id),
                    location: ErrorLocation::Path,
                }
            ]),
            Error::MissingTenantId => crate::handlers::Response::new_failure(
                StatusCode::BAD_REQUEST,
                vec![ResponseError {
                    name: "tenancy-mode".to_string(),
                    message: "single-tenant request made while echo server in multi-tenant mode".to_string(),
                }],
                vec![],
            ),
            Error::IncludedTenantIdWhenNotNeeded => crate::handlers::Response::new_failure(
                StatusCode::BAD_REQUEST,
                vec![ResponseError {
                    name: "tenancy-mode".to_string(),
                    message: "multi-tenant request made while echo server in single-tenant mode".to_string(),
                }],
                vec![],
            ),
            Error::MissingTopic => crate::handlers::Response::new_failure(
                StatusCode::BAD_REQUEST,
                vec![ResponseError {
                    name: "topic".to_string(),
                    message: "encrypted push notifications require topic to be set".to_string(),
                }],
                vec![],
            ),
            // If the client cannot be found we gracefully handle this
            Error::ClientNotFound => crate::handlers::Response::new_success(StatusCode::ACCEPTED),
            Error::JwtError(_) | Error::InvalidAuthentication => crate::handlers::Response::new_failure(
                StatusCode::UNAUTHORIZED,
                vec![ResponseError {
                    name: "authentication".to_string(),
                    message: "the provided authentication is not sufficient".to_string(),
                }],
                vec![
                    ErrorField {
                        field: axum::http::header::AUTHORIZATION.to_string(),
                        description: "invalid authorization token".to_string(),
                        location: ErrorLocation::Header,
                    }
                ],
            ),
            Error::InvalidProjectId(id) => crate::handlers::Response::new_failure(
                StatusCode::BAD_REQUEST,
                vec![ResponseError {
                    name: "project_id".to_string(),
                    message: format!("the provided project id ({}) is not valid", id),
                }],
                vec![
                    ErrorField {
                        field: "id".to_string(),
                        description: "invalid project id".to_string(),
                        location: ErrorLocation::Body,
                    }
                ],
            ),
            Error::Io(_) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "io".to_string(),
                    message: "failed to perform io task, this should not have occurred, please report at: https://github.com/walletconnect/echo-server".to_string(),
                },
            ], vec![]),
            Error::DecryptedNotificationDecode(_) => crate::handlers::Response::new_failure(StatusCode::ACCEPTED, vec![
                ResponseError {
                    name: "json".to_string(),
                    message: "Decrypted notification does not decode as base64".to_string(),
                },
            ], vec![]),
            Error::DecryptedNotificationParse(_) => crate::handlers::Response::new_failure(StatusCode::ACCEPTED, vec![
                ResponseError {
                    name: "json".to_string(),
                    message: "Decrypted notification does not parse as JSON".to_string(),
                },
            ], vec![]),
            Error::ToStr(_) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "header_decode".to_string(),
                    message: "failed to decode header using axum, please try again or report error at: https://github.com/walletconnect/echo-server".to_string(),
                },
            ], vec![]),
            Error::EmptyField(f) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "field".to_string(),
                    message: "field is missing from request".to_string(),
                },
            ], vec![
                ErrorField {
                    field: f.to_owned(),
                    description: "missing from request".to_string(),
                    // Note (Harry Bairstow): Currently only used in body
                    location: ErrorLocation::Body,
                }
            ]),
            Error::FromRequestError => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "unknown".to_string(),
                    message: "unknown error when forwarding request without signature validation, please report this at: https://github.com/walletconnect/echo-server".to_string(),
                },
            ], vec![]),
            Error::ToBytesError => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "decode".to_string(),
                    message: "failed to decode body as bytes, please ensure your JSON body is encoded properly".to_string(),
                },
            ], vec![]),
            // Figure out
            // TODO: Error::InvalidOptionsProvided(_) => {}
            Error::FromUtf8Error(_) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "decode".to_string(),
                    message: "failed to decode body as Utf8, please ensure your JSON body is encoded properly".to_string(),
                },
            ], vec![]),
            Error::MultipartError(e) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "body".to_string(),
                    message: "failed to decode the multipart body".to_string(),
                },
                ResponseError {
                    name: "multipart".to_string(),
                    message: format!("{:?}", e),
                },
            ], vec![]),
            Error::InvalidMultipartBody => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "body".to_string(),
                    message: "multipart body did not conform to specification".to_string(),
                },
            ], vec![]),
            Error::InvalidApnsType(t) => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "decoding_error".to_string(),
                    message: format!("failed to decode apns type, \"{}\" is invalid", t),
                },
            ], vec![
                ErrorField {
                    field: "type".to_string(),
                    description: "apns push type decoding failed".to_string(),
                    location: ErrorLocation::Unknown,
                }
            ]),
            Error::InternalServerError => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "unknown_error".to_string(),
                    message: "This error should not have occurred. Please file an issue at: https://github.com/walletconnect/echo-server".to_string(),
                },
            ], vec![]),
            Error::GeoIpReader(_) | Error::BatchCollector(_) => crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                ResponseError {
                    name: "o11y".to_string(),
                    message: "Internal error monitoring the request".to_string(),
                },
            ], vec![]),
            Error::JWT(_) => crate::handlers::Response::new_failure(StatusCode::UNAUTHORIZED, vec![
                ResponseError {
                    name: "invalid_auth".to_string(),
                    message: "JWT Authentication Failed".to_string(),
                },
            ], vec![]),
            Error::MissmatchedTenantId => crate::handlers::Response::new_failure(StatusCode::BAD_REQUEST, vec![
                ResponseError {
                    name: "missmatched_identifiers".to_string(),
                    message: "The requested tenant doesn't have this client registered".to_string(),
                },
            ], vec![
                ErrorField {
                    field: "tenant_id".to_string(),
                    description: "doesn't match registered id".to_string(),
                    location: ErrorLocation::Path,
                },
                ErrorField {
                    field: "id".to_string(),
                    description: "doesn't match registered id".to_string(),
                    location: ErrorLocation::Path,
                }
            ]),
            Error::ClientDeleted => crate::handlers::Response::new_failure(StatusCode::ACCEPTED, vec![
                ResponseError {
                    name: "client_deleted".to_string(),
                    message: "Request Accepted, client deleted due to invalid token".to_string(),
                },
            ], vec![]),
            Error::TenantSuspended => crate::handlers::Response::new_failure(StatusCode::ACCEPTED, vec![
                ResponseError {
                    name: "tenant_suspended".to_string(),
                    message: "Request Accepted, tenant suspended due to invalid configuration".to_string(),
                },
            ], vec![]),
            Error::PayloadTooLarge => crate::handlers::Response::new_failure(
                StatusCode::BAD_REQUEST,
                vec![ResponseError {
                    name: "body".to_string(),
                    message: "Message payload is too large".to_string(),
                }],
                vec![],
            ),
            e => {
                warn!("Error does not have response clause, {:?}", e);

                crate::handlers::Response::new_failure(StatusCode::INTERNAL_SERVER_ERROR, vec![
                    ResponseError {
                        name: "unknown_error".to_string(),
                        message: "This error should not have occurred. Please file an issue at: https://github.com/walletconnect/echo-server".to_string(),
                    },
                    ResponseError {
                        name: "dbg".to_string(),
                        message: format!("{e:?}"),
                    },
                ], vec![])
            }
        }.into_response();

        if response.status().is_client_error() {
            warn!("HTTP client error: {self:?}");
        }

        if response.status().is_server_error() {
            error!("HTTP server error: {self:?}");
        }

        response
    }
}
