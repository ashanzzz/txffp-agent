use crate::credential::{CredentialProvider, Credentials};
use crate::mcp::handle_mcp_request;
use crate::service::{ServiceError, TxffpService};
use crate::txffp::*;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

#[derive(Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub human_action: Option<serde_json::Value>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Json<Self> {
        Json(Self {
            ok: true,
            data: Some(data),
            error: None,
        })
    }

    pub fn fail(code: impl Into<String>, message: impl Into<String>) -> Json<Self> {
        Json(Self {
            ok: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                human_action: None,
            }),
        })
    }

    pub fn fail_with_action(
        code: impl Into<String>,
        message: impl Into<String>,
        action: serde_json::Value,
    ) -> Json<Self> {
        Json(Self {
            ok: false,
            data: None,
            error: Some(ApiError {
                code: code.into(),
                message: message.into(),
                human_action: Some(action),
            }),
        })
    }
}

pub fn create_router(service: Arc<TxffpService>) -> Router {
    Router::new()
        // Top-level health and human action page
        .route("/health", get(health_check))
        .route("/human/{token}", get(render_human_page))
        // MCP Streamable HTTP endpoint
        .route(
            "/mcp",
            axum::routing::post(handle_mcp_request).get(handle_mcp_request),
        )
        // API v1 routes
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/auth/status", get(get_auth_status))
        .route("/api/v1/auth/ensure", axum::routing::post(ensure_auth))
        .route("/api/v1/auth/check", axum::routing::post(check_auth))
        .route("/api/v1/cards", get(get_cards))
        .route(
            "/api/v1/invoices/preview",
            axum::routing::post(preview_invoice),
        )
        .route(
            "/api/v1/invoices",
            axum::routing::post(create_invoice).get(list_invoices),
        )
        .route("/api/v1/operations/{id}", get(get_operation))
        .route(
            "/api/v1/human-actions/{id}/complete",
            axum::routing::post(complete_human_action),
        )
        .route(
            "/api/v1/human-actions/{id}/cancel",
            axum::routing::post(cancel_human_action),
        )
        .route("/api/v1/settings", get(get_settings))
        .route(
            "/api/v1/settings/credentials",
            axum::routing::put(update_credentials).delete(delete_credentials),
        )
        .with_state(service)
}

// Handlers

async fn health_check(State(service): State<Arc<TxffpService>>) -> impl IntoResponse {
    let steel_ok = service.browser.check_health().await;
    ApiResponse::success(serde_json::json!({
        "status": "ok",
        "service": "txffp-server",
        "version": env!("CARGO_PKG_VERSION"),
        "steel_connected": steel_ok,
        "database": "sqlite_connected"
    }))
}

async fn render_human_page(
    Path(token): Path<String>,
    State(service): State<Arc<TxffpService>>,
) -> impl IntoResponse {
    match service.human_action.get_by_token(&token).await {
        Ok(Some(action)) => {
            let html = service.human_action.render_html_page(&action);
            Html(html).into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Html("<h1>404 Not Found</h1><p>未找到对应的人工处理通道或该任务已失效。</p>"),
        )
            .into_response(),
    }
}

async fn get_auth_status(State(service): State<Arc<TxffpService>>) -> impl IntoResponse {
    let state = service.auth_status().await;
    ApiResponse::success(state)
}

async fn ensure_auth(State(service): State<Arc<TxffpService>>) -> impl IntoResponse {
    match service.ensure_auth().await {
        Ok(state) => ApiResponse::success(serde_json::json!({
            "status": state.status,
            "message": state.message,
            "session_active": state.session.is_some()
        })),
        Err(ServiceError::CredentialsRequired) => ApiResponse::fail(
            "CREDENTIALS_REQUIRED",
            "尚未配置票根账号和密码，请在设置中配置",
        ),
        Err(ServiceError::HumanActionRequired(action_id)) => {
            let action_url = format!("/human/{}", action_id);
            ApiResponse::fail_with_action(
                "HUMAN_ACTION_REQUIRED",
                "需要用户介入完成安全验证或登录操作",
                serde_json::json!({
                    "action_id": action_id,
                    "url": action_url
                }),
            )
        }
        Err(e) => ApiResponse::fail("AUTH_ERROR", e.to_string()),
    }
}

async fn check_auth(
    State(service): State<Arc<TxffpService>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let action_id = payload
        .get("action_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    match service.check_auth_after_action(action_id).await {
        Ok(state) => ApiResponse::success(state),
        Err(e) => ApiResponse::fail("AUTH_VERIFICATION_FAILED", e.to_string()),
    }
}

async fn get_cards(State(service): State<Arc<TxffpService>>) -> impl IntoResponse {
    match service.list_cards().await {
        Ok(cards) => ApiResponse::success(cards),
        Err(e) => map_service_error(e),
    }
}

async fn preview_invoice(
    State(service): State<Arc<TxffpService>>,
    Json(req): Json<InvoicePreviewRequest>,
) -> impl IntoResponse {
    match service.preview_invoice(req).await {
        Ok(preview) => ApiResponse::success(preview),
        Err(e) => map_service_error(e),
    }
}

async fn create_invoice(
    State(service): State<Arc<TxffpService>>,
    Json(req): Json<CreateInvoiceRequest>,
) -> impl IntoResponse {
    match service.create_invoice(req).await {
        Ok(resp) => ApiResponse::success(resp),
        Err(e) => map_service_error(e),
    }
}

async fn list_invoices(State(service): State<Arc<TxffpService>>) -> impl IntoResponse {
    match service.list_invoices().await {
        Ok(list) => ApiResponse::success(list),
        Err(e) => map_service_error(e),
    }
}

async fn get_operation(
    Path(id): Path<String>,
    State(service): State<Arc<TxffpService>>,
) -> impl IntoResponse {
    let row = sqlx::query(
        "SELECT id, type, phase, parameters, result, error, created_at, updated_at FROM operations WHERE id = ?1",
    )
    .bind(&id)
    .fetch_optional(&service.pool)
    .await;

    match row {
        Ok(Some(r)) => {
            let id: String = r.get("id");
            let op_type: String = r.get("type");
            let phase: String = r.get("phase");
            let params_str: String = r.get("parameters");
            let result_str: Option<String> = r.get("result");
            let error_str: Option<String> = r.get("error");
            let created_at: String = r.get("created_at");
            let updated_at: String = r.get("updated_at");

            ApiResponse::success(serde_json::json!({
                "id": id,
                "type": op_type,
                "phase": phase,
                "parameters": serde_json::from_str::<serde_json::Value>(&params_str).ok(),
                "result": result_str.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()),
                "error": error_str,
                "created_at": created_at,
                "updated_at": updated_at
            }))
        }
        Ok(None) => ApiResponse::fail("NOT_FOUND", "未找到指定的操作记录"),
        Err(e) => ApiResponse::fail("DB_ERROR", e.to_string()),
    }
}

async fn complete_human_action(
    Path(id): Path<String>,
    State(service): State<Arc<TxffpService>>,
) -> impl IntoResponse {
    match service.check_auth_after_action(&id).await {
        Ok(_) => ApiResponse::success(serde_json::json!({ "completed": true })),
        Err(e) => ApiResponse::fail("COMPLETE_FAILED", e.to_string()),
    }
}

async fn cancel_human_action(
    Path(id): Path<String>,
    State(service): State<Arc<TxffpService>>,
) -> impl IntoResponse {
    match service.human_action.cancel_action(&id).await {
        Ok(ok) => ApiResponse::success(serde_json::json!({ "cancelled": ok })),
        Err(e) => ApiResponse::fail("CANCEL_FAILED", e.to_string()),
    }
}

async fn get_settings(State(service): State<Arc<TxffpService>>) -> impl IntoResponse {
    let cred_status = service.credentials.get_status().await;
    ApiResponse::success(serde_json::json!({
        "credentials": cred_status,
        "auth_mode": service.config.auth_mode,
        "auto_login": service.config.auto_login,
        "max_auth_attempts": service.config.max_auth_attempts,
        "steel_base_url": service.config.steel_base_url,
        "research_mode": service.config.research_mode,
    }))
}

#[derive(Deserialize)]
struct UpdateCredentialsPayload {
    pub username: String,
    pub password: String,
    pub phone: Option<String>,
}

async fn update_credentials(
    State(service): State<Arc<TxffpService>>,
    Json(payload): Json<UpdateCredentialsPayload>,
) -> impl IntoResponse {
    let creds = Credentials {
        username: payload.username,
        password: payload.password,
        phone: payload.phone,
    };

    match service.credentials.update_credentials(creds).await {
        Ok(_) => ApiResponse::success(serde_json::json!({ "updated": true })),
        Err(e) => ApiResponse::fail("UPDATE_CREDENTIALS_FAILED", e.to_string()),
    }
}

async fn delete_credentials(State(service): State<Arc<TxffpService>>) -> impl IntoResponse {
    match service.credentials.delete_credentials().await {
        Ok(_) => ApiResponse::success(serde_json::json!({ "deleted": true })),
        Err(e) => ApiResponse::fail("DELETE_CREDENTIALS_FAILED", e.to_string()),
    }
}

fn map_service_error<T: Serialize>(err: ServiceError) -> Json<ApiResponse<T>> {
    match err {
        ServiceError::CredentialsRequired => {
            ApiResponse::fail("CREDENTIALS_REQUIRED", "请先配置票根账号密码")
        }
        ServiceError::HumanActionRequired(action_id) => ApiResponse::fail_with_action(
            "HUMAN_ACTION_REQUIRED",
            "需要人工操作",
            serde_json::json!({ "action_id": action_id }),
        ),
        ServiceError::DuplicateSubmission => {
            ApiResponse::fail("INVOICE_ALREADY_SUBMITTED", "该开票任务已在处理或已完成")
        }
        ServiceError::AuthRequired(msg) => ApiResponse::fail("LOGIN_REQUIRED", msg),
        ServiceError::InvalidRequest(msg) => ApiResponse::fail("INVALID_REQUEST", msg),
        ServiceError::Browser(msg) => ApiResponse::fail("STEEL_UNAVAILABLE", msg),
        ServiceError::Remote(msg) => ApiResponse::fail("TXFFP_UNAVAILABLE", msg),
        ServiceError::Database(e) => ApiResponse::fail("DB_ERROR", e.to_string()),
    }
}
