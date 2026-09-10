//! Legacy FCM ("server key") credentials are no longer accepted — Google decommissioned
//! the legacy FCM HTTP API in June 2024. The tests that registered a legacy key and
//! asserted success, plus the `enabled_providers` and `tenant_delete` variants built on
//! them, are gone; their FCM v1 equivalents live in `fcm_v1.rs`. No credential is needed
//! here any more, so ECHO_TEST_FCM_KEY is no longer read.

use {
    crate::{context::EchoServerContext, functional::multitenant::generate_random_tenant_id},
    echo_server::{
        handlers::{create_tenant::TenantRegisterBody, get_tenant::GetTenantResponse},
        providers::PROVIDER_FCM,
    },
    test_context::test_context,
};

/// Any legacy server key is refused, and nothing is stored. This replaces
/// `tenant_update_fcm_valid` (which asserted the opposite) and `tenant_update_fcm_bad`
/// (which could not pass while the dead API was consulted for validation).
#[test_context(EchoServerContext)]
#[tokio::test]
async fn tenant_update_fcm_is_rejected(ctx: &mut EchoServerContext) {
    let (tenant_id, jwt_token) = generate_random_tenant_id(&ctx.config.jwt_secret);

    // Register tenant
    let client = reqwest::Client::new();
    let register_response = client
        .post(format!("http://{}/tenants", ctx.server.public_addr))
        .bearer_auth(&jwt_token)
        .json(&TenantRegisterBody {
            id: tenant_id.clone(),
        })
        .send()
        .await
        .expect("Call failed");
    assert_eq!(register_response.status(), reqwest::StatusCode::OK);

    let form = reqwest::multipart::Form::new().text("api_key", "invalid-key");

    let response = client
        .post(format!(
            "http://{}/tenants/{}/fcm",
            ctx.server.public_addr, tenant_id
        ))
        .bearer_auth(&jwt_token)
        .multipart(form)
        .send()
        .await
        .expect("Call failed");

    assert_eq!(
        response.status(),
        reqwest::StatusCode::GONE,
        "legacy FCM credentials must be refused"
    );

    // The tenant is left with no FCM provider, rather than one that cannot deliver.
    let response = client
        .get(format!(
            "http://{}/tenants/{}",
            ctx.server.public_addr, tenant_id
        ))
        .bearer_auth(&jwt_token)
        .send()
        .await
        .expect("Call failed");
    assert!(response.status().is_success());
    let response = response.json::<GetTenantResponse>().await.unwrap();
    assert!(
        !response
            .enabled_providers
            .contains(&PROVIDER_FCM.to_owned()),
        "a rejected legacy key must not enable the FCM provider"
    );
}
