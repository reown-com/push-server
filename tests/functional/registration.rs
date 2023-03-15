use {
    crate::context::SingleTenantServerContext,
    echo_server::handlers::register_client::RegisterBody,
    random_string::generate,
    relay_rpc::{
        auth::{
            ed25519_dalek::Keypair,
            rand::{rngs::StdRng, SeedableRng},
        },
        domain::ClientId,
    },
    std::sync::Arc,
    test_context::test_context,
};

#[test_context(SingleTenantServerContext)]
#[tokio::test]
async fn test_registration(ctx: &mut SingleTenantServerContext) {
    let charset = "1234567890";
    let random_client_id = ClientId::new(Arc::from(generate(12, charset)));
    let payload = RegisterBody {
        client_id: random_client_id.clone(),
        push_type: "noop".to_string(),
        token: "test".to_string(),
    };

    let seed: [u8; 32] = "THIS_IS_TEST_VALUE_SHOULD_NOT_BE_USED_IN_PROD"
        .to_string()
        .as_bytes()[..32]
        .try_into()
        .unwrap();
    let mut seeded = StdRng::from_seed(seed);
    let keypair = Keypair::generate(&mut seeded);

    dbg!(ctx.server.public_addr.to_string());

    let jwt = relay_rpc::auth::AuthToken::new(random_client_id.value().clone())
        .aud(format!("127.0.0.1:{}", ctx.server.public_addr.port()))
        .as_jwt(&keypair)
        .unwrap()
        .to_string();

    // Register client
    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/clients", ctx.server.public_addr))
        .header("Authorization", jwt)
        .json(&payload)
        .send()
        .await;
    dbg!(&response);
    let response = response.expect("Call failed");

    assert!(
        response.status().is_success(),
        "Response was not successful"
    );

    // Update token
    let payload = RegisterBody {
        client_id: random_client_id,
        push_type: "noop".to_string(),
        token: "new_token".to_string(),
    };
    let response = client
        .post(format!("http://{}/clients", ctx.server.public_addr))
        .json(&payload)
        .send()
        .await
        .expect("Call failed");

    assert!(
        response.status().is_success(),
        "Response was not successful"
    );
}

#[test_context(SingleTenantServerContext)]
#[tokio::test]
async fn test_deregistration(ctx: &mut SingleTenantServerContext) {
    let charset = "1234567890";
    let random_client_id = ClientId::new(Arc::from(generate(12, charset)));
    let payload = RegisterBody {
        client_id: random_client_id.clone(),
        push_type: "noop".to_string(),
        token: "test".to_string(),
    };

    let client = reqwest::Client::new();
    let register_response = client
        .post(format!("http://{}/clients", ctx.server.public_addr))
        .json(&payload)
        .send()
        .await
        .expect("Call failed");

    assert!(
        register_response.status().is_success(),
        "Failed to register client"
    );

    let client = reqwest::Client::new();
    let delete_response = client
        .delete(format!(
            "http://{}/clients/{}",
            ctx.server.public_addr, random_client_id
        ))
        .send()
        .await
        .expect("Call failed")
        .status();

    assert!(delete_response.is_success(), "Failed to unregister client");
}
