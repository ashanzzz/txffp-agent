use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub data_dir: PathBuf,
    pub secrets_path: PathBuf,
    pub auth_mode: AuthMode,
    pub auto_login: bool,
    pub max_auth_attempts: u32,
    pub steel_base_url: String,
    pub cdp_base_url: String,
    pub scrapling_base_url: String,
    pub scrapling_mcp_auth_token: Option<String>,
    pub research_mode: bool,
    pub api_token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    Auto,
    Manual,
    #[default]
    Hybrid,
}

impl AppConfig {
    pub fn load() -> Self {
        let _ = dotenvy::dotenv();

        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8000);

        let data_dir_str = std::env::var("DATA_DIR").unwrap_or_else(|_| {
            if std::path::Path::new("/data").exists() {
                "/data".to_string()
            } else {
                "./data".to_string()
            }
        });
        let data_dir = PathBuf::from(data_dir_str);

        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            let db_path = data_dir.join("txffp.db");
            format!("sqlite://{}", db_path.display())
        });

        let secrets_path = std::env::var("SECRETS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| data_dir.join("secrets.toml"));

        let auth_mode = std::env::var("TXFFP_AUTH_MODE")
            .ok()
            .and_then(|m| match m.to_lowercase().as_str() {
                "auto" => Some(AuthMode::Auto),
                "manual" => Some(AuthMode::Manual),
                "hybrid" => Some(AuthMode::Hybrid),
                _ => None,
            })
            .unwrap_or_default();

        let auto_login = std::env::var("TXFFP_AUTO_LOGIN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let max_auth_attempts = std::env::var("TXFFP_MAX_AUTH_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3)
            .clamp(1, 5);

        let steel_base_url = std::env::var("STEEL_BASE_URL")
            .unwrap_or_else(|_| "http://192.168.8.11:13000".to_string());

        let cdp_base_url = std::env::var("CDP_BASE_URL").unwrap_or_else(|_| {
            if steel_base_url.contains(":13000") {
                steel_base_url.replace(":13000", ":19223")
            } else if steel_base_url.contains(":3000") {
                steel_base_url.replace(":3000", ":9223")
            } else {
                "http://192.168.8.11:19223".to_string()
            }
        });

        let scrapling_base_url = std::env::var("SCRAPLING_BASE_URL")
            .unwrap_or_else(|_| "http://192.168.8.11:8111".to_string());

        let scrapling_mcp_auth_token = std::env::var("SCRAPLING_MCP_AUTH_TOKEN").ok();

        let research_mode = std::env::var("TXFFP_RESEARCH_MODE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        let api_token = std::env::var("TXFFP_API_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty());

        Self {
            host,
            port,
            database_url,
            data_dir,
            secrets_path,
            auth_mode,
            auto_login,
            max_auth_attempts,
            steel_base_url,
            cdp_base_url,
            scrapling_base_url,
            scrapling_mcp_auth_token,
            research_mode,
            api_token,
        }
    }
}
