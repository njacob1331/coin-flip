use anyhow::Result;

use crate::app::App;

mod api;
mod app;
mod market_state;
mod order;
mod orderbook;
mod strategy;
mod subscription_manager;
mod tracker;
mod utils;
mod ws;

#[tokio::main]
async fn main() -> Result<()> {
    App::run().await?;

    Ok(())
}
