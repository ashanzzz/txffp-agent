use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub phone: Option<String>,
}

// Custom Debug implementation to prevent passwords from ever leaking to logs!
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"***REDACTED***")
            .field("phone", &self.phone)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStatus {
    pub username_configured: bool,
    pub password_configured: bool,
    pub phone_configured: bool,
    pub source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("Deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("Credentials not found")]
    NotFound,
}

#[async_trait]
pub trait CredentialProvider: Send + Sync {
    async fn get_credentials(&self) -> Option<Credentials>;
    async fn has_credentials(&self) -> bool;
    async fn get_status(&self) -> CredentialStatus;
    async fn update_credentials(&self, creds: Credentials) -> Result<(), CredentialError>;
    async fn delete_credentials(&self) -> Result<(), CredentialError>;
}

#[derive(Debug, Serialize, Deserialize)]
struct SecretsToml {
    username: Option<String>,
    password: Option<String>,
    phone: Option<String>,
}

pub struct FileAndEnvCredentialProvider {
    secrets_path: PathBuf,
    cached: Arc<RwLock<Option<Credentials>>>,
}

impl FileAndEnvCredentialProvider {
    pub fn new(secrets_path: PathBuf) -> Self {
        Self {
            secrets_path,
            cached: Arc::new(RwLock::new(None)),
        }
    }

    async fn read_from_file(&self) -> Option<Credentials> {
        if !self.secrets_path.exists() {
            return None;
        }

        let content = tokio::fs::read_to_string(&self.secrets_path).await.ok()?;
        let parsed: SecretsToml = toml::from_str(&content).ok()?;

        if let (Some(username), Some(password)) = (parsed.username, parsed.password) {
            if !username.trim().is_empty() && !password.trim().is_empty() {
                return Some(Credentials {
                    username,
                    password,
                    phone: parsed.phone,
                });
            }
        }
        None
    }

    fn read_from_env() -> Option<Credentials> {
        let username = std::env::var("TXFFP_USERNAME").ok()?;
        let password = std::env::var("TXFFP_PASSWORD").ok()?;
        let phone = std::env::var("TXFFP_PHONE").ok();

        if !username.trim().is_empty() && !password.trim().is_empty() {
            Some(Credentials {
                username,
                password,
                phone,
            })
        } else {
            None
        }
    }
}

#[async_trait]
impl CredentialProvider for FileAndEnvCredentialProvider {
    async fn get_credentials(&self) -> Option<Credentials> {
        // 1. Check in-memory cache
        {
            let lock = self.cached.read().await;
            if let Some(ref c) = *lock {
                return Some(c.clone());
            }
        }

        // 2. Check local secret file (/data/secrets.toml)
        if let Some(creds) = self.read_from_file().await {
            let mut lock = self.cached.write().await;
            *lock = Some(creds.clone());
            return Some(creds);
        }

        // 3. Fallback to environment variables
        if let Some(creds) = Self::read_from_env() {
            let mut lock = self.cached.write().await;
            *lock = Some(creds.clone());
            return Some(creds);
        }

        None
    }

    async fn has_credentials(&self) -> bool {
        self.get_credentials().await.is_some()
    }

    async fn get_status(&self) -> CredentialStatus {
        if let Some(creds) = self.get_credentials().await {
            let source = if self.secrets_path.exists() {
                "secrets_file".to_string()
            } else {
                "environment".to_string()
            };

            CredentialStatus {
                username_configured: !creds.username.is_empty(),
                password_configured: !creds.password.is_empty(),
                phone_configured: creds.phone.is_some(),
                source,
            }
        } else {
            CredentialStatus {
                username_configured: false,
                password_configured: false,
                phone_configured: false,
                source: "none".to_string(),
            }
        }
    }

    async fn update_credentials(&self, creds: Credentials) -> Result<(), CredentialError> {
        let toml_data = SecretsToml {
            username: Some(creds.username.clone()),
            password: Some(creds.password.clone()),
            phone: creds.phone.clone(),
        };

        let content = toml::to_string_pretty(&toml_data)?;

        if let Some(parent) = self.secrets_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&self.secrets_path, content).await?;

        let mut lock = self.cached.write().await;
        *lock = Some(creds);
        Ok(())
    }

    async fn delete_credentials(&self) -> Result<(), CredentialError> {
        if self.secrets_path.exists() {
            tokio::fs::remove_file(&self.secrets_path).await?;
        }
        let mut lock = self.cached.write().await;
        *lock = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_debug_redaction() {
        let creds = Credentials {
            username: "testuser".to_string(),
            password: "supersecretpassword".to_string(),
            phone: Some("13800000000".to_string()),
        };

        let debug_str = format!("{:?}", creds);
        assert!(debug_str.contains("testuser"));
        assert!(debug_str.contains("***REDACTED***"));
        assert!(!debug_str.contains("supersecretpassword"));
    }

    #[tokio::test]
    async fn test_file_credential_provider_crud() {
        let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let secrets_path = temp_dir.join("secrets.toml");

        let provider = FileAndEnvCredentialProvider::new(secrets_path.clone());

        assert!(!provider.has_credentials().await);

        let creds = Credentials {
            username: "myuser".to_string(),
            password: "mypassword".to_string(),
            phone: None,
        };

        provider.update_credentials(creds.clone()).await.unwrap();
        assert!(provider.has_credentials().await);

        let retrieved = provider.get_credentials().await.unwrap();
        assert_eq!(retrieved.username, "myuser");
        assert_eq!(retrieved.password, "mypassword");

        provider.delete_credentials().await.unwrap();
        assert!(!provider.has_credentials().await);

        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
