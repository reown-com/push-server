use {
    crate::{context::EchoServerContext, functional::multitenant::generate_random_tenant_id},
    echo_server::{
        handlers::{create_tenant::TenantRegisterBody, get_tenant::GetTenantResponse},
        providers::PROVIDER_FCM,
    },
    std::env,
    test_context::test_context,
};

#[test_context(EchoServerContext)]
#[tokio::test]
async fn tenant_update_fcm_valid(ctx: &mut EchoServerContext) {
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

    // Send valid API Key
    let api_key = env::var("ECHO_TEST_FCM_KEY").unwrap();
    let form = reqwest::multipart::Form::new().text("api_key", api_key);

    let response_fcm_update = client
        .post(format!(
            "http://{}/tenants/{}/fcm",
            ctx.server.public_addr, tenant_id
        ))
        .bearer_auth(&jwt_token)
        .multipart(form)
        .send()
        .await
        .expect("Call failed");
    assert_eq!(response_fcm_update.status(), reqwest::StatusCode::OK);
}

#[test_context(EchoServerContext)]
#[tokio::test]
async fn tenant_enabled_providers(ctx: &mut EchoServerContext) {
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

    // Send valid API Key
    let api_key = env::var("ECHO_TEST_FCM_KEY").unwrap();
    let form = reqwest::multipart::Form::new().text("api_key", api_key);

    let response_fcm_update = client
        .post(format!(
            "http://{}/tenants/{}/fcm",
            ctx.server.public_addr, tenant_id
        ))
        .bearer_auth(&jwt_token)
        .multipart(form)
        .send()
        .await
        .expect("Call failed");
    assert_eq!(response_fcm_update.status(), reqwest::StatusCode::OK);

    // Get tenant
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
    println!("response: {response:?}");
    assert!(response
        .enabled_providers
        .contains(&PROVIDER_FCM.to_owned()));
}

#[test_context(EchoServerContext)]
#[tokio::test]
async fn tenant_delete(ctx: &mut EchoServerContext) {
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

    // Send valid API Key
    let api_key = env::var("ECHO_TEST_FCM_KEY").unwrap();
    let form = reqwest::multipart::Form::new().text("api_key", api_key);

    let response_fcm_update = client
        .post(format!(
            "http://{}/tenants/{}/fcm",
            ctx.server.public_addr, tenant_id
        ))
        .bearer_auth(&jwt_token)
        .multipart(form)
        .send()
        .await
        .expect("Call failed");
    assert_eq!(response_fcm_update.status(), reqwest::StatusCode::OK);

    // Get tenant
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
    println!("response: {response:?}");
    assert!(response
        .enabled_providers
        .contains(&PROVIDER_FCM.to_owned()));

    let response_fcm_delete = client
        .delete(format!(
            "http://{}/tenants/{}/fcm",
            ctx.server.public_addr, tenant_id
        ))
        .bearer_auth(&jwt_token)
        .send()
        .await
        .expect("Call failed");
    assert_eq!(
        response_fcm_delete.status(),
        reqwest::StatusCode::NO_CONTENT
    );

    // Get tenant
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
    println!("response: {response:?}");
    assert!(!response
        .enabled_providers
        .contains(&PROVIDER_FCM.to_owned()));
}

/// Google decommissioned the legacy FCM HTTP API in June 2024: a dry-run send to
/// https://fcm.googleapis.com/fcm/send now answers 404, never 401, so update_fcm's
/// check can no longer tell a bad legacy key from a good one and accepts both (see
/// the comment in src/handlers/update_fcm.rs). Nothing in this repo can make this
/// assertion hold while legacy FCM is still offered; un-ignore it if legacy key
/// validation is restored, or delete it with the legacy provider.
#[ignore = "legacy FCM HTTP API is decommissioned; bad keys are no longer detectable"]
#[test_context(EchoServerContext)]
#[tokio::test]
async fn tenant_update_fcm_bad(ctx: &mut EchoServerContext) {
    let (tenant_id, jwt_token) = generate_random_tenant_id(&ctx.config.jwt_secret);

    // Register tenant
    let client = reqwest::Client::new();
    client
        .post(format!("http://{}/tenants", ctx.server.public_addr))
        .bearer_auth(&jwt_token)
        .json(&TenantRegisterBody {
            id: tenant_id.clone(),
        })
        .send()
        .await
        .expect("Call failed");

    // Send invalid API Key
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

    assert!(!response.status().is_success(), "Response was successful");
}
