use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GetEventByTicker {
    #[serde(default)]
    pub contracts: Vec<Contract>,
}

#[derive(Debug, Deserialize)]
pub struct ListMarketEvents {
    #[serde(default)]
    pub data: Vec<Event>,
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit: usize,
    pub offset: u64,
    pub total: u64,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub ticker: String,
    pub status: String,
    pub contracts: Vec<Contract>,
}

#[derive(Debug, Deserialize)]
pub struct Contract {
    #[serde(rename = "instrumentSymbol")]
    pub instrument_symbol: String,
    pub status: String,
    #[serde(rename = "expiryDate")]
    pub expiry_date: Option<DateTime<Utc>>,
}

impl Contract {
    pub fn eq(&self, symbol: &str) -> bool {
        self.instrument_symbol == symbol
    }
}
