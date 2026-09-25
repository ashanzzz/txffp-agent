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
use std::sync::Arc;
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
            // Already logged in, quickly verify
            return Ok(current_state);
        }

        // Check credentials
        let creds = self.credentials.get_credentials().await;
        if creds.is_none() {
            self.auth
                .set_status(AuthStatus::CredentialsRequired, "未配置票根账号密码")
                .await;
            return Err(ServiceError::CredentialsRequired);
        }

        let _creds = creds.unwrap();

        // Check if auto login is disabled or manual only
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

        // Auto login attempt with Steel
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

        // Create browser session for login
        match self.browser.create_session().await {
            Ok(session) => {
                let viewer_url = session.session_viewer_url.clone();
                let action = self
                    .human_action
                    .create_action(
                        HumanActionType::Login,
                        "已启动浏览器登录会话，若出现短信验证或人机验证请在下方界面完成",
                        None,
                        viewer_url,
                        None,
                        30,
                    )
                    .await?;

                self.auth
                    .set_human_action(
                        action.id.clone(),
                        AuthStatus::HumanActionRequired,
                        "登录会话已就绪，等待用户在交互窗口确认或完成验证",
                    )
                    .await;

                Err(ServiceError::HumanActionRequired(action.id))
            }
            Err(e) => {
                let err_msg = format!("无法连接 Steel 浏览器服务: {}", e);
                self.auth.set_status(AuthStatus::Error, &err_msg).await;
                Err(ServiceError::Browser(err_msg))
            }
        }
    }

    pub async fn check_auth_after_action(
        &self,
        action_id: &str,
    ) -> Result<AuthState, ServiceError> {
        let _ = self.human_action.complete_action(action_id).await;

        self.auth
            .set_status(AuthStatus::Checking, "正在校验登录有效性...")
            .await;

        let session = AuthSession {
            session_id: format!("sess_{}", Uuid::new_v4().simple()),
            cookies: vec!["JSESSIONID=demo".to_string()],
            token: None,
            profile: Some(UserProfile {
                username: Some("verified_user".to_string()),
                real_name: None,
                phone: None,
                user_id: Some("uid_1001".to_string()),
            }),
            last_verified_at: Utc::now(),
            expires_at: None,
        };

        self.auth.set_logged_in(session).await;
        Ok(self.auth.get_state().await)
    }

    pub async fn list_cards(&self) -> Result<Vec<EtcCard>, ServiceError> {
        self.ensure_auth().await?;

        Ok(vec![EtcCard {
            card_id: "card_01".to_string(),
            card_no_masked: "1101************1234".to_string(),
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
        self.ensure_auth().await?;

        let mock_records = vec![
            TransactionRecord {
                record_id: "rec_001".to_string(),
                card_id: req.card_id.clone().unwrap_or_else(|| "card_01".to_string()),
                plate_number: "京A*****".to_string(),
                en_time: format!("{} 08:30:00", req.start_date),
                ex_time: format!("{} 09:15:00", req.start_date),
                en_station: "北京站入口".to_string(),
                ex_station: "收费站A出口".to_string(),
                amount: Decimal::new(2550, 2),
                invoice_status: "UNINVOICED".to_string(),
            },
            TransactionRecord {
                record_id: "rec_002".to_string(),
                card_id: req.card_id.clone().unwrap_or_else(|| "card_01".to_string()),
                plate_number: "京A*****".to_string(),
                en_time: format!("{} 18:00:00", req.end_date),
                ex_time: format!("{} 18:45:00", req.end_date),
                en_station: "收费站A入口".to_string(),
                ex_station: "北京站出口".to_string(),
                amount: Decimal::new(3200, 2),
                invoice_status: "UNINVOICED".to_string(),
            },
        ];

        let total_amount: Decimal = mock_records.iter().map(|r| r.amount).sum();
        let count = mock_records.len();

        Ok(InvoicePreviewResponse {
            start_date: req.start_date,
            end_date: req.end_date,
            total_records: count,
            invoiceable_records: count,
            total_amount,
            card_id: req.card_id,
            title_name: Some("测试抬头有限公司".to_string()),
            records: mock_records,
            can_submit: count > 0,
        })
    }

    pub async fn create_invoice(
        &self,
        req: CreateInvoiceRequest,
    ) -> Result<CreateInvoiceResponse, ServiceError> {
        self.ensure_auth().await?;

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
        let params_json = serde_json::to_string(&req).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO operations (id, type, phase, parameters, created_at, updated_at)
            VALUES (?1, 'CREATE_INVOICE', 'IN_PROGRESS', ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET phase = 'IN_PROGRESS', updated_at = ?4
            "#,
        )
        .bind(&req.idempotency_key)
        .bind(&params_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let preview = self
            .preview_invoice(InvoicePreviewRequest {
                start_date: req.start_date,
                end_date: req.end_date,
                card_id: req.card_id,
                invoice_title_id: req.invoice_title_id,
            })
            .await?;

        if preview.invoiceable_records == 0 {
            let _ = sqlx::query(
                "UPDATE operations SET phase = 'FAILED', error = 'NO_INVOICEABLE_RECORDS', updated_at = ?1 WHERE id = ?2",
            )
            .bind(&now)
            .bind(&req.idempotency_key)
            .execute(&self.pool)
            .await;

            return Err(ServiceError::InvalidRequest(
                "没有可开票的通行记录".to_string(),
            ));
        }

        let invoice_id = format!("inv_{}", Uuid::new_v4().simple());
        let result_json = serde_json::json!({
            "invoice_id": invoice_id,
            "total_amount": preview.total_amount,
            "record_count": preview.invoiceable_records
        })
        .to_string();

        sqlx::query(
            "UPDATE operations SET phase = 'COMPLETED', result = ?1, updated_at = ?2 WHERE id = ?3",
        )
        .bind(&result_json)
        .bind(&now)
        .bind(&req.idempotency_key)
        .execute(&self.pool)
        .await?;

        Ok(CreateInvoiceResponse {
            invoice_id,
            status: "SUCCESS".to_string(),
            message: "开票申请已成功提交".to_string(),
            total_amount: preview.total_amount,
            record_count: preview.invoiceable_records,
        })
    }

    pub async fn list_invoices(&self) -> Result<Vec<InvoiceItem>, ServiceError> {
        self.ensure_auth().await?;

        Ok(vec![InvoiceItem {
            invoice_id: "inv_demo_101".to_string(),
            invoice_code: Some("011002300111".to_string()),
            invoice_number: Some("12345678".to_string()),
            amount: Decimal::new(5750, 2),
            issue_date: "2026-09-20".to_string(),
            status: "ISSUED".to_string(),
            pdf_download_url: Some("/api/v1/invoices/inv_demo_101/download".to_string()),
            summary_download_url: Some(
                "/api/v1/invoices/inv_demo_101/download-summary".to_string(),
            ),
        }])
    }
}
