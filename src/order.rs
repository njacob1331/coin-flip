use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Order {
    symbol: String,
    side: String,
    #[serde(rename = "type")]
    order_type: String,
    time_in_force: String,
    price: Option<String>,
    stop_price: Option<String>,
    quantity: String,
    client_order_id: Option<String>,
    event_outcome: String,
}
