//! HTTP client for the cloud backend. Blocking esp-idf HTTP with TLS via
//! the global certificate bundle, so both `http://<lan-ip>:3000` (dev)
//! and `https://` (Railway) base URLs work.

use crate::types::{MenuItem, Session, SessionStatus};
use anyhow::{ensure, Context, Result};
use embedded_svc::http::client::Client;
use embedded_svc::io::{Read, Write};
use esp_idf_svc::http::client::{Configuration, EspHttpConnection};
use log::info;

pub struct ApiClient {
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    fn client(&self) -> Result<Client<EspHttpConnection>> {
        let conn = EspHttpConnection::new(&Configuration {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
            timeout: Some(std::time::Duration::from_secs(15)),
            ..Default::default()
        })?;
        Ok(Client::wrap(conn))
    }

    fn read_body(response: &mut impl Read<Error = esp_idf_svc::io::EspIOError>) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = response.read(&mut buf).context("read body")?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&buf[..n]);
        }
        Ok(body)
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let mut client = self.client()?;
        let request = client.get(url).with_context(|| format!("GET {url}"))?;
        let mut response = request.submit().with_context(|| format!("GET {url}"))?;
        let status = response.status();
        ensure!((200..300).contains(&status), "GET {url} -> HTTP {status}");
        let body = Self::read_body(&mut response)?;
        serde_json::from_slice(&body).with_context(|| format!("parse response of GET {url}"))
    }

    pub fn fetch_menu(&self) -> Result<Vec<MenuItem>> {
        let url = format!("{}/api/menu", self.base_url);
        info!("API: GET {url}");
        self.get_json(&url)
    }

    pub fn create_session(&self, item_id: &str) -> Result<Session> {
        let url = format!("{}/api/session", self.base_url);
        info!("API: POST {url} item_id={item_id}");
        let body = serde_json::to_vec(&serde_json::json!({ "item_id": item_id }))?;
        let mut client = self.client()?;
        let mut request = client
            .post(&url, &[("Content-Type", "application/json")])
            .with_context(|| format!("POST {url}"))?;
        request.write_all(&body).context("write body")?;
        request.flush().context("flush body")?;
        let mut response = request.submit().with_context(|| format!("POST {url}"))?;
        let status = response.status();
        ensure!((200..300).contains(&status), "POST {url} -> HTTP {status}");
        let body = Self::read_body(&mut response)?;
        serde_json::from_slice(&body).with_context(|| format!("parse response of POST {url}"))
    }

    pub fn poll_status(&self, session_id: &str) -> Result<SessionStatus> {
        let url = format!("{}/api/session/{}/status", self.base_url, session_id);
        self.get_json(&url)
    }
}
