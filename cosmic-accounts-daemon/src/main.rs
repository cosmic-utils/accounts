use axum::{extract::Query, http::StatusCode, response::Html, routing::get, Router};
use cosmic_accounts::{CosmicAccounts, Result};
use serde::Deserialize;
use tracing::info;
use tracing_subscriber;
use zbus::connection;

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    #[allow(unused)]
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

async fn handle_callback(Query(params): Query<CallbackQuery>) -> (StatusCode, Html<String>) {
    info!("Received OAuth callback: {:?}", params);

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
    } else if params.code.is_some() {
        // TODO: Process the OAuth code here
        // You can communicate with the D-Bus service or handle the token exchange
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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting COSMIC Accounts daemon with integrated HTTP server...");

    let mut accounts = CosmicAccounts::new()?;
    accounts.setup_providers().await?;

    let router = Router::new().route("/callback", get(handle_callback));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .map_err(|e| cosmic_accounts::AccountsError::Io(e))?;

    info!("HTTP server will listen on http://127.0.0.1:8080");
    info!("OAuth callback URL: http://127.0.0.1:8080/callback");

    info!("Setting up D-Bus connection...");
    let _conn = connection::Builder::session()?
        .name("com.system76.CosmicAccounts")?
        .serve_at("/com/system76/CosmicAccounts", accounts)?
        .build()
        .await?;

    info!("D-Bus service started on: com.system76.CosmicAccounts");
    info!("Object path: /com/system76/CosmicAccounts");

    info!("COSMIC Accounts daemon started successfully");

    axum::serve(listener, router).await.unwrap();

    Ok(())
}
