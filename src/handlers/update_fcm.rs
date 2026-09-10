use {
    crate::{
        error::{Error, Error::LegacyFcmApiRetired},
        handlers::validate_tenant_request,
        state::AppState,
    },
    axum::{
        extract::{Path, State},
        http::HeaderMap,
    },
    std::sync::Arc,
    tracing::{error, instrument, warn},
};

/// Legacy FCM server keys are no longer accepted.
///
/// Google decommissioned the legacy FCM HTTP API in June 2024 — a POST to
/// https://fcm.googleapis.com/fcm/send now answers 404 — so a key supplied here could
/// neither be validated nor used to deliver a notification. This endpoint previously
/// "validated" the key with a dry-run send and mapped everything except a 401 to
/// success, which meant it accepted any string, stored it, and restored a suspended
/// tenant on the strength of it.
///
/// Tenants supply a service account key to `POST /:id/fcm_v1` instead; see
/// [`crate::handlers::update_fcm_v1`]. `DELETE /:id/fcm` still works, so a tenant
/// carrying a stale legacy key can clear it.
#[instrument(skip_all, name = "update_fcm_handler")]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<(), Error> {
    // JWT token verification
    #[cfg(feature = "cloud")]
    let jwt_verification_result =
        validate_tenant_request(&state.jwt_validation_client, &headers, &id).await;

    #[cfg(not(feature = "cloud"))]
    let jwt_verification_result = validate_tenant_request(&state.jwt_validation_client, &headers);

    if let Err(e) = jwt_verification_result {
        error!(
            tenant_id = %id,
            err = ?e,
            "JWT verification failed"
        );
        return Err(e);
    }

    warn!(
        tenant_id = %id,
        "rejected legacy FCM credentials, tenant must migrate to /fcm_v1"
    );

    Err(LegacyFcmApiRetired)
}
