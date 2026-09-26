//! HTTP surface for the local desk.
//!
//! There is no order route. Webhook JSON is stored and nothing is forwarded
//! to a broker.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use strikedesk_core::rulebook;
use strikedesk_screener::{load_config, scan, Config};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

const ALERT_CAP: usize = 200;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub repo_root: PathBuf,
    pub webhook_token: Option<String>,
    pub alerts: Arc<Mutex<VecDeque<StoredAlert>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredAlert {
    pub received_at: String,
    pub symbol: String,
    pub badge: Option<String>,
    pub strike: Option<f64>,
    pub price: Option<f64>,
    pub time: Option<String>,
    pub message: Option<String>,
}

pub fn load_state(config_path: &Path) -> Result<AppState, strikedesk_screener::ScanError> {
    let config = load_config(config_path)?;
    let absolute = if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(config_path)
    };
    let repo_root = absolute
        .parent()
        .and_then(|dir| dir.parent())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let webhook_token = std::env::var("STRIKEDESK_WEBHOOK_TOKEN")
        .ok()
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty());
    Ok(AppState {
        config,
        repo_root,
        webhook_token,
        alerts: Arc::new(Mutex::new(VecDeque::new())),
    })
}

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/scan", get(scan_handler))
        .route("/rules", get(rules))
        .route("/tv/pine/:name", get(pine))
        .route("/tv/webhook", post(webhook))
        .route("/tv/alerts", get(list_alerts))
        .fallback(unknown_api)
        .with_state(state);
    let index = PathBuf::from("web/build/index.html");
    let mut app = Router::new()
        .nest("/api", api)
        .layer(CorsLayer::permissive());
    if index.exists() {
        let files = ServeDir::new("web/build").not_found_service(ServeFile::new(index));
        app = app.fallback_service(files);
    } else {
        app = app.fallback(get(help));
    }
    app
}

async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "strikedesk",
        "orders_enabled": false
    }))
}

#[derive(Deserialize)]
struct ScanQuery {
    source: Option<String>,
    preset: Option<String>,
}

async fn scan_handler(State(state): State<AppState>, Query(query): Query<ScanQuery>) -> Response {
    let source = query
        .source
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| state.config.data.source.clone());
    let preset = query
        .preset
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "qullamaggie".to_string());
    match scan(&state.config, &source, &preset).await {
        Ok(report) => Json(report).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": err.to_string(), "orders_enabled": false})),
        )
            .into_response(),
    }
}

async fn rules(State(state): State<AppState>) -> Json<Vec<strikedesk_core::RuleDoc>> {
    Json(rulebook(&state.config.badges))
}

async fn pine(State(state): State<AppState>, AxumPath(name): AxumPath<String>) -> Response {
    let file = match name.as_str() {
        "badges" => "tradingview/strikedesk_badges.pine",
        "strike" => "tradingview/strikedesk_strike.pine",
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "unknown pine script"})),
            )
                .into_response()
        }
    };
    let path = state.repo_root.join(file);
    match std::fs::read_to_string(&path) {
        Ok(text) => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": format!("could not read {}: {err}", path.display())})),
        )
            .into_response(),
    }
}

async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    if let Some(expected) = &state.webhook_token {
        let presented = headers
            .get("x-strikedesk-token")
            .and_then(|value| value.to_str().ok());
        if presented != Some(expected.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "webhook token mismatch", "orders_enabled": false})),
            )
                .into_response();
        }
    }
    let Some(symbol) = body.get("symbol").and_then(Value::as_str) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "symbol is required", "orders_enabled": false})),
        )
            .into_response();
    };
    let symbol = symbol.trim().to_ascii_uppercase();
    if symbol.is_empty()
        || symbol.len() > 16
        || !symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "symbol is invalid", "orders_enabled": false})),
        )
            .into_response();
    }
    let alert = StoredAlert {
        received_at: Utc::now().to_rfc3339(),
        symbol,
        badge: body
            .get("badge")
            .and_then(Value::as_str)
            .map(str::to_string),
        strike: body.get("strike").and_then(Value::as_f64),
        price: body.get("price").and_then(Value::as_f64),
        time: body.get("time").and_then(Value::as_str).map(str::to_string),
        message: body
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    append_alert(&state, &alert);
    {
        let mut alerts = state.alerts.lock().await;
        alerts.push_back(alert);
        while alerts.len() > ALERT_CAP {
            alerts.pop_front();
        }
    }
    Json(json!({"ok": true, "stored": true, "orders_enabled": false})).into_response()
}

fn append_alert(state: &AppState, alert: &StoredAlert) {
    let path = PathBuf::from(&state.config.data.alerts_path);
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            eprintln!("strikedesk alert log skipped: {err}");
            return;
        }
    }
    let line = match serde_json::to_string(alert) {
        Ok(line) => line,
        Err(err) => {
            eprintln!("strikedesk alert log skipped: {err}");
            return;
        }
    };
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            if let Err(err) = writeln!(file, "{line}") {
                eprintln!("strikedesk alert log skipped: {err}");
            }
        }
        Err(err) => eprintln!("strikedesk alert log skipped: {err}"),
    }
}

async fn list_alerts(State(state): State<AppState>) -> Json<Value> {
    let alerts = state.alerts.lock().await;
    let rows: Vec<_> = alerts.iter().rev().cloned().collect();
    Json(json!({
        "orders_enabled": false,
        "alerts": rows
    }))
}

async fn unknown_api() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error": "not found", "orders_enabled": false})),
    )
        .into_response()
}

async fn help() -> Html<&'static str> {
    Html(
        "<!doctype html><meta charset=utf-8><title>Strikedesk</title>\
         <body style='font:16px/1.5 sans-serif;background:#070b14;color:#e8eefc;padding:32px'>\
         <h1>Strikedesk API</h1>\
         <p>The desk UI is not built yet. From the repo: <code>cd web && npm install && npm run dev</code>.</p>\
         <p>Badges choose what to chart. This process does not place broker orders.</p>\
         <p><a style='color:#ffe14a' href='/api/health'>/api/health</a> · \
         <a style='color:#ffe14a' href='/api/scan'>/api/scan</a> · \
         <a style='color:#ffe14a' href='/api/rules'>/api/rules</a></p>",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn state() -> AppState {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/screener.toml");
        load_state(&path).unwrap()
    }

    async fn json_body(response: Response) -> Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_refuses_orders() {
        let app = router(state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["orders_enabled"], false);
        assert_eq!(body["service"], "strikedesk");
    }

    #[tokio::test]
    async fn scan_is_ranked_and_has_no_order_route() {
        let app = router(state());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/scan?source=fixtures&preset=qullamaggie")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["orders_enabled"], false);
        assert!(body["rows"].as_array().unwrap().len() >= 15);
        let missing = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orders")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn webhook_stores_json_and_rejects_a_missing_symbol() {
        let app = router(state());
        let bad = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tv/webhook")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"badge":"KQ"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

        let ok = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tv/webhook")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"symbol":"qmco","badge":"KQ","strike":88,"price":18.4,"message":"desk test"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        let stored = json_body(ok).await;
        assert_eq!(stored["orders_enabled"], false);

        let listed = app
            .oneshot(
                Request::builder()
                    .uri("/api/tv/alerts")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let listed = json_body(listed).await;
        assert_eq!(listed["alerts"][0]["symbol"], "QMCO");
        assert_eq!(listed["orders_enabled"], false);
    }

    #[tokio::test]
    async fn pine_badges_are_served_as_text() {
        let app = router(state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tv/pine/badges")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(text.contains("//@version=5"));
        assert!(text.contains("KQ"));
    }
}
