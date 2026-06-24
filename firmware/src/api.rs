use crate::types::{MenuItem, Session, SessionStatus};
use log::info;

pub struct ApiClient {
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn fetch_menu(&self) -> anyhow::Result<Vec<MenuItem>> {
        info!("API: fetching menu from {}/api/menu", self.base_url);
        Ok(vec![])
    }

    pub fn create_session(&self, item_id: &str) -> anyhow::Result<Session> {
        info!("API: creating session for item {}", item_id);
        anyhow::bail!("not implemented")
    }

    pub fn poll_status(&self, session_id: &str) -> anyhow::Result<SessionStatus> {
        info!("API: polling status for session {}", session_id);
        anyhow::bail!("not implemented")
    }
}
