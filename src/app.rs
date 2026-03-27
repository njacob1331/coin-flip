use std::{ops::Sub, sync::Arc};

use anyhow::Result;

use dashmap::DashMap;
use futures_util::future::join_all;

use tokio_util::sync::CancellationToken;

use crate::{
    api::client::ApiClient,
    market_state::MarketState,
    orderbook::BookKeeper,
    strategy::signal::SignalDetector,
    subscription_manager::Tracker,
    ws::{
        client::WsClient,
        message::{DepthUpdate, Message, Subscription},
    },
};

pub struct App;

impl App {
    pub async fn run() -> Result<()> {
        // let orderbooks = Arc::new(DashMap::new());
        let token = CancellationToken::new();

        // 2. Setup Communication Pipes
        let (ws_tx, ws_rx) = kanal::bounded_async::<String>(64);
        let (feed_tx, feed_rx) = kanal::bounded_async::<DepthUpdate>(2048);
        let (sub_tx, sub_rx) = kanal::bounded_async::<Subscription>(16);
        let (resub_tx, resub_rx) = kanal::bounded_async::<String>(16);

        // 3. Initialize Components
        let (writer, reader) = WsClient::connect().await?;
        let api_client = Arc::new(ApiClient::new());
        let market_state = Arc::new(MarketState::new());
        let tracker = Tracker::new(
            api_client.clone(),
            ws_tx,
            resub_rx,
            sub_rx,
            market_state.clone(),
        );

        // 4. Start Orchestration
        let handles = vec![
            WsClient::spawn_reader(reader, feed_tx, sub_tx),
            WsClient::spawn_writer(writer, ws_rx),
            Tracker::spawn(tracker),
            BookKeeper::spawn_book_keeper(market_state.clone(), feed_rx, resub_tx),
            // SignalDetector::spawn_singal_detector(market_state.clone()),
        ];

        // 5. Wait for Shutdown
        tokio::select! {
            _ = tokio::signal::ctrl_c() => token.cancel(),
            _ = join_all(handles) => {},
        }

        Ok(())
    }
}
