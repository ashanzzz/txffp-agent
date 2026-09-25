use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteelSession {
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "websocketUrl", default)]
    pub websocket_url: Option<String>,
    #[serde(rename = "debugUrl", default)]
    pub debug_url: Option<String>,
    #[serde(rename = "sessionViewerUrl", default)]
    pub session_viewer_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpTarget {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "type", default)]
    pub target_type: String,
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl", default)]
    pub websocket_debugger_url: Option<String>,
    #[serde(rename = "devtoolsFrontendUrl", default)]
    pub devtools_frontend_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFillResult {
    pub current_url: String,
    pub captcha_needed: bool,
    pub submit_ready: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("CDP error: {0}")]
    Cdp(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Steel API error: {0}")]
    Api(String),
}

#[derive(Clone)]
pub struct SteelBrowserDriver {
    pub steel_base_url: String,
    pub cdp_base_url: String,
    client: reqwest::Client,
}

impl SteelBrowserDriver {
    pub fn new(steel_base_url: impl Into<String>, cdp_base_url: impl Into<String>) -> Self {
        Self {
            steel_base_url: steel_base_url.into().trim_end_matches('/').to_string(),
            cdp_base_url: cdp_base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn check_health(&self) -> bool {
        let url = format!("{}/json/version", self.cdp_base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn list_targets(&self) -> Result<Vec<CdpTarget>, BrowserError> {
        let url = format!("{}/json/list", self.cdp_base_url);
        let res = self.client.get(&url).send().await?;
        if !res.status().is_success() {
            return Err(BrowserError::Cdp(format!(
                "Failed to list targets: HTTP {}",
                res.status()
            )));
        }
        let targets: Vec<CdpTarget> = res.json().await?;
        Ok(targets)
    }

    pub async fn open_tab(&self, target_url: &str) -> Result<CdpTarget, BrowserError> {
        let url = format!("{}/json/new?{}", self.cdp_base_url, target_url);
        let res = self.client.put(&url).send().await?;
        if !res.status().is_success() {
            return Err(BrowserError::Cdp(format!(
                "Failed to create tab: HTTP {}",
                res.status()
            )));
        }
        let target: CdpTarget = res.json().await?;
        Ok(target)
    }

    pub async fn close_tab(&self, target_id: &str) -> Result<(), BrowserError> {
        let url = format!("{}/json/close/{}", self.cdp_base_url, target_id);
        let _ = self.client.put(&url).send().await;
        Ok(())
    }

    pub async fn evaluate(
        &self,
        target: &CdpTarget,
        expression: &str,
    ) -> Result<Value, BrowserError> {
        let mut ws_url = target.websocket_debugger_url.clone().ok_or_else(|| {
            BrowserError::Cdp("No webSocketDebuggerUrl available for target".to_string())
        })?;

        // Normalize host/port if mapped through Docker port (e.g. host port 19223)
        if let Ok(cdp_parsed) = reqwest::Url::parse(&self.cdp_base_url) {
            if let Some(host) = cdp_parsed.host_str() {
                let port_suffix = if let Some(port) = cdp_parsed.port() {
                    format!("{}:{}", host, port)
                } else {
                    host.to_string()
                };

                if let Ok(mut parsed_ws) = reqwest::Url::parse(&ws_url) {
                    if let Some(ws_host) = parsed_ws.host_str() {
                        if ws_host == host && parsed_ws.port() != cdp_parsed.port() {
                            let _ = parsed_ws.set_port(cdp_parsed.port());
                            ws_url = parsed_ws.to_string();
                        }
                    }
                }
                let _ = port_suffix;
            }
        }

        let (ws_stream, _) = timeout(Duration::from_secs(5), connect_async(&ws_url))
            .await
            .map_err(|_| BrowserError::Timeout)??;

        let (mut write, mut read) = ws_stream.split();

        let req = serde_json::json!({
            "id": 1,
            "method": "Runtime.evaluate",
            "params": {
                "expression": expression,
                "returnByValue": true
            }
        });

        write.send(Message::Text(req.to_string().into())).await?;

        let res = timeout(Duration::from_secs(10), async {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        if json.get("id").and_then(|v| v.as_i64()) == Some(1) {
                            if let Some(err) = json.get("error") {
                                return Err(BrowserError::Cdp(err.to_string()));
                            }
                            let val = json
                                .pointer("/result/result/value")
                                .cloned()
                                .unwrap_or(Value::Null);
                            return Ok(val);
                        }
                    }
                }
            }
            Err(BrowserError::Cdp(
                "WebSocket closed without response".to_string(),
            ))
        })
        .await
        .map_err(|_| BrowserError::Timeout)??;

        Ok(res)
    }

    pub async fn autofill_and_check_login(
        &self,
        target: &CdpTarget,
        username: &str,
        password: &str,
    ) -> Result<LoginFillResult, BrowserError> {
        let escaped_user = username.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_pass = password.replace('\\', "\\\\").replace('"', "\\\"");

        let script = format!(
            r#"
            (() => {{
                const loginInput = document.getElementById('loginName');
                const passInput = document.getElementById('passwd');
                const submitBtn = document.getElementById('submitButton');
                const captchaContainer = document.getElementById('sc');
                const captchaParam = document.getElementById('captchaVerifyParam');

                let filled = false;
                if (loginInput && passInput) {{
                    loginInput.value = "{username}";
                    loginInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    loginInput.dispatchEvent(new Event('change', {{ bubbles: true }}));

                    passInput.value = "{password}";
                    passInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    passInput.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    filled = true;
                }}

                const hasCaptchaEmbed = !!document.getElementById('aliyunCaptcha-window-embed');
                const captchaValue = captchaParam ? captchaParam.value : "";
                const isSubmitReady = submitBtn && submitBtn.classList.contains('taiji_ajaxForm') && !!captchaValue;

                return {{
                    current_url: window.location.href,
                    captcha_needed: hasCaptchaEmbed && !captchaValue,
                    submit_ready: isSubmitReady,
                    error_message: filled ? null : "未找到登录输入框"
                }};
            }})()
            "#,
            username = escaped_user,
            password = escaped_pass
        );

        let val = self.evaluate(target, &script).await?;
        let res: LoginFillResult = serde_json::from_value(val)
            .map_err(|e| BrowserError::Cdp(format!("Failed to parse login fill result: {}", e)))?;

        Ok(res)
    }
}
