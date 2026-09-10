use {
    crate::{
        error::{
            Error,
            Error::{BadFcmApiKey, InvalidMultipartBody},
        },
        handlers::validate_tenant_request,
        increment_counter,
        state::AppState,
        stores::tenant::TenantFcmUpdateParams,
    },
    axum::{
        extract::{Multipart, Path, State},
        http::HeaderMap,
        Json,
    },
    fcm::FcmError,
    serde::Serialize,
    std::sync::Arc,
    tracing::{error, instrument},
};

pub struct FcmUpdateBody {
    api_key: String,
    /// Used to ensure that at least one value has changed
    value_changed_: bool,
}

#[derive(Serialize)]
pub struct UpdateTenantFcmResponse {
    success: bool,
}

#[instrument(skip_all, name = "update_fcm_handler")]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    mut form_body: Multipart,
) -> Result<Json<UpdateTenantFcmResponse>, Error> {
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

    // -- check if tenant is real
    let _existing_tenant = state.tenant_store.get_tenant(&id).await?;

    // ---- retrieve body from form
    let mut body = FcmUpdateBody {
        api_key: Default::default(),
        value_changed_: false,
    };
    while let Some(field) = form_body.next_field().await? {
        let name = field.name().unwrap_or("unknown").to_string();
        let data = field.text().await?;

        if name.to_lowercase().as_str() == "api_key" {
            body.api_key = data;
            body.value_changed_ = true;
        };
    }
    if !body.value_changed_ {
        return Err(InvalidMultipartBody);
    }

    // ---- checks
    // NOTE: this validates nothing any more. Google decommissioned the legacy FCM HTTP
    // API in June 2024, so the dry-run send below fails with 404 for every key, valid
    // or not, and only FcmError::Unauthorized (401) maps to BadFcmApiKey — so any key
    // is accepted and stored, and a suspended tenant is restored on it. The legacy
    // provider in src/providers/fcm.rs sends through the same dead endpoint. Fixing
    // this properly means dropping legacy FCM in favour of the v1 path, which is a
    // product decision; see tenant_update_fcm_bad in tests/functional/multitenant/fcm.rs.
    let fcm_api_key = body.api_key.clone();
    let mut test_message_builder = fcm::MessageBuilder::new(&fcm_api_key, "wc-notification-test");
    test_message_builder.dry_run(true);
    let test_message = test_message_builder.finalize();
    let test_notification = fcm::Client::new().send(test_message).await;
    match test_notification {
        Err(e) => match e {
            FcmError::Unauthorized => Err(BadFcmApiKey),
            _ => Ok(()),
        },
        Ok(_) => Ok(()),
    }?;

    // ---- handler
    let update_body = TenantFcmUpdateParams {
        fcm_api_key: body.api_key,
    };

    let new_tenant = state
        .tenant_store
        .update_tenant_fcm(&id, update_body)
        .await?;

    if new_tenant.suspended {
        // If suspended, it can be restored now because valid credentials have been
        // provided
        state.tenant_store.unsuspend_tenant(&new_tenant.id).await?;
    }

    increment_counter!(state.metrics, tenant_fcm_updates);

    Ok(Json(UpdateTenantFcmResponse { success: true }))
}
