use std::collections::HashMap;

use serde_json::Value;
use zbus::{fdo, interface};

const DBUS_NAME: &str = "dev.edfloreshz.Accounts.Provider.Google";
const OBJECT_PATH: &str = "/dev/edfloreshz/Accounts/Provider";
const USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

struct GoogleProvider;

#[interface(name = "dev.edfloreshz.Accounts.Provider1")]
impl GoogleProvider {
    async fn get_user_info(&self, access_token: &str) -> fdo::Result<HashMap<String, String>> {
        let client = reqwest::Client::new();

        let response = client
            .get(USER_INFO_URL)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| fdo::Error::Failed(format!("Request to Google failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(fdo::Error::Failed(format!(
                "Google userinfo request failed: {status} - {body}"
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| fdo::Error::Failed(format!("Invalid response from Google: {e}")))?;

        let mut info = HashMap::new();
        info.insert(
            "display_name".to_string(),
            data["name"].as_str().unwrap_or("Unknown").to_string(),
        );
        let email = data["email"].as_str().unwrap_or("Unknown").to_string();
        info.insert("username".to_string(), email.clone());
        if data["email"].as_str().is_some() {
            info.insert("email".to_string(), email);
        }

        Ok(info)
    }

    async fn get_service_config(&self, service: &str) -> fdo::Result<HashMap<String, String>> {
        let mut config = HashMap::new();
        match service.to_lowercase().as_str() {
            "calendar" => {
                config.insert(
                    "uri".to_string(),
                    "https://apidata.googleusercontent.com/caldav/v2/".to_string(),
                );
                config.insert("accept_ssl_errors".to_string(), "false".to_string());
            }
            "todo" => {
                config.insert(
                    "uri".to_string(),
                    "https://tasks.googleapis.com/tasks/v1/".to_string(),
                );
                config.insert("accept_ssl_errors".to_string(), "false".to_string());
            }
            "email" => {
                config.insert("imap_host".to_string(), "imap.gmail.com".to_string());
                config.insert("imap_supported".to_string(), "true".to_string());
                config.insert("imap_use_ssl".to_string(), "true".to_string());
                config.insert("imap_use_tls".to_string(), "false".to_string());
                config.insert("imap_accept_ssl_errors".to_string(), "false".to_string());

                config.insert("smtp_host".to_string(), "smtp.gmail.com".to_string());
                config.insert("smtp_supported".to_string(), "true".to_string());
                config.insert("smtp_use_auth".to_string(), "true".to_string());
                config.insert("smtp_use_ssl".to_string(), "false".to_string());
                config.insert("smtp_use_tls".to_string(), "true".to_string());
                config.insert("smtp_accept_ssl_errors".to_string(), "false".to_string());
                config.insert("smtp_auth_login".to_string(), "false".to_string());
                config.insert("smtp_auth_plain".to_string(), "false".to_string());
                config.insert("smtp_auth_xoauth2".to_string(), "true".to_string());
            }
            other => {
                return Err(fdo::Error::Failed(format!(
                    "Google provider does not support service: {other}"
                )));
            }
        }
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Google provider on {DBUS_NAME}");

    let _connection = zbus::connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(OBJECT_PATH, GoogleProvider)?
        .build()
        .await?;

    tracing::info!("Google provider ready");
    std::future::pending::<()>().await;
    Ok(())
}
