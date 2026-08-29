use std::collections::HashMap;

use serde_json::Value;
use zbus::{fdo, interface};

const DBUS_NAME: &str = "dev.edfloreshz.Accounts.Provider.Microsoft";
const OBJECT_PATH: &str = "/dev/edfloreshz/Accounts/Provider";
const USER_INFO_URL: &str = "https://graph.microsoft.com/v1.0/me";

struct MicrosoftProvider;

#[interface(name = "dev.edfloreshz.Accounts.Provider1")]
impl MicrosoftProvider {
    async fn get_user_info(&self, access_token: &str) -> fdo::Result<HashMap<String, String>> {
        let client = reqwest::Client::new();

        let response = client
            .get(USER_INFO_URL)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| fdo::Error::Failed(format!("Request to Microsoft Graph failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(fdo::Error::Failed(format!(
                "Microsoft Graph /me request failed: {status} - {body}"
            )));
        }

        let data: Value = response.json().await.map_err(|e| {
            fdo::Error::Failed(format!("Invalid response from Microsoft Graph: {e}"))
        })?;

        let mut info = HashMap::new();
        info.insert(
            "display_name".to_string(),
            data["displayName"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string(),
        );

        let principal = data["userPrincipalName"]
            .as_str()
            .unwrap_or("Unknown")
            .to_string();
        info.insert("username".to_string(), principal.clone());

        let email = data["mail"]
            .as_str()
            .or(data["userPrincipalName"].as_str())
            .map(|s| s.to_string());
        if let Some(email) = email {
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
                    "https://outlook.office365.com/".to_string(),
                );
                config.insert("accept_ssl_errors".to_string(), "false".to_string());
            }
            "todo" => {
                config.insert(
                    "uri".to_string(),
                    "https://graph.microsoft.com/v1.0/me/todo".to_string(),
                );
                config.insert("accept_ssl_errors".to_string(), "false".to_string());
            }
            "contacts" => {
                config.insert(
                    "uri".to_string(),
                    "https://outlook.office365.com/".to_string(),
                );
                config.insert("accept_ssl_errors".to_string(), "false".to_string());
            }
            "email" => {
                config.insert("imap_host".to_string(), "outlook.office365.com".to_string());
                config.insert("imap_port".to_string(), "993".to_string());
                config.insert("smtp_port".to_string(), "587".to_string());
                config.insert("imap_supported".to_string(), "true".to_string());
                config.insert("imap_use_ssl".to_string(), "true".to_string());
                config.insert("imap_use_tls".to_string(), "false".to_string());
                config.insert("imap_accept_ssl_errors".to_string(), "false".to_string());

                config.insert("smtp_host".to_string(), "smtp.office365.com".to_string());
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
                    "Microsoft provider does not support service: {other}"
                )));
            }
        }
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Microsoft provider on {DBUS_NAME}");

    let _connection = zbus::connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(OBJECT_PATH, MicrosoftProvider)?
        .build()
        .await?;

    tracing::info!("Microsoft provider ready");
    std::future::pending::<()>().await;
    Ok(())
}
