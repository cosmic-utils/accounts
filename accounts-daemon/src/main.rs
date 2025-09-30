use crate::{account::AccountsInterface, services::ServiceFactory};
use accounts::{AccountsClient, models::Account};
use axum::{Router, extract::Query, http::StatusCode, response::Html, routing::get};
use serde::Deserialize;
use tokio::sync::OnceCell;
use tracing::info;
use tracing_subscriber;

mod account;
mod auth;
mod error;
mod models;
mod services;
mod storage;

pub use error::{Error, Result};
use zbus::Connection;

pub static CONNECTION: OnceCell<Connection> = OnceCell::const_new();

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Accounts for COSMIC daemon with integrated HTTP server...");

    let router = Router::new().route("/callback", get(handle_callback));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .map_err(|e| Error::Io(e))?;

    info!("HTTP server will listen on http://127.0.0.1:8080");
    info!("OAuth callback URL: http://127.0.0.1:8080/callback");

    info!("Setting up D-Bus connection...");
    let service = AccountsInterface::new()
        .await
        .map_err(|e| zbus::Error::Failure(e.to_string()))?;

    let accounts: Vec<Account> = service
        .list_accounts()
        .await
        .into_iter()
        .map(Into::into)
        .collect();

    CONNECTION
        .set(
            zbus::connection::Builder::session()?
                .name("dev.edfloreshz.Accounts")?
                .serve_at("/dev/edfloreshz/Accounts/Account", service)?
                .build()
                .await?,
        )
        .unwrap();

    for account in accounts {
        let services = ServiceFactory::create_services(&account);
        for service in services {
            service.add_service().await?;
        }
    }

    info!("D-Bus service started on: dev.edfloreshz.Accounts");
    info!("Object path: /dev/edfloreshz/Accounts");

    info!("Accounts for COSMIC daemon started successfully");

    axum::serve(listener, router).await.unwrap();

    Ok(())
}

async fn handle_callback(Query(params): Query<CallbackQuery>) -> (StatusCode, Html<String>) {
    info!("Received OAuth callback: {:?}", params);

    let Ok(mut client) = AccountsClient::new().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("Accounts for COSMIC Client failed to initialize".to_string()),
        );
    };

    if let Some(error) = &params.error {
        let html = format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Authentication Error</title>
                <style>
                    body {{ font-family: sans-serif; margin: 40px; text-align: center; }}
                    .error {{ color: #d73a49; background: #ffeef0; padding: 20px; border-radius: 8px; }}
                </style>
            </head>
            <body>
                <div class="error">
                    <h2>Authentication Failed</h2>
                    <p><strong>Error:</strong> {}</p>
                    <p><strong>Description:</strong> {}</p>
                    <p>You can close this window.</p>
                </div>
            </body>
            </html>
            "#,
            error,
            params
                .error_description
                .as_deref()
                .unwrap_or("No description")
        );
        (StatusCode::BAD_REQUEST, Html(html))
    } else if let (Some(authorization_code), Some(csrf_token)) = (params.code, params.state) {
        let account_id = match client
            .complete_authentication(&csrf_token, &authorization_code)
            .await
        {
            Ok(account_id) => {
                match client.account_added(&account_id).await {
                    Ok(_) => {
                        tracing::info!("Account added with ID: {}", account_id);
                    }
                    Err(err) => {
                        tracing::error!("Failed to add account: {}", err);
                    }
                }
                account_id
            }
            Err(_err) => {
                if matches!(Error::AccountAlreadyExists, _err) {
                    match client.account_exists().await {
                        Ok(_) => {
                            tracing::info!("Account already exists");
                        }
                        Err(err) => {
                            tracing::error!("Failed to check account existence: {}", err);
                        }
                    }
                }
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html(format!("Failed to authenticate user: {}", _err)),
                );
            }
        };

        tracing::info!("User authenticated with ID: {}", account_id);

        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Authentication Success</title>
                <style>
                    body { font-family: sans-serif; margin: 40px; text-align: center; }
                    .success { color: #28a745; background: #d4edda; padding: 20px; border-radius: 8px; }
                </style>
            </head>
            <body>
                <div class="success">
                    <h2>Authentication Successful!</h2>
                    <p>You can now close this window.</p>
                </div>
            </body>
            </html>
        "#;
        (StatusCode::OK, Html(html.to_string()))
    } else {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Invalid Callback</title>
                <style>
                    body { font-family: sans-serif; margin: 40px; text-align: center; }
                    .warning { color: #856404; background: #fff3cd; padding: 20px; border-radius: 8px; }
                </style>
            </head>
            <body>
                <div class="warning">
                    <h2>Invalid Callback</h2>
                    <p>Missing required parameters.</p>
                </div>
            </body>
            </html>
        "#;
        (StatusCode::BAD_REQUEST, Html(html.to_string()))
    }
}
