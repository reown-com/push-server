use {
    super::{LegacyPushMessage, PushMessage, RawPushMessage},
    crate::{blob::DecryptedPayloadBlob, error::Error, providers::PushProvider},
    a2::{ClientConfig, ErrorReason, NotificationBuilder, NotificationOptions},
    async_trait::async_trait,
    std::io::Read,
    tracing::{debug, info, instrument, warn},
};

#[derive(Debug, Clone)]
pub struct ApnsProvider {
    client: a2::Client,
    topic: String,
}

impl ApnsProvider {
    pub fn new_cert<R>(
        cert: &mut R,
        password: String,
        endpoint: a2::Endpoint,
        topic: String,
    ) -> crate::error::Result<Self>
    where
        R: Read,
    {
        Ok(ApnsProvider {
            client: a2::Client::certificate(cert, password.as_str(), ClientConfig::new(endpoint))?,
            topic,
        })
    }

    pub fn new_token<R>(
        pkcs8_pem: &mut R,
        key_id: String,
        team_id: String,
        endpoint: a2::Endpoint,
        topic: String,
    ) -> crate::error::Result<Self>
    where
        R: Read,
    {
        Ok(ApnsProvider {
            client: a2::Client::token(pkcs8_pem, key_id, team_id, ClientConfig::new(endpoint))?,
            topic,
        })
    }
}

#[async_trait]
impl PushProvider for ApnsProvider {
    #[instrument(name = "send_apns_notification")]
    async fn send_notification(
        &self,
        token: String,
        body: PushMessage,
    ) -> crate::error::Result<()> {
        let opt = NotificationOptions {
            apns_id: None,
            apns_expiration: None,
            apns_priority: None,
            apns_topic: Some(&self.topic),
            apns_collapse_id: None,
            apns_push_type: None,
        };

        let result = match body {
            PushMessage::RawPushMessage(RawPushMessage {
                topic,
                tag,
                message,
            }) => {
                // Sending `always_raw` encrypted message
                debug!("Sending raw encrypted message");
                let mut notification_payload = a2::DefaultNotificationBuilder::new()
                    .set_content_available()
                    .set_mutable_content()
                    .set_title("You have new notifications. Open to view")
                    .build(token.as_str(), opt);

                notification_payload.add_custom_data("topic", &topic)?;
                notification_payload.add_custom_data("tag", &tag)?;
                notification_payload.add_custom_data("message", &message)?;

                self.client.send(notification_payload).await
            }
            PushMessage::LegacyPushMessage(LegacyPushMessage { id: _, payload }) => {
                // TODO tidy after https://github.com/WalletConnect/a2/issues/67 is closed
                if payload.is_encrypted() {
                    debug!("Sending legacy `is_encrypted` message");
                    let mut notification_payload = a2::DefaultNotificationBuilder::new()
                        .set_content_available()
                        .set_mutable_content()
                        .set_title("You have new notifications. Open to view")
                        .build(token.as_str(), opt);

                    notification_payload.add_custom_data("topic", &payload.topic)?;
                    notification_payload.add_custom_data("blob", &payload.blob)?;

                    self.client.send(notification_payload).await
                } else {
                    debug!("Sending plain message");
                    let blob = DecryptedPayloadBlob::from_base64_encoded(&payload.blob)?;

                    let mut notification_payload = a2::DefaultNotificationBuilder::new()
                        .set_content_available()
                        .set_mutable_content()
                        .set_title(&blob.title)
                        .set_body(&blob.body)
                        .build(token.as_str(), opt);

                    notification_payload.add_custom_data("topic", &payload.topic)?;

                    self.client.send(notification_payload).await
                }
            }
        };

        match result {
            Ok(response) => {
                if response.error.is_some() {
                    warn!(
                        "Unexpected APNS error. a2 lib shouldn't allow returning Ok containing \
                         error response. Status: {} Error: {:?}",
                        response.code, response.error
                    );
                    Err(Error::Apns(a2::Error::ResponseError(response)))
                } else {
                    Ok(())
                }
            }
            Err(e) => match e {
                a2::Error::ResponseError(res) => match res.error {
                    None => Err(Error::Apns(a2::Error::ResponseError(res))),
                    Some(response) => match response.reason {
                        ErrorReason::BadDeviceToken => {
                            Err(Error::BadDeviceToken("Bad device token".to_string()))
                        }
                        // Note: This will have the device deleted because the token was not for the
                        // configured topic
                        ErrorReason::DeviceTokenNotForTopic => Err(Error::BadDeviceToken(
                            "The device token does not match the specified topic".to_string(),
                        )),
                        ErrorReason::Unregistered => Err(Error::BadDeviceToken(
                            "The device token is inactive for the specified topic".to_string(),
                        )),
                        ErrorReason::TopicDisallowed => Err(Error::BadApnsCredentials),
                        // InvalidProviderToken reflecting that APNS certificate must be reissued
                        ErrorReason::InvalidProviderToken => Err(Error::ApnsInvalidProviderToken),
                        ErrorReason::PayloadTooLarge => Err(Error::PayloadTooLarge),
                        reason => Err(Error::ApnsResponse(reason)),
                    },
                },
                a2::Error::ConnectionError(ref hyper_error) => {
                    let dbg = format!("{hyper_error:?}");
                    // e.g. Apns(ConnectionError(hyper::Error(Io, Custom { kind: InvalidData, error: "received fatal alert: CertificateExpired" })))
                    // Checking if debug fmt contains something is strange.
                    // Logging stuff here temporarily so we can determine better
                    // ways to detect this error (e.g. display). Ideally we can extract
                    // the error field directly and check if exactly equal to the above
                    // rather than using contains()
                    match dbg {
                        dbg if dbg.contains("received fatal alert: CertificateExpired") => {
                            info!("APNs certificate expired: debug:{dbg}, display: {hyper_error}");
                            Err(Error::ApnsCertificateExpired)
                        }
                        _ => Err(Error::Apns(e)),
                    }
                }
                a2::Error::ClientError(ref client_error) => {
                    let dbg = format!("{client_error:?}");
                    match dbg {
                        dbg if dbg.contains("received fatal alert: UnknownCA") => {
                            info!(
                                "APNs certificate unknown CA: debug:{dbg}, display: {client_error}"
                            );
                            Err(Error::ApnsCertificateUnknownCA)
                        }
                        dbg if dbg.contains("received fatal alert: CertificateExpired") => {
                            info!("APNs certificate expired: debug:{dbg}, display: {client_error}");
                            Err(Error::ApnsCertificateExpired)
                        }
                        _ => Err(Error::Apns(e)),
                    }
                }
                e => Err(Error::Apns(e)),
            },
        }
    }
}
