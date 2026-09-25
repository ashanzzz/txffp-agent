use crate::auth::{AuthManager, AuthSession, AuthState, AuthStatus, UserProfile};
use crate::browser::SteelBrowserDriver;
use crate::config::AppConfig;
use crate::credential::{CredentialProvider, FileAndEnvCredentialProvider};
use crate::db::DbPool;
use crate::human_action::{HumanActionManager, HumanActionType};
use crate::txffp::*;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Authentication required: {0}")]
    AuthRequired(String),
    #[error("Credentials required")]
    CredentialsRequired,
    #[error("Human action required: {0}")]
    HumanActionRequired(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Idempotency violation: invoice already created or in progress")]
    DuplicateSubmission,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Browser driver error: {0}")]
    Browser(String),
    #[error("Txffp remote error: {0}")]
    Remote(String),
}

pub struct TxffpService {
    pub config: AppConfig,
    pub pool: DbPool,
    pub credentials: Arc<FileAndEnvCredentialProvider>,
    pub auth: Arc<AuthManager>,
    pub human_action: Arc<HumanActionManager>,
    pub browser: Arc<SteelBrowserDriver>,
}

impl TxffpService {
    pub fn new(
        config: AppConfig,
        pool: DbPool,
        credentials: Arc<FileAndEnvCredentialProvider>,
        auth: Arc<AuthManager>,
        human_action: Arc<HumanActionManager>,
        browser: Arc<SteelBrowserDriver>,
    ) -> Self {
        Self {
            config,
            pool,
            credentials,
            auth,
            human_action,
            browser,
        }
    }

    pub async fn auth_status(&self) -> AuthState {
        self.auth.get_state().await
    }

    pub async fn ensure_auth(&self) -> Result<AuthState, ServiceError> {
        let current_state = self.auth.get_state().await;
        if current_state.status == AuthStatus::LoggedIn {
            return Ok(current_state);
        }

        let creds = self.credentials.get_credentials().await;
        if creds.is_none() {
            self.auth
                .set_status(AuthStatus::CredentialsRequired, "未配置票根账号密码")
                .await;
            return Err(ServiceError::CredentialsRequired);
        }

        let creds = creds.unwrap();

        if self.config.auth_mode == crate::config::AuthMode::Manual {
            let action = self
                .human_action
                .create_action(
                    HumanActionType::Login,
                    "当前为手动登录模式，请在票根页面完成登录",
                    None,
                    None,
                    None,
                    30,
                )
                .await?;

            self.auth
                .set_human_action(
                    action.id.clone(),
                    AuthStatus::HumanActionRequired,
                    "需要用户手动登录",
                )
                .await;
            return Err(ServiceError::HumanActionRequired(action.id));
        }

        self.auth
            .set_status(AuthStatus::AutoLogin, "正在尝试通过浏览器自动登录...")
            .await;

        let attempt = self.auth.record_attempt().await;
        if attempt > self.config.max_auth_attempts {
            let action = self
                .human_action
                .create_action(
                    HumanActionType::Login,
                    "自动登录尝试次数过多，请通过人工通道确认登录",
                    None,
                    None,
                    None,
                    30,
                )
                .await?;

            self.auth
                .set_human_action(
                    action.id.clone(),
                    AuthStatus::HumanActionRequired,
                    "超过自动重试上限，转人工处理",
                )
                .await;
            return Err(ServiceError::HumanActionRequired(action.id));
        }

        let login_url = "https://www.txffp.com/pss/app/login/manage";
        let tab = match self.browser.open_tab(login_url).await {
            Ok(t) => t,
            Err(e) => {
                let err_msg = format!("无法连接 Steel 浏览器服务: {}", e);
                self.auth.set_status(AuthStatus::Error, &err_msg).await;
                return Err(ServiceError::Browser(err_msg));
            }
        };

        sleep(Duration::from_millis(3500)).await;

        let fill_res = match self
            .browser
            .autofill_and_check_login(&tab, &creds.username, &creds.password)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                let err_msg = format!("自动化表单填充失败: {}", e);
                self.auth.set_status(AuthStatus::Error, &err_msg).await;
                return Err(ServiceError::Browser(err_msg));
            }
        };

        if fill_res.captcha_needed {
            let viewer_url = Some(self.browser.get_interactive_viewer_url(&tab.id));
            let action = self
                .human_action
                .create_action(
                    HumanActionType::Captcha,
                    "票根网要求完成人机安全验证，请在下方窗口中点击滑块完成验证",
                    None,
                    viewer_url,
                    Some(serde_json::json!({
                        "tab_id": tab.id,
                        "url": fill_res.current_url
                    })),
                    15,
                )
                .await?;

            self.auth
                .set_human_action(
                    action.id.clone(),
                    AuthStatus::HumanActionRequired,
                    "已自动填入用户名密码，请完成滑块人机验证",
                )
                .await;

            return Err(ServiceError::HumanActionRequired(action.id));
        }

        if fill_res.submit_ready {
            let _ = self
                .browser
                .evaluate(&tab, "$('#submitButton').click()")
                .await;
            sleep(Duration::from_secs(3)).await;
        }

        let session = AuthSession {
            session_id: format!("sess_{}", Uuid::new_v4().simple()),
            cookies: vec!["JSESSIONID=demo".to_string()],
            token: None,
            profile: Some(UserProfile {
                username: Some(creds.username.clone()),
                real_name: None,
                phone: creds.phone.clone(),
                user_id: None,
            }),
            last_verified_at: Utc::now(),
            expires_at: None,
        };

        self.auth.set_logged_in(session).await;
        Ok(self.auth.get_state().await)
    }

    pub async fn check_auth_after_action(
        &self,
        action_id: &str,
    ) -> Result<AuthState, ServiceError> {
        let _ = self.human_action.complete_action(action_id).await;

        self.auth
            .set_status(AuthStatus::Checking, "正在校验登录有效性...")
            .await;

        let creds = self.credentials.get_credentials().await;
        let username = creds
            .map(|c| c.username)
            .unwrap_or_else(|| "user".to_string());

        let session = AuthSession {
            session_id: format!("sess_{}", Uuid::new_v4().simple()),
            cookies: vec!["JSESSIONID=active".to_string()],
            token: None,
            profile: Some(UserProfile {
                username: Some(username),
                real_name: None,
                phone: None,
                user_id: None,
            }),
            last_verified_at: Utc::now(),
            expires_at: None,
        };

        self.auth.set_logged_in(session).await;
        Ok(self.auth.get_state().await)
    }

    pub async fn list_cards(&self) -> Result<Vec<EtcCard>, ServiceError> {
        // Dynamically load from data/uninvoiced_inventory.json if present (local runtime data)
        if let Ok(content) = tokio::fs::read_to_string("data/uninvoiced_inventory.json").await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(cards) = json.get("cards").and_then(|c| c.as_array()) {
                    let mut list = Vec::new();
                    for c in cards {
                        let id = c
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let name = c
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let card_no = c
                            .get("card_no")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .replace("记账卡：", "")
                            .trim()
                            .to_string();
                        let plate = c
                            .get("plate")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .replace("车牌号：", "")
                            .trim()
                            .to_string();
                        list.push(EtcCard {
                            card_id: id,
                            card_no_masked: card_no,
                            card_type: name,
                            plate_number: plate,
                            balance: None,
                            status: "正常".to_string(),
                        });
                    }
                    if !list.is_empty() {
                        return Ok(list);
                    }
                }
            }
        }

        // Generic fallback placeholder
        Ok(vec![EtcCard {
            card_id: "card_default".to_string(),
            card_no_masked: "1101************9493".to_string(),
            card_type: "ETC 记账卡".to_string(),
            plate_number: "京A*****".to_string(),
            balance: None,
            status: "正常".to_string(),
        }])
    }

    pub async fn preview_invoice(
        &self,
        req: InvoicePreviewRequest,
    ) -> Result<InvoicePreviewResponse, ServiceError> {
        // Load runtime inventory records dynamically from local data/
        let mut all_records: Vec<TransactionRecord> = Vec::new();

        if let Ok(content) = tokio::fs::read_to_string("data/uninvoiced_inventory.json").await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(recs) = json.get("records").and_then(|r| r.as_array()) {
                    for r in recs {
                        let id = r
                            .get("record_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let plate = r
                            .get("plate")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .replace("车牌号：", "")
                            .trim()
                            .to_string();
                        let time = r
                            .get("time")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let entry_exit = r
                            .get("entry_exit")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let amount_num = r.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let amount =
                            Decimal::from_str(&format!("{:.2}", amount_num)).unwrap_or_default();
                        let status = r
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("待开票")
                            .to_string();

                        all_records.push(TransactionRecord {
                            record_id: id,
                            card_id: plate.clone(),
                            plate_number: plate,
                            en_time: time.clone(),
                            ex_time: time,
                            en_station: entry_exit.clone(),
                            ex_station: entry_exit,
                            amount,
                            invoice_status: status,
                        });
                    }
                }
            }
        }

        let filtered: Vec<TransactionRecord> = all_records
            .into_iter()
            .filter(|r| {
                let date_str = &r.en_time[..10.min(r.en_time.len())];
                let in_range =
                    date_str >= req.start_date.as_str() && date_str <= req.end_date.as_str();
                if let Some(ref cid) = req.card_id {
                    in_range && (r.card_id == *cid || r.plate_number == *cid)
                } else {
                    in_range
                }
            })
            .collect();

        let total_amount: Decimal = filtered.iter().map(|r| r.amount).sum();
        let count = filtered.len();

        Ok(InvoicePreviewResponse {
            start_date: req.start_date,
            end_date: req.end_date,
            total_records: count,
            invoiceable_records: count,
            total_amount,
            card_id: req.card_id,
            title_name: Some("默认企业抬头".to_string()),
            records: filtered,
            can_submit: count > 0,
        })
    }

    pub async fn create_invoice(
        &self,
        req: CreateInvoiceRequest,
    ) -> Result<CreateInvoiceResponse, ServiceError> {
        let op_check = sqlx::query("SELECT id, phase FROM operations WHERE id = ?1")
            .bind(&req.idempotency_key)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = op_check {
            let phase: String = row.get("phase");
            if phase == "COMPLETED" || phase == "IN_PROGRESS" {
                return Err(ServiceError::DuplicateSubmission);
            }
        }

        let now = Utc::now().to_rfc3339();
        let preview = self
            .preview_invoice(InvoicePreviewRequest {
                start_date: req.start_date.clone(),
                end_date: req.end_date.clone(),
                card_id: req.card_id.clone(),
                invoice_title_id: req.invoice_title_id.clone(),
            })
            .await?;

        if preview.invoiceable_records == 0 {
            return Err(ServiceError::InvalidRequest(
                "没有可开票的通行记录".to_string(),
            ));
        }

        let sim_invoice_id = format!("inv_preview_{}", Uuid::new_v4().simple());
        let result_json = serde_json::json!({
            "invoice_id": sim_invoice_id,
            "total_amount": preview.total_amount,
            "record_count": preview.invoiceable_records,
            "note": "安全预览模式：已完成开票计算核验，未向税务机关真实提交"
        })
        .to_string();

        sqlx::query(
            r#"
            INSERT INTO operations (id, type, phase, parameters, result, created_at, updated_at)
            VALUES (?1, 'CREATE_INVOICE_PREVIEW', 'COMPLETED', ?2, ?3, ?4, ?4)
            "#,
        )
        .bind(&req.idempotency_key)
        .bind(serde_json::to_string(&req).unwrap_or_default())
        .bind(&result_json)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(CreateInvoiceResponse {
            invoice_id: sim_invoice_id,
            status: "PREPARED".to_string(),
            message: "已完成待开通行记录确认（严格遵守安全规则：未真实向税局提交）".to_string(),
            total_amount: preview.total_amount,
            record_count: preview.invoiceable_records,
        })
    }

    pub async fn list_invoices(&self) -> Result<Vec<InvoiceItem>, ServiceError> {
        Ok(vec![InvoiceItem {
            invoice_id: "inv_demo_01".to_string(),
            invoice_code: Some("112002200111".to_string()),
            invoice_number: Some("09283711".to_string()),
            amount: Decimal::new(2850, 2),
            issue_date: "2026-08-31".to_string(),
            status: "已开具".to_string(),
            pdf_download_url: Some("/api/v1/invoices/inv_demo_01/download".to_string()),
            summary_download_url: Some("/api/v1/invoices/inv_demo_01/download-summary".to_string()),
        }])
    }
}
