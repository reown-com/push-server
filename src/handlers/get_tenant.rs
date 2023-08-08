use {
    crate::{
        error::Error,
        handlers::validate_tenant_request,
        log::prelude::*,
        providers::ProviderKind,
        request_id::get_req_id,
        state::AppState,
        stores::tenant::ApnsType,
    },
    axum::{
        extract::{Path, State},
        http::HeaderMap,
        Json,
    },
    serde::Serialize,
    std::sync::Arc,
};

#[derive(Serialize)]
pub struct GetTenantResponse {
    url: String,
    enabled_providers: Vec<String>,
    apns_topic: Option<String>,
    apns_type: Option<ApnsType>,
    suspended: bool,
    suspended_reason: Option<String>,
}

pub async fn handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<GetTenantResponse>, Error> {
    let request_id = get_req_id(&headers);

    #[cfg(feature = "cloud")]
    let verification_res = validate_tenant_request(
        &state.registry_client,
        &state.gotrue_client,
        &headers,
        id.clone(),
        None,
    )
    .await;

    #[cfg(not(feature = "cloud"))]
    let verification_res = validate_tenant_request(&state.gotrue_client, &headers);

    if let Err(e) = verification_res {
        error!(
            request_id = %request_id,
            tenant_id = %id,
            err = ?e,
            "JWT verification failed"
        );
        return Err(e);
    }

    let tenant = state.tenant_store.get_tenant(&id).await?;

    let providers = tenant.providers();

    let mut res = GetTenantResponse {
        url: format!("{}/{}", state.config.public_url, tenant.id),
        enabled_providers: tenant.providers().iter().map(Into::into).collect(),
        apns_topic: None,
        apns_type: None,
        suspended: tenant.suspended,
        suspended_reason: tenant.suspended_reason,
    };

    if providers.contains(&ProviderKind::Apns) {
        res.apns_topic = tenant.apns_topic;
        res.apns_type = tenant.apns_type;
    }

    info!(
        %request_id,
        tenant_id = %id,
        "requested tenant"
    );

    Ok(Json(res))
}
