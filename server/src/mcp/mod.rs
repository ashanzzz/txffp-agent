use crate::credential::CredentialProvider;
use crate::service::TxffpService;
use crate::txffp::*;
use axum::extract::State;
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub async fn handle_mcp_request(
    State(service): State<Arc<TxffpService>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let id = req.id.unwrap_or(Value::Null);

    let res = match req.method.as_str() {
        "tools/list" => {
            let tools = serde_json::json!({
                "tools": [
                    {
                        "name": "txffp_auth_status",
                        "description": "查询当前票根账号登录状态与凭据配置",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "txffp_auth_ensure",
                        "description": "确保当前票根处于已登录状态。若未登录则自动执行安全登录或返回人工操作通道",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "txffp_list_cards",
                        "description": "查询用户绑定的所有 ETC 卡列表",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    {
                        "name": "txffp_invoice_preview",
                        "description": "按日期范围预览待开票通行记录（只读操作，不产生真实发票）",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "start_date": { "type": "string", "description": "起始日期 YYYY-MM-DD" },
                                "end_date": { "type": "string", "description": "结束日期 YYYY-MM-DD" },
                                "card_id": { "type": "string", "description": "指定的 ETC 卡 ID（可选）" }
                            },
                            "required": ["start_date", "end_date"]
                        }
                    },
                    {
                        "name": "txffp_invoice_create",
                        "description": "正式提交开票申请（具有幂等保护）",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "start_date": { "type": "string", "description": "起始日期 YYYY-MM-DD" },
                                "end_date": { "type": "string", "description": "结束日期 YYYY-MM-DD" },
                                "idempotency_key": { "type": "string", "description": "唯一的幂等操作标识" },
                                "card_id": { "type": "string", "description": "指定的 ETC 卡 ID（可选）" }
                            },
                            "required": ["start_date", "end_date", "idempotency_key"]
                        }
                    },
                    {
                        "name": "txffp_invoice_list",
                        "description": "获取已开具的历史发票列表与下载链接",
                        "inputSchema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                ]
            });
            Ok(tools)
        }
        "tools/call" => {
            let params = req.params.unwrap_or_default();
            let tool_name = params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            execute_tool(service, tool_name, arguments).await
        }
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Method not found: {}", req.method),
            data: None,
        }),
    };

    match res {
        Ok(result) => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }),
        Err(err) => Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(err),
        }),
    }
}

async fn execute_tool(
    service: Arc<TxffpService>,
    name: &str,
    args: Value,
) -> Result<Value, JsonRpcError> {
    match name {
        "txffp_auth_status" => {
            let status = service.auth_status().await;
            let cred_status = service.credentials.get_status().await;
            Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&serde_json::json!({
                        "auth_state": status,
                        "credentials": cred_status
                    })).unwrap_or_default()
                }]
            }))
        }
        "txffp_auth_ensure" => match service.ensure_auth().await {
            Ok(state) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("认证有效：状态={:?}", state.status)
                }]
            })),
            Err(crate::service::ServiceError::HumanActionRequired(action_id)) => {
                Ok(serde_json::json!({
                    "isError": true,
                    "content": [{
                        "type": "text",
                        "text": format!("需要人工协助处理安全验证！请打开处理通道: /human/{}", action_id)
                    }]
                }))
            }
            Err(e) => Ok(serde_json::json!({
                "isError": true,
                "content": [{
                    "type": "text",
                    "text": format!("认证失败: {}", e)
                }]
            })),
        },
        "txffp_list_cards" => match service.list_cards().await {
            Ok(cards) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&cards).unwrap_or_default()
                }]
            })),
            Err(e) => Ok(serde_json::json!({
                "isError": true,
                "content": [{ "type": "text", "text": e.to_string() }]
            })),
        },
        "txffp_invoice_preview" => {
            let req: Result<InvoicePreviewRequest, _> = serde_json::from_value(args);
            match req {
                Ok(req) => match service.preview_invoice(req).await {
                    Ok(preview) => Ok(serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&preview).unwrap_or_default()
                        }]
                    })),
                    Err(e) => Ok(serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": e.to_string() }]
                    })),
                },
                Err(e) => Err(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                }),
            }
        }
        "txffp_invoice_create" => {
            let req: Result<CreateInvoiceRequest, _> = serde_json::from_value(args);
            match req {
                Ok(req) => match service.create_invoice(req).await {
                    Ok(resp) => Ok(serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&resp).unwrap_or_default()
                        }]
                    })),
                    Err(e) => Ok(serde_json::json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": e.to_string() }]
                    })),
                },
                Err(e) => Err(JsonRpcError {
                    code: -32602,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                }),
            }
        }
        "txffp_invoice_list" => match service.list_invoices().await {
            Ok(invoices) => Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&invoices).unwrap_or_default()
                }]
            })),
            Err(e) => Ok(serde_json::json!({
                "isError": true,
                "content": [{ "type": "text", "text": e.to_string() }]
            })),
        },
        _ => Err(JsonRpcError {
            code: -32601,
            message: format!("Tool not found: {}", name),
            data: None,
        }),
    }
}
