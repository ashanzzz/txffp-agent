use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthStatus {
    #[default]
    Unknown,
    Checking,
    CredentialsRequired,
    LoggedOut,
    AutoLogin,
    LoginRequired,
    LoginInProgress,
    VerificationRequired,
    MfaRequired,
    CaptchaRequired,
    HumanActionRequired,
    AuthUnverified,
    LoggedIn,
    SessionExpired,
    RateLimited,
    Locked,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserProfile {
    pub username: Option<String>,
    pub real_name: Option<String>,
    pub phone: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub session_id: String,
    pub cookies: Vec<String>,
    pub token: Option<String>,
    pub profile: Option<UserProfile>,
    pub last_verified_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub status: AuthStatus,
    pub message: String,
    pub session: Option<AuthSession>,
    pub last_check_at: DateTime<Utc>,
    pub attempt_count: u32,
    pub active_human_action_id: Option<String>,
}

pub struct AuthManager {
    state: Arc<RwLock<AuthState>>,
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AuthState {
                status: AuthStatus::Unknown,
                message: "Authentication status uninitialized".to_string(),
                session: None,
                last_check_at: Utc::now(),
                attempt_count: 0,
                active_human_action_id: None,
            })),
        }
    }

    pub async fn get_state(&self) -> AuthState {
        self.state.read().await.clone()
    }

    pub async fn get_status(&self) -> AuthStatus {
        self.state.read().await.status
    }

    pub async fn set_status(&self, status: AuthStatus, message: impl Into<String>) {
        let mut lock = self.state.write().await;
        lock.status = status;
        lock.message = message.into();
        lock.last_check_at = Utc::now();
    }

    pub async fn set_logged_in(&self, session: AuthSession) {
        let mut lock = self.state.write().await;
        lock.status = AuthStatus::LoggedIn;
        lock.message = "Successfully authenticated and verified".to_string();
        lock.session = Some(session);
        lock.last_check_at = Utc::now();
        lock.attempt_count = 0;
        lock.active_human_action_id = None;
    }

    pub async fn record_attempt(&self) -> u32 {
        let mut lock = self.state.write().await;
        lock.attempt_count += 1;
        lock.attempt_count
    }

    pub async fn reset_attempts(&self) {
        let mut lock = self.state.write().await;
        lock.attempt_count = 0;
    }

    pub async fn set_human_action(
        &self,
        action_id: String,
        status: AuthStatus,
        message: impl Into<String>,
    ) {
        let mut lock = self.state.write().await;
        lock.status = status;
        lock.message = message.into();
        lock.active_human_action_id = Some(action_id);
        lock.last_check_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_manager_lifecycle() {
        let auth = AuthManager::new();
        assert_eq!(auth.get_status().await, AuthStatus::Unknown);

        auth.set_status(AuthStatus::AutoLogin, "Attempting auto login")
            .await;
        assert_eq!(auth.get_status().await, AuthStatus::AutoLogin);

        let attempt1 = auth.record_attempt().await;
        let attempt2 = auth.record_attempt().await;
        assert_eq!(attempt1, 1);
        assert_eq!(attempt2, 2);

        auth.set_human_action(
            "ha_123".to_string(),
            AuthStatus::HumanActionRequired,
            "Need SMS",
        )
        .await;
        let state = auth.get_state().await;
        assert_eq!(state.status, AuthStatus::HumanActionRequired);
        assert_eq!(state.active_human_action_id.as_deref(), Some("ha_123"));

        let session = AuthSession {
            session_id: "sess_1".to_string(),
            cookies: vec!["cookie=abc".to_string()],
            token: None,
            profile: None,
            last_verified_at: Utc::now(),
            expires_at: None,
        };
        auth.set_logged_in(session).await;
        let final_state = auth.get_state().await;
        assert_eq!(final_state.status, AuthStatus::LoggedIn);
        assert_eq!(final_state.attempt_count, 0);
        assert!(final_state.session.is_some());
    }
}
