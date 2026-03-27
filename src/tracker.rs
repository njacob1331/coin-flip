// use std::{
//     collections::{BTreeSet, HashMap, HashSet, hash_map::Entry},
//     sync::Arc,
//     time::Duration,
// };

// use anyhow::Result;
// use dashmap::DashMap;
// use kanal::{AsyncReceiver, AsyncSender};
// use serde_json::json;
// use tokio::task::JoinHandle;

// use crate::{
//     api::{client::ApiClient, response::Contract},
//     market_state::{Market, MarketState},
//     orderbook::Orderbook,
//     ws::message::Subscription,
// };

// // #[derive(Debug)]
// // pub struct Tracker {
// //     tracking: HashMap<String, Contract>,
// //     client: Arc<ApiClient>,
// //     ws_tx: AsyncSender<String>,
// //     orderbooks: Arc<DashMap<String, Orderbook>>,
// // }

// // impl Tracker {
// //     pub fn new(
// //         client: Arc<ApiClient>,
// //         ws_tx: AsyncSender<String>,
// //         orderbooks: Arc<DashMap<String, Orderbook>>,
// //     ) -> Self {
// //         Self {
// //             tracking: HashMap::new(),
// //             client,
// //             ws_tx,
// //             orderbooks,
// //         }
// //     }

// //     pub async fn refresh(&mut self) -> Result<()> {
// //         let latest_active: HashMap<String, Contract> = self
// //             .client
// //             .list_prediction_market_events()
// //             .await?
// //             .into_iter()
// //             .flat_map(|e| e.contracts)
// //             .filter(|c| c.status == "active")
// //             .map(|c| (c.instrument_symbol.clone(), c))
// //             .collect();

// //         let to_remove: Vec<String> = self
// //             .tracking
// //             .keys()
// //             .filter(|sym| !latest_active.contains_key(*sym))
// //             .cloned()
// //             .collect();

// //         for symbol in to_remove {
// //             if self.unsubscribe(&symbol).await.is_ok() {
// //                 self.orderbooks.remove(&symbol);
// //                 self.tracking.remove(&symbol);
// //             }
// //         }

// //         for (symbol, contract) in latest_active {
// //             if !self.tracking.contains_key(&symbol) {
// //                 if self.subscribe(&symbol).await.is_ok() {
// //                     self.tracking.insert(symbol, contract);
// //                 }
// //             }
// //         }

// //         Ok(())
// //     }

// //     pub async fn resubscribe(&self, to: &str) -> Result<()> {
// //         println!("re-subbing: {:?}", to);
// //         let to = format!("{to}@depth@100ms");
// //         self.send_request("unsubscribe", &to).await?;
// //         self.send_request("subscribe", &to).await?;

// //         Ok(())
// //     }

// //     pub async fn subscribe(&self, to: &str) -> Result<()> {
// //         println!("subscribing to: {:?}", to);
// //         let to = format!("{to}@depth@100ms");
// //         self.send_request("subscribe", &to).await
// //     }

// //     pub async fn unsubscribe(&self, from: &str) -> Result<()> {
// //         println!("un-subscribing from: {:?}", from);
// //         let from = format!("{from}@depth@100ms");
// //         self.send_request("unsubscribe", &from).await
// //     }

// //     async fn send_request(&self, method: &str, params: &str) -> Result<()> {
// //         let request = json!({
// //             "method": method,
// //             "params": [params],
// //         })
// //         .to_string();

// //         self.ws_tx.send(request).await?;

// //         Ok(())
// //     }

// //     pub fn spawn_tracker(
// //         orderbooks: Arc<DashMap<String, Orderbook>>,
// //         client: Arc<ApiClient>,
// //         ws_tx: AsyncSender<String>,
// //         resub_rx: AsyncReceiver<String>,
// //     ) -> JoinHandle<()> {
// //         let mut tracker = Tracker::new(client, ws_tx, orderbooks);

// //         tokio::spawn(async move {
// //             let mut interval = tokio::time::interval(Duration::from_secs(60));

// //             loop {
// //                 tokio::select! {
// //                     _ = interval.tick() => {
// //                         println!("Tracker: Performing scheduled refresh...");
// //                         if let Err(e) = tracker.refresh().await {
// //                             eprintln!("Tracker refresh error: {e}");
// //                         }
// //                     }

// //                     res = resub_rx.recv() => {
// //                         match res {
// //                             Ok(symbol) => {
// //                                 println!("Tracker: Received reset signal for {symbol}");
// //                                 if let Err(e) = tracker.resubscribe(&symbol).await {
// //                                     eprintln!("Resubscribe failed for {symbol}: {e}");
// //                                 }
// //                             }
// //                             Err(_) => {
// //                                 eprintln!("Tracker: Resub channel closed, exiting loop.");
// //                                 break;
// //                             }
// //                         }
// //                     }
// //                 }
// //             }
// //         })
// //     }
// // }

// #[derive(Debug)]
// enum SubscriptionAction {
//     Subscribe { symbol: String, contract: Contract },
//     Unsubscribe { symbol: String },
// }

// #[derive(Debug)]
// pub struct Tracker {
//     client: Arc<ApiClient>,
//     ws_tx: AsyncSender<String>,
//     resub_rx: AsyncReceiver<String>,
//     market_state: Arc<MarketState>,
//     subscription_queue: HashMap<String, SubscriptionAction>,
// }

// impl Tracker {
//     pub fn new(
//         client: Arc<ApiClient>,
//         ws_tx: AsyncSender<String>,
//         resub_rx: AsyncReceiver<String>,
//         market_state: Arc<MarketState>,
//     ) -> Self {
//         Self {
//             client,
//             ws_tx,
//             resub_rx,
//             market_state,
//             subscription_queue: HashMap::new(),
//         }
//     }

//     pub async fn refresh(&mut self) -> Result<()> {
//         let latest_active: Vec<Contract> = self
//             .client
//             .list_prediction_market_events()
//             .await?
//             .into_iter()
//             .flat_map(|e| e.contracts)
//             .filter(|c| c.status == "active")
//             .collect();

//         let latest_symbols: HashSet<&str> = latest_active
//             .iter()
//             .map(|c| c.instrument_symbol.as_str())
//             .collect();

//         // ✅ Collect keys first, dropping the iter (and all locks) immediately
//         let remove: Vec<String> = self
//             .market_state
//             .markets
//             .iter()
//             .filter(|m| !latest_symbols.contains(m.key().as_str()))
//             .map(|m| m.key().clone())
//             .collect(); // <-- iter dropped here, all shard locks released

//         if !remove.is_empty() {
//             for key in remove {
//                 if self.unsubscribe(&key).await.is_ok() {
//                     self.market_state.markets.remove(&key);
//                 }
//             }
//         }

//         for contract in latest_active {
//             let symbol = contract.instrument_symbol.clone();
//             if !self.market_state.markets.contains_key(&symbol) {
//                 self.market_state
//                     .markets
//                     .insert(symbol.clone(), Market::new(contract));
//                 if self.subscribe(&symbol).await.is_err() {
//                     self.market_state.markets.remove(&symbol);
//                 }
//             }
//         }

//         Ok(())
//     }

//     pub async fn resubscribe(&self, to: &str) -> Result<()> {
//         println!("re-subbing: {:?}", to);
//         let to = format!("{to}@depth@100ms");
//         self.market_state.markets.remove(&to);
//         self.send_request("unsubscribe", &to).await?;
//         self.send_request("subscribe", &to).await?;

//         Ok(())
//     }

//     pub async fn subscribe(&self, to: &str) -> Result<()> {
//         println!("subscribing to: {:?}", to);
//         let to = format!("{to}@depth@100ms");
//         self.send_request("subscribe", &to).await
//     }

//     pub async fn unsubscribe(&self, from: &str) -> Result<()> {
//         println!("un-subscribing from: {:?}", from);
//         let from = format!("{from}@depth@100ms");
//         self.send_request("unsubscribe", &from).await
//     }

//     async fn send_request(&self, method: &str, params: &str) -> Result<()> {
//         let request = json!({
//             "method": method,
//             "params": [params],
//         })
//         .to_string();

//         self.ws_tx.send(request).await?;

//         Ok(())
//     }

//     pub async fn manage_market_state(&self, rx: AsyncReceiver<Subscription>) {
//         while let Ok(sub) = rx.recv().await {
//             let id = sub.id;

//             if let Some(next) = self.subscription_queue.get(&id) {
//                 match next {
//                     SubscriptionAction::Subscribe { symbol, contract } => {
//                         self.market_state
//                             .markets
//                             .insert(symbol.to_owned(), Market::new(*contract));
//                     }
//                     SubscriptionAction::Unsubscribe { symbol } => {
//                         self.market_state.markets.remove(symbol);
//                     }
//                 };
//             }
//         }
//     }

//     pub fn spawn(mut self) -> JoinHandle<()> {
//         tokio::spawn(async move {
//             let mut interval = tokio::time::interval(Duration::from_secs(60));

//             loop {
//                 tokio::select! {
//                     _ = interval.tick() => {
//                         println!("Tracker: Performing scheduled refresh...");
//                         if let Err(e) = self.refresh().await {
//                             eprintln!("Tracker refresh error: {e}");
//                         }
//                     }

//                     res_sub = self.resub_rx.recv() => {
//                         match res_sub {
//                             Ok(symbol) => {
//                                 println!("Tracker: Received reset signal for {symbol}");
//                                 if let Err(e) = self.resubscribe(&symbol).await {
//                                     eprintln!("Resubscribe failed for {symbol}: {e}");
//                                 }
//                             }
//                             Err(_) => {
//                                 eprintln!("Tracker: Resub channel closed, exiting loop.");
//                                 break;
//                             }
//                         }
//                     }
//                 }
//             }
//         })
//     }
// }
