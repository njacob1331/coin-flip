use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use serde_with::{DisplayFromStr, serde_as};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Message {
    Subscription(Subscription),
    DepthUpdate(DepthUpdate),
    BookTicker(BookTicker),
    Unknown(Value),
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionError {
    pub code: i16,
    pub msg: String,
}

// #[derive(Debug, Clone, Deserialize)]
// pub enum Subscription {
//     Ok {
//         id: u32,
//         status: i16,
//     },
//     Error {
//         id: u32,
//         status: i16,
//         error: Option<SubscriptionError>,
//     },
// }

#[derive(Debug, Clone, Deserialize)]
pub struct Subscription {
    pub id: u32,
    pub status: i16,
    pub error: Option<SubscriptionError>,
}

impl Subscription {
    pub fn is_err(&self) -> bool {
        self.error.is_some()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriceLevel {
    price: Decimal,
    quantity: Decimal,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct DepthUpdate {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time_ns: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub last_update_id: u64,
    #[serde(rename = "b")]
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub bids: Vec<(Decimal, Decimal)>,
    #[serde(rename = "a")]
    #[serde_as(as = "Vec<(DisplayFromStr, DisplayFromStr)>")]
    pub asks: Vec<(Decimal, Decimal)>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct BookTicker {
    #[serde(rename = "u")]
    pub update_id: usize,
    #[serde(rename = "E")]
    pub event_time_ns: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub best_bid_price: Option<String>,
    #[serde(rename = "B")]
    pub best_bid_quanitity: Option<String>,
    #[serde(rename = "a")]
    pub best_ask_price: Option<String>,
    #[serde(rename = "A")]
    pub best_ask_quanitity: Option<String>,
}
