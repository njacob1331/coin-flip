use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use kanal::{AsyncReceiver, AsyncSender};
use serde_json::json;
use tokio::task::JoinHandle;

use crate::{
    api::{client::ApiClient, response::Contract},
    market_state::{Market, MarketState},
    orderbook::Orderbook,
    ws::message::Subscription,
};

pub struct Tracker {
    client: Arc<ApiClient>,
    ws_tx: AsyncSender<String>,
    resub_rx: AsyncReceiver<String>,
    ack_rx: AsyncReceiver<Subscription>,
    market_state: Arc<MarketState>,
    queue: HashMap<u32, String>,
    subscriptions: HashMap<String, Contract>,
    next_id: u32,
}

impl Tracker {
    pub fn new(
        client: Arc<ApiClient>,
        ws_tx: AsyncSender<String>,
        resub_rx: AsyncReceiver<String>,
        ack_rx: AsyncReceiver<Subscription>,
        market_state: Arc<MarketState>,
    ) -> Self {
        Self {
            client,
            ws_tx,
            resub_rx,
            ack_rx,
            market_state,
            queue: HashMap::new(),
            subscriptions: HashMap::new(),
            next_id: 1,
        }
    }

    fn next_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub async fn refresh(&mut self) -> Result<()> {
        let latest: Vec<Contract> = self
            .client
            .list_prediction_market_events()
            .await?
            .into_iter()
            .flat_map(|e| e.contracts)
            .collect();

        let subscribe: HashSet<&str> = latest
            .iter()
            .map(|c| c.instrument_symbol.as_str())
            .collect();

        let unsubscribe: Vec<String> = self.subscriptions.keys().cloned().collect();

        for unsub in unsubscribe {
            let _ = self.send_unsubscribe(unsub).await;
        }

        for sub in subscribe {
            let _ = self.send_subscribe(symbol, contract)
        }

        Ok(())
    }

    async fn send_subscribe(&mut self, symbol: String, contract: Contract) -> Result<()> {
        let id = self.next_id();
        let topic = format!("{symbol}@depth@100ms");

        let request = json!({
            "id": id,
            "method": "subscribe",
            "params": [topic],
        })
        .to_string();

        println!("requesting subscription to: {symbol}");
        self.queue.insert(id, symbol.clone());

        if let Err(e) = self.ws_tx.send(request).await {
            self.queue.remove(&id);
        }

        Ok(())
    }

    async fn send_unsubscribe(&mut self, symbol: String) -> Result<()> {
        let id = self.next_id();
        let topic = format!("{symbol}@depth@100ms");

        let request = json!({
            "id": id,
            "method": "unsubscribe",
            "params": [topic],
        })
        .to_string();

        self.pending.insert(
            id,
            PendingAction::Unsubscribe {
                symbol: symbol.clone(),
            },
        );

        println!("requesting un-subscription from: {symbol}");
        self.states.insert(symbol, SubState::Unsubscribing);

        self.ws_tx.send(request).await?;
        Ok(())
    }

    fn handle_ack(&mut self, ack: Subscription) {
        let Some(action) = self.pending.remove(&ack.id) else {
            return;
        };

        if let Some(error) = ack.error {
            eprintln!("subscription error: {}", error.msg);

            return;
        }

        match action {
            PendingAction::Subscribe { symbol, contract } => {
                self.market_state
                    .markets
                    .insert(symbol.clone(), Market::new(contract));

                println!("subscribed to: {symbol}");
                self.states.insert(symbol, SubState::Subscribed);
            }

            PendingAction::Unsubscribe { symbol } => {
                self.market_state.markets.remove(&symbol);
                println!("un-subscribed from: {symbol}");
                self.states.remove(&symbol);
            }

            PendingAction::Resubscribe { symbol } => {
                println!("re-subscribed to: {symbol}");
                self.states.insert(symbol, SubState::Subscribed);
            }
        }
    }

    async fn handle_resync(&mut self, symbol: String) -> Result<()> {
        let Some(state) = self.states.get(&symbol) else {
            return Ok(());
        };

        if *state == SubState::Resyncing {
            return Ok(()); // dedupe
        }

        // reset orderbook ONLY
        if let Some(mut market) = self.market_state.markets.get_mut(&symbol) {
            market.orderbook = Orderbook::new();
        }

        self.states.insert(symbol.clone(), SubState::Resyncing);

        let id = self.next_id();
        let topic = format!("{symbol}@depth@100ms");

        let request = json!({
            "id": id,
            "method": "subscribe",
            "params": [topic],
        })
        .to_string();

        self.pending.insert(
            id,
            PendingAction::Resubscribe {
                symbol: symbol.clone(),
            },
        );

        self.ws_tx.send(request).await?;
        Ok(())
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = self.refresh().await {
                            eprintln!("refresh error: {e}");
                        }
                    }

                    Ok(symbol) = self.resub_rx.recv() => {
                        if let Err(e) = self.handle_resync(symbol).await {
                            eprintln!("resync error: {e}");
                        }
                    }

                    Ok(ack) = self.ack_rx.recv() => {
                        self.handle_ack(ack);
                    }
                }
            }
        })
    }
}
