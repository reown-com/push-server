use {
    crate::{
        error::Error::{self},
        handlers::validate_tenant_request,
        increment_counter,
        state::AppState,
    },
    axum::{
        extract::{Path, State},
        http::HeaderMap,
    },
    hyper::StatusCode,
    std::sync::Arc,
    tracing::{error, instrument},
};

#[instrument(skip_all, name = "delete_fcm_v1_handler")]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<StatusCode, Error> {
    // JWT token verification
    #[cfg(feature = "cloud")]
    let jwt_verification_result =
        validate_tenant_request(&state.jwt_validation_client, &headers, &id).await;

    // -- check if tenant is real
    let _existing_tenant = state.tenant_store.get_tenant(&id).await?;

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

    let new_tenant = state.tenant_store.update_tenant_delete_fcm_v1(&id).await?;

    if new_tenant.suspended {
        // If suspended, it can be restored now because valid credentials have been
        // provided
        state.tenant_store.unsuspend_tenant(&new_tenant.id).await?;
    }

    increment_counter!(state.metrics, tenant_fcm_v1_updates);

    Ok(StatusCode::NO_CONTENT)
}
