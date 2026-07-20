use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub price_usdc: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub payment_url: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionStatus {
    pub status: PaymentStatus,
}
