use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use dotenvy::dotenv;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha384;

type HmacSha384 = Hmac<Sha384>;

#[derive(Debug)]
pub struct Auth {
    key: String,
    secret: String,
}

impl Auth {
    pub fn authenticate() -> Self {
        dotenv().ok();

        Self {
            key: std::env::var("GEMINI_KEY").expect("Missing GEMINI_KEY"),
            secret: std::env::var("GEMINI_SECRET").expect("Missing GEMINI_SECRET"),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn secret(&self) -> &str {
        &self.secret
    }

    pub fn sign(&self, path: &str) -> Result<(String, String)> {
        // 1. Nonce must be an integer (number), not a string
        let nonce = chrono::Utc::now().timestamp_millis();

        // 2. The 'request' MUST be the endpoint path (e.g., "/v1/order/events")
        let payload_json = json!({
            "request": path,
            "nonce": nonce,
        });

        let payload_str = nonce.to_string();
        let payload_b64 = STANDARD.encode(payload_str);

        // 3. Gemini secrets are raw strings; do not base64-decode the secret
        let mut mac = HmacSha384::new_from_slice(self.secret().as_bytes())?;
        mac.update(payload_b64.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        Ok((payload_b64, signature))
    }
}
