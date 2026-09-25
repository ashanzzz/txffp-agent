use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HumanActionType {
    Login,
    SmsCode,
    Captcha,
    Mfa,
    QrConfirm,
    DeviceConfirm,
    AccountSelection,
    Agreement,
    UnknownPage,
    AuthConfirmation,
    LiveInvoiceConfirm,
}

impl std::fmt::Display for HumanActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HumanActionState {
    WaitingUser,
    Completed,
    Cancelled,
    Expired,
}

impl std::fmt::Display for HumanActionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanAction {
    pub id: String,
    pub token: String,
    pub action_type: HumanActionType,
    pub state: HumanActionState,
    pub message: String,
    pub operation_id: Option<String>,
    pub context_json: Option<serde_json::Value>,
    pub viewer_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct HumanActionManager {
    pool: SqlitePool,
}

impl HumanActionManager {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_action(
        &self,
        action_type: HumanActionType,
        message: impl Into<String>,
        operation_id: Option<String>,
        viewer_url: Option<String>,
        context: Option<serde_json::Value>,
        ttl_minutes: i64,
    ) -> Result<HumanAction, sqlx::Error> {
        let id = format!("ha_{}", Uuid::new_v4().simple());
        let token = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + Duration::minutes(ttl_minutes);
        let msg = message.into();

        let context_val = context.unwrap_or_else(|| {
            serde_json::json!({
                "viewer_url": viewer_url
            })
        });
        let context_str = serde_json::to_string(&context_val).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO human_actions (id, token, type, state, message, operation_id, context_json, expires_at, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(&id)
        .bind(&token)
        .bind(action_type.to_string())
        .bind(HumanActionState::WaitingUser.to_string())
        .bind(&msg)
        .bind(&operation_id)
        .bind(&context_str)
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(HumanAction {
            id,
            token,
            action_type,
            state: HumanActionState::WaitingUser,
            message: msg,
            operation_id,
            context_json: Some(context_val),
            viewer_url,
            created_at: now,
            expires_at,
            updated_at: now,
        })
    }

    pub async fn get_by_token(&self, token: &str) -> Result<Option<HumanAction>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, token, type, state, message, operation_id, context_json, expires_at, created_at, updated_at
            FROM human_actions
            WHERE token = ?1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            let type_str: String = r.get("type");
            let state_str: String = r.get("state");
            let message: String = r.get("message");
            let operation_id: Option<String> = r.get("operation_id");
            let context_str: Option<String> = r.get("context_json");
            let expires_at_str: String = r.get("expires_at");
            let created_at_str: String = r.get("created_at");
            let updated_at_str: String = r.get("updated_at");

            let action_type = match type_str.as_str() {
                "SmsCode" | "SMS_CODE" => HumanActionType::SmsCode,
                "Captcha" | "CAPTCHA" => HumanActionType::Captcha,
                "Mfa" | "MFA" => HumanActionType::Mfa,
                "QrConfirm" | "QR_CONFIRM" => HumanActionType::QrConfirm,
                "DeviceConfirm" | "DEVICE_CONFIRM" => HumanActionType::DeviceConfirm,
                "AccountSelection" | "ACCOUNT_SELECTION" => HumanActionType::AccountSelection,
                "Agreement" | "AGREEMENT" => HumanActionType::Agreement,
                "UnknownPage" | "UNKNOWN_PAGE" => HumanActionType::UnknownPage,
                "AuthConfirmation" | "AUTH_CONFIRMATION" => HumanActionType::AuthConfirmation,
                "LiveInvoiceConfirm" | "LIVE_INVOICE_CONFIRM" => {
                    HumanActionType::LiveInvoiceConfirm
                }
                _ => HumanActionType::Login,
            };

            let state = match state_str.as_str() {
                "Completed" | "COMPLETED" => HumanActionState::Completed,
                "Cancelled" | "CANCELLED" => HumanActionState::Cancelled,
                "Expired" | "EXPIRED" => HumanActionState::Expired,
                _ => HumanActionState::WaitingUser,
            };

            let context_val: Option<serde_json::Value> = context_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());

            let viewer_url = context_val
                .as_ref()
                .and_then(|c| c.get("viewer_url"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            HumanAction {
                id: r.get("id"),
                token: r.get("token"),
                action_type,
                state,
                message,
                operation_id,
                context_json: context_val,
                viewer_url,
                created_at: DateTime::parse_from_rfc3339(&created_at_str)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                expires_at: DateTime::parse_from_rfc3339(&expires_at_str)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
            }
        }))
    }

    pub async fn complete_action(&self, id: &str) -> Result<bool, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        let state = HumanActionState::Completed.to_string();
        let res = sqlx::query(
            r#"
            UPDATE human_actions
            SET state = ?1, updated_at = ?2
            WHERE id = ?3 AND state = 'WaitingUser'
            "#,
        )
        .bind(&state)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected() > 0)
    }

    pub async fn cancel_action(&self, id: &str) -> Result<bool, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        let state = HumanActionState::Cancelled.to_string();
        let res = sqlx::query(
            r#"
            UPDATE human_actions
            SET state = ?1, updated_at = ?2
            WHERE id = ?3 AND state = 'WaitingUser'
            "#,
        )
        .bind(&state)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(res.rows_affected() > 0)
    }

    pub fn render_html_page(&self, action: &HumanAction) -> String {
        let viewer_html = if let Some(ref url) = action.viewer_url {
            format!(
                r#"<div class="viewer-container">
                    <iframe src="{}" class="viewer-iframe" title="Browser Session"></iframe>
                    <div class="viewer-hint">如果页面未自动加载，请 <a href="{}" target="_blank">在新窗口打开浏览器</a></div>
                </div>"#,
                url, url
            )
        } else {
            r#"<div class="no-viewer">当前任务不需要远程浏览器实时交互，或浏览器画面未就绪。</div>"#
                .to_string()
        };

        let state_badge = match action.state {
            HumanActionState::WaitingUser => {
                r#"<span class="badge badge-waiting">等待用户操作</span>"#
            }
            HumanActionState::Completed => r#"<span class="badge badge-completed">已完成</span>"#,
            HumanActionState::Cancelled => r#"<span class="badge badge-cancelled">已取消</span>"#,
            HumanActionState::Expired => r#"<span class="badge badge-expired">已过期</span>"#,
        };

        format!(
            r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>票根开票助手 - 需要人工操作</title>
    <style>
        :root {{
            --bg-color: #f8fafc;
            --panel-bg: #ffffff;
            --text-main: #0f172a;
            --text-sub: #64748b;
            --primary: #2563eb;
            --primary-hover: #1d4ed8;
            --success: #16a34a;
            --warning: #ca8a04;
            --danger: #dc2626;
            --border: #e2e8f0;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background-color: var(--bg-color);
            color: var(--text-main);
            padding: 24px;
            display: flex;
            justify-content: center;
        }}
        .container {{
            width: 100%;
            max-width: 1100px;
            background: var(--panel-bg);
            border: 1px solid var(--border);
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.05);
            padding: 24px;
        }}
        header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            border-bottom: 1px solid var(--border);
            padding-bottom: 16px;
            margin-bottom: 20px;
        }}
        h1 {{ font-size: 20px; font-weight: 600; }}
        .badge {{
            padding: 4px 10px;
            border-radius: 4px;
            font-size: 13px;
            font-weight: 500;
        }}
        .badge-waiting {{ background: #fef3c7; color: #92400e; }}
        .badge-completed {{ background: #dcfce7; color: #166534; }}
        .badge-cancelled {{ background: #f1f5f9; color: #475569; }}
        .badge-expired {{ background: #fee2e2; color: #991b1b; }}
        .message-box {{
            background: #eff6ff;
            border-left: 4px solid var(--primary);
            padding: 14px 18px;
            border-radius: 4px;
            margin-bottom: 20px;
            font-size: 15px;
            line-height: 1.5;
        }}
        .viewer-container {{
            width: 100%;
            height: 600px;
            border: 1px solid var(--border);
            border-radius: 6px;
            overflow: hidden;
            display: flex;
            flex-direction: column;
            margin-bottom: 20px;
        }}
        .viewer-iframe {{
            flex: 1;
            width: 100%;
            border: none;
        }}
        .viewer-hint {{
            background: #f8fafc;
            border-top: 1px solid var(--border);
            padding: 8px 14px;
            font-size: 13px;
            color: var(--text-sub);
        }}
        .actions {{
            display: flex;
            gap: 12px;
            justify-content: flex-end;
        }}
        button {{
            padding: 10px 20px;
            border-radius: 6px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            border: 1px solid transparent;
            transition: all 0.15s ease;
        }}
        .btn-primary {{
            background-color: var(--primary);
            color: white;
        }}
        .btn-primary:hover {{ background-color: var(--primary-hover); }}
        .btn-secondary {{
            background-color: white;
            color: var(--text-sub);
            border-color: var(--border);
        }}
        .btn-secondary:hover {{ background-color: #f1f5f9; }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>票根开票助手 - 人工处理通道</h1>
            {state_badge}
        </header>

        <div class="message-box">
            <strong>任务提示：</strong> {message}
        </div>

        {viewer_html}

        <div class="actions">
            <button class="btn-secondary" onclick="handleAction('cancel')">取消任务</button>
            <button class="btn-primary" onclick="handleAction('complete')">我已完成操作并继续</button>
        </div>
    </div>

    <script>
        async function handleAction(type) {{
            const actionId = "{action_id}";
            const endpoint = `/api/v1/human-actions/${{actionId}}/${{type}}`;
            try {{
                const res = await fetch(endpoint, {{ method: 'POST' }});
                const data = await res.json();
                if (data.ok) {{
                    alert(type === 'complete' ? '已通知系统重新验证！' : '任务已取消。');
                    window.location.reload();
                }} else {{
                    alert('操作失败: ' + (data.error?.message || '未知错误'));
                }}
            }} catch (e) {{
                alert('网络请求失败: ' + e.message);
            }}
        }}
    </script>
</body>
</html>"#,
            state_badge = state_badge,
            message = action.message,
            viewer_html = viewer_html,
            action_id = action.id,
        )
    }
}
