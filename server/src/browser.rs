use serde::{Deserialize, Serialize};

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

#[derive(Debug, thiserror::Error)]
pub enum BrowserError {
    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Steel API error: {0}")]
    Api(String),
}

#[derive(Clone)]
pub struct SteelBrowserDriver {
    base_url: String,
    client: reqwest::Client,
}

impl SteelBrowserDriver {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn check_health(&self) -> bool {
        let url = format!("{}/v1/sessions", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn create_session(&self) -> Result<SteelSession, BrowserError> {
        let url = format!("{}/v1/sessions", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "dimensions": { "width": 1920, "height": 1080 }
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(BrowserError::Api(format!(
                "Failed to create Steel session: {}",
                text
            )));
        }

        let session: SteelSession = res.json().await?;
        Ok(session)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<SteelSession, BrowserError> {
        let url = format!("{}/v1/sessions/{}", self.base_url, session_id);
        let res = self.client.get(&url).send().await?;

        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(BrowserError::Api(format!(
                "Failed to get Steel session: {}",
                text
            )));
        }

        let session: SteelSession = res.json().await?;
        Ok(session)
    }

    pub async fn release_session(&self, session_id: &str) -> Result<(), BrowserError> {
        let url = format!("{}/v1/sessions/{}/release", self.base_url, session_id);
        let _ = self.client.post(&url).send().await;
        Ok(())
    }
}
