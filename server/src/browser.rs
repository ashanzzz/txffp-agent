use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use tokio::time::{sleep, timeout};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoLoginResult {
    pub success: bool,
    pub tab_id: String,
    pub current_url: String,
    pub message: String,
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

    pub async fn get_active_txffp_tab(&self) -> Result<Option<CdpTarget>, BrowserError> {
        let targets = self.list_targets().await?;
        let tab = targets
            .into_iter()
            .find(|t| t.url.contains("txffp.com") || t.title.contains("票根"));
        Ok(tab)
    }

    pub async fn check_session_alive(&self) -> Result<bool, BrowserError> {
        let targets = self.list_targets().await?;
        let target = match targets
            .into_iter()
            .find(|t| t.url.contains("pss.txffp.com"))
        {
            Some(t) => t,
            None => return Ok(false),
        };

        let script = r#"
            (() => {
                const text = document.body ? document.body.innerText : "";
                const url = window.location.href;
                const hasDashboard = text.includes("个人中心") || text.includes("我的ETC");
                const hasRelogin = text.includes("重新登录") || text.includes("无法访问") || url.includes("sso.txffp.com");
                return hasDashboard && !hasRelogin;
            })()
        "#;

        if let Ok(val) = self.evaluate(&target, script).await {
            return Ok(val.as_bool().unwrap_or(false));
        }

        Ok(false)
    }

    pub fn get_interactive_viewer_url(&self, target_id: &str) -> String {
        let cdp_host = self
            .cdp_base_url
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        format!(
            "http://{}/devtools/devtools_app.html?ws={}/devtools/page/{}",
            cdp_host, cdp_host, target_id
        )
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

        if let Ok(cdp_parsed) = reqwest::Url::parse(&self.cdp_base_url) {
            if let Some(host) = cdp_parsed.host_str() {
                if let Ok(mut parsed_ws) = reqwest::Url::parse(&ws_url) {
                    if let Some(ws_host) = parsed_ws.host_str() {
                        if ws_host == host && parsed_ws.port() != cdp_parsed.port() {
                            let _ = parsed_ws.set_port(cdp_parsed.port());
                            ws_url = parsed_ws.to_string();
                        }
                    }
                }
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

    pub async fn dispatch_mouse_click(
        &self,
        target: &CdpTarget,
        x: f64,
        y: f64,
    ) -> Result<(), BrowserError> {
        let mut ws_url = target.websocket_debugger_url.clone().ok_or_else(|| {
            BrowserError::Cdp("No webSocketDebuggerUrl available for target".to_string())
        })?;

        if let Ok(cdp_parsed) = reqwest::Url::parse(&self.cdp_base_url) {
            if let Some(host) = cdp_parsed.host_str() {
                if let Ok(mut parsed_ws) = reqwest::Url::parse(&ws_url) {
                    if let Some(ws_host) = parsed_ws.host_str() {
                        if ws_host == host && parsed_ws.port() != cdp_parsed.port() {
                            let _ = parsed_ws.set_port(cdp_parsed.port());
                            ws_url = parsed_ws.to_string();
                        }
                    }
                }
            }
        }

        let (ws_stream, _) = timeout(Duration::from_secs(5), connect_async(&ws_url))
            .await
            .map_err(|_| BrowserError::Timeout)??;

        let (mut write, _) = ws_stream.split();

        // 1. mouseMoved
        let move_req = serde_json::json!({
            "id": 10,
            "method": "Input.dispatchMouseEvent",
            "params": { "type": "mouseMoved", "x": x, "y": y }
        });
        write
            .send(Message::Text(move_req.to_string().into()))
            .await?;
        sleep(Duration::from_millis(100)).await;

        // 2. mousePressed
        let press_req = serde_json::json!({
            "id": 11,
            "method": "Input.dispatchMouseEvent",
            "params": { "type": "mousePressed", "x": x, "y": y, "button": "left", "clickCount": 1 }
        });
        write
            .send(Message::Text(press_req.to_string().into()))
            .await?;
        sleep(Duration::from_millis(80)).await;

        // 3. mouseReleased
        let release_req = serde_json::json!({
            "id": 12,
            "method": "Input.dispatchMouseEvent",
            "params": { "type": "mouseReleased", "x": x, "y": y, "button": "left", "clickCount": 1 }
        });
        write
            .send(Message::Text(release_req.to_string().into()))
            .await?;

        Ok(())
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

    pub async fn execute_auto_login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AutoLoginResult, BrowserError> {
        let login_url = "https://www.txffp.com/pss/app/login/manage";
        let target = self.open_tab(login_url).await?;
        // 0. Quick check: Is the browser already logged in?
        sleep(Duration::from_millis(2000)).await;
        if let Ok(val) = self.evaluate(&target, "document.body ? (document.body.innerText.includes('个人中心') && document.body.innerText.includes('我的ETC')) : false").await {
            if val.as_bool() == Some(true) {
                return Ok(AutoLoginResult {
                    success: true,
                    tab_id: target.id,
                    current_url: target.url,
                    message: "检测到已有活跃登录会话，直接复用".to_string(),
                });
            }
        }

        // Poll for inputs and captcha icon
        let mut coords_opt: Option<(f64, f64, String)> = None;
        let escaped_user = username.replace('\\', "\\\\").replace('"', "\\\"");
        let escaped_pass = password.replace('\\', "\\\\").replace('"', "\\\"");

        for _ in 0..20 {
            sleep(Duration::from_millis(600)).await;

            let check_script = format!(
                r#"
                (() => {{
                    const u = document.getElementById('loginName');
                    const p = document.getElementById('passwd');
                    const icon = document.getElementById('aliyunCaptcha-checkbox-icon') || document.getElementById('aliyunCaptcha-checkbox-body');
                    if (u && p && !u.value) {{
                        u.value = "{username}";
                        u.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        u.dispatchEvent(new Event('change', {{ bubbles: true }}));
                        p.value = "{password}";
                        p.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        p.dispatchEvent(new Event('change', {{ bubbles: true }}));
                    }}
                    if (!icon) return null;
                    const r = icon.getBoundingClientRect();
                    if (r.width === 0 || r.height === 0) return null;
                    return {{
                        x: r.x + r.width / 2,
                        y: r.y + r.height / 2,
                        text: document.getElementById('sc')?.innerText?.trim() || ''
                    }};
                }})()
                "#,
                username = escaped_user,
                password = escaped_pass
            );

            if let Ok(val) = self.evaluate(&target, &check_script).await {
                if let (Some(x), Some(y)) = (
                    val.get("x").and_then(|v| v.as_f64()),
                    val.get("y").and_then(|v| v.as_f64()),
                ) {
                    let text = val
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    coords_opt = Some((x, y, text));
                    break;
                }
            }
        }

        let (x, y, sc_text) = coords_opt.ok_or_else(|| {
            BrowserError::Cdp("Timed out waiting for login form and captcha".to_string())
        })?;

        // If timed out, reset first
        if sc_text.contains("重试") {
            let _ = self.dispatch_mouse_click(&target, x, y).await;
            sleep(Duration::from_millis(1500)).await;
        }

        // Click captcha checkbox
        self.dispatch_mouse_click(&target, x, y).await?;
        sleep(Duration::from_millis(2500)).await;

        // Check if verified
        let verify_script = r#"
            (() => {
                const captchaParam = document.getElementById('captchaVerifyParam');
                const submitBtn = document.getElementById('submitButton');
                const sc = document.getElementById('sc');
                return {
                    captcha: captchaParam ? captchaParam.value : "",
                    ready: submitBtn ? submitBtn.classList.contains('taiji_ajaxForm') : false,
                    text: sc ? sc.innerText.trim() : ""
                };
            })()
        "#;

        let verify_val = self.evaluate(&target, verify_script).await?;
        let is_ready = verify_val
            .get("ready")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_captcha = verify_val
            .get("captcha")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        if is_ready || has_captcha {
            // Click submit
            let _ = self.evaluate(&target, "$('#submitButton').click()").await;
            sleep(Duration::from_millis(4000)).await;

            let final_val = self.evaluate(&target, "window.location.href").await?;
            let final_url = final_val.as_str().unwrap_or_default().to_string();

            Ok(AutoLoginResult {
                success: true,
                tab_id: target.id,
                current_url: final_url,
                message: "自动登录已成功完成".to_string(),
            })
        } else {
            Ok(AutoLoginResult {
                success: false,
                tab_id: target.id,
                current_url: target.url,
                message: "人机验证未直接通过，已转入人工通道".to_string(),
            })
        }
    }
}
