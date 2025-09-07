use axum::{Router, extract::Query, http::StatusCode, response::Html, routing::get};
use serde::Deserialize;
use tracing::info;

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
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/callback", get(handle_callback));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    info!("Server listening on http://127.0.0.1:8080");
    info!("Callback URL: http://127.0.0.1:8080/callback");

    axum::serve(listener, app).await.unwrap();
}
