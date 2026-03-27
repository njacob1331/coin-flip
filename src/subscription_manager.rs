// to-do
// seperate api refresh call into its own task
// SubscriptionManager becomes a consumer of messages from the api refresher

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use kanal::{AsyncReceiver, AsyncSender};
use serde_json::json;
use tokio::task::JoinHandle;

use crate::{
    api::{client::ApiClient, response::Contract},
    ws::message::Subscription,
};

enum QueueItem {
    Subscribe { contract: Contract, ts: Instant },
    Unsubscribe { symbol: String, ts: Instant },
}

impl QueueItem {
    fn symbol(&self) -> &str {
        match self {
            Self::Subscribe { contract, .. } => &contract.instrument_symbol,
            Self::Unsubscribe { symbol, .. } => &symbol,
        }
    }

    fn ts(&self) -> &Instant {
        match self {
            QueueItem::Subscribe { ts, .. } => ts,
            QueueItem::Unsubscribe { ts, .. } => ts,
        }
    }

    fn is_stale(&self) -> bool {
        self.ts().elapsed() > Duration::from_secs(15)
    }
}

pub struct SubscriptionManager {
    client: Arc<ApiClient>,
    ws_tx: AsyncSender<String>,
    resub_rx: AsyncReceiver<String>,
    ack_rx: AsyncReceiver<Subscription>,
    queue: HashMap<u32, QueueItem>,
    subscriptions: HashMap<String, Contract>,
    next_id: u32,
}

impl SubscriptionManager {
    pub fn new(
        client: Arc<ApiClient>,
        ws_tx: AsyncSender<String>,
        resub_rx: AsyncReceiver<String>,
        ack_rx: AsyncReceiver<Subscription>,
    ) -> Self {
        Self {
            client,
            ws_tx,
            resub_rx,
            ack_rx,
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

    async fn send_ws_request(&mut self, method: &str, symbol: &str) -> Result<u32> {
        println!("requesting '{method}' for: {symbol}");

        let id = self.next_id();
        let stream = format!("{symbol}@depth@100ms");
        let request = json!({
            "id": id,
            "method": method,
            "params": [stream],
        })
        .to_string();

        self.ws_tx.send(request).await?;

        Ok(id)
    }

    async fn subscribe(&mut self, contract: Contract) -> Result<()> {
        match self
            .send_ws_request("subscribe", &contract.instrument_symbol)
            .await
        {
            Ok(id) => {
                self.queue.insert(
                    id,
                    QueueItem::Subscribe {
                        contract,
                        ts: Instant::now(),
                    },
                );
            }
            Err(e) => {
                eprintln!("failed to subscribe to {}: {e}", contract.instrument_symbol);
                return Err(e);
            }
        }

        Ok(())
    }

    async fn unsubscribe(&mut self, symbol: &str) -> Result<()> {
        match self.send_ws_request("unsubscribe", symbol).await {
            Ok(id) => {
                self.queue.insert(
                    id,
                    QueueItem::Unsubscribe {
                        symbol: symbol.to_string(),
                        ts: Instant::now(),
                    },
                );
            }
            Err(e) => {
                eprintln!("failed to un-subscribe from {}: {e}", symbol);
                return Err(e);
            }
        }

        Ok(())
    }

    pub fn in_flight(&self, symbol: &str) -> bool {
        self.queue.values().any(|v| v.symbol() == symbol)
    }

    pub async fn refresh(&mut self) -> Result<()> {
        self.queue.extract_if(|_, v| v.is_stale()).for_each(drop);

        let latest: Vec<Contract> = self
            .client
            .list_prediction_market_events()
            .await?
            .into_iter()
            .flat_map(|e| e.contracts)
            .filter(|c| c.is_active())
            .collect();

        let latest_symbols: HashSet<String> =
            latest.iter().map(|c| c.instrument_symbol.clone()).collect();

        for contract in latest {
            if !self.subscriptions.contains_key(&contract.instrument_symbol)
                && !self.in_flight(&contract.instrument_symbol)
            {
                let _ = self.subscribe(contract).await;
            }
        }

        let stale_symbols: Vec<String> = self
            .subscriptions
            .keys()
            .filter(|symbol| !latest_symbols.contains(symbol.as_str()))
            .cloned()
            .collect();

        for symbol in stale_symbols {
            if !self.in_flight(&symbol) {
                let _ = self.unsubscribe(&symbol).await;
            }
        }

        Ok(())
    }

    fn manage_queue(&mut self, subscription: Subscription) {
        let Some(item) = self.queue.remove(&subscription.id) else {
            eprintln!("subscription with id '{}' not in queue", &subscription.id);
            return;
        };

        if subscription.is_err() {
            eprintln!("subscription error for '{}'", item.symbol());
            return;
        }

        match item {
            QueueItem::Subscribe { contract, .. } => self
                .subscriptions
                .insert(contract.instrument_symbol.clone(), contract),
            QueueItem::Unsubscribe { symbol, .. } => self.subscriptions.remove(&symbol),
        };
    }

    pub fn spawn(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    biased;

                    Ok(sub) = self.ack_rx.recv() => {
                        self.manage_queue(sub);
                    }

                    _ = interval.tick() => {
                        if let Err(e) = self.refresh().await {
                            eprintln!("refresh error: {e}");
                        }
                    }
                }
            }
        })
    }
}
