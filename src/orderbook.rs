use dashmap::DashMap;
use kanal::{AsyncReceiver, AsyncSender};
use rust_decimal::Decimal;
use tokio::task::JoinHandle;

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    market_state::MarketState,
    utils::fmt_opt,
    ws::message::{BookTicker, DepthUpdate, Message},
};

pub enum Sequence {
    Valid,
    Stale,
    Gap,
}

#[derive(Debug)]
pub struct Orderbook {
    pub bids: BTreeMap<Decimal, Decimal>,
    pub asks: BTreeMap<Decimal, Decimal>,
    pub last_sequence_id: u64,
}

impl Orderbook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_sequence_id: 0,
        }
    }

    pub fn check_sequence(&self, first_id: u64, last_id: u64) -> Sequence {
        if self.last_sequence_id == 0 {
            return Sequence::Valid;
        }

        if last_id <= self.last_sequence_id {
            return Sequence::Stale;
        }

        if first_id > self.last_sequence_id + 1 {
            return Sequence::Gap;
        }

        Sequence::Valid
    }

    pub fn update(&mut self, update: DepthUpdate) {
        self.last_sequence_id = update.last_update_id;

        // 2. Apply Bids
        for (price, qty) in update.bids {
            Self::apply_level(&mut self.bids, price, qty);
        }

        // 3. Apply Asks
        for (price, qty) in update.asks {
            Self::apply_level(&mut self.asks, price, qty);
        }
    }

    // Helper to consolidate logic and ensure negative quantities aren't stored
    fn apply_level(levels: &mut BTreeMap<Decimal, Decimal>, price: Decimal, qty: Decimal) {
        if qty.is_zero() || qty.is_sign_negative() {
            levels.remove(&price);
        } else {
            levels.insert(price, qty);
        }
    }

    pub fn is_crossed(&self) -> bool {
        if let (Some(bid), Some(ask)) = (self.yes_bid(), self.yes_ask()) {
            return bid >= ask;
        }

        false
    }

    // --- Price Accessors ---

    pub fn yes_bid(&self) -> Option<Decimal> {
        self.bids.last_key_value().map(|(p, _)| *p)
    }

    pub fn yes_ask(&self) -> Option<Decimal> {
        self.asks.first_key_value().map(|(p, _)| *p)
    }

    /// The "No" bid price is (1 - Best Yes Ask)
    pub fn no_bid(&self) -> Option<Decimal> {
        self.yes_ask().map(|p| Decimal::from(1) - p)
    }

    /// The "No" ask price is (1 - Best Yes Bid)
    pub fn no_ask(&self) -> Option<Decimal> {
        self.yes_bid().map(|p| Decimal::from(1) - p)
    }

    pub fn spread(&self) -> Option<Decimal> {
        match (self.yes_bid(), self.yes_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }
}

// pub struct BookKeeper;

// impl BookKeeper {
//     pub fn spawn_book_keeper(
//         orderbooks: Arc<DashMap<String, Orderbook>>,
//         feed_rx: AsyncReceiver<Message>,
//         resub_tx: AsyncSender<String>,
//     ) -> JoinHandle<()> {
//         tokio::spawn(async move {
//             while let Ok(msg) = feed_rx.recv().await {
//                 let Message::DepthUpdate(update) = msg else {
//                     continue;
//                 };

//                 let symbol = update.symbol.clone();
//                 let mut corrupted = false;

//                 {
//                     let mut book = orderbooks
//                         .entry(symbol.clone())
//                         .or_insert_with(Orderbook::new);

//                     match book.check_sequence(update.first_update_id, update.last_update_id) {
//                         Sequence::Valid => {
//                             book.update(update);

//                             if book.is_crossed() {
//                                 eprintln!("Book for {symbol} is crossed.");
//                                 corrupted = true;
//                             } else {
//                                 // Only print if healthy
//                                 println!(
//                                     "[{}] YES ASK: {} - NO ASK: {}",
//                                     symbol,
//                                     fmt_opt(book.yes_ask().as_ref()),
//                                     fmt_opt(book.no_ask().as_ref())
//                                 );
//                             }
//                         }
//                         Sequence::Gap => {
//                             eprintln!("Sequence gap for {symbol}.");
//                             corrupted = true;
//                         }
//                         _ => {} // Stale data, do nothing
//                     }
//                 } // 'book' borrow is dropped here!

//                 if corrupted {
//                     orderbooks.remove(&symbol);
//                     let _ = resub_tx.try_send(symbol);
//                 }
//             }
//         })
//     }
// }

pub struct BookKeeper;

impl BookKeeper {
    pub fn spawn_book_keeper(
        market_state: Arc<MarketState>,
        feed_rx: AsyncReceiver<DepthUpdate>,
        resub_tx: AsyncSender<String>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Ok(update) = feed_rx.recv().await {
                let symbol = update.symbol.clone().to_uppercase();

                let corrupted = {
                    let Some(mut market) = market_state.markets.get_mut(&symbol) else {
                        continue;
                    };

                    // Access through market directly — no rebinding
                    match market
                        .orderbook
                        .check_sequence(update.first_update_id, update.last_update_id)
                    {
                        Sequence::Valid => {
                            market.orderbook.update(update);
                            if market.orderbook.is_crossed() {
                                eprintln!("Book for {symbol} is crossed.");
                                true
                            } else {
                                println!(
                                    "[{}] YES ASK: {} - NO ASK: {}",
                                    symbol,
                                    fmt_opt(market.orderbook.yes_ask().as_ref()),
                                    fmt_opt(market.orderbook.no_ask().as_ref())
                                );
                                false
                            }
                        }
                        Sequence::Gap => {
                            eprintln!("Sequence gap for {symbol}.");
                            true
                        }
                        _ => false, // Stale, do nothing
                    }
                };

                if corrupted {
                    let _ = resub_tx.send(symbol).await;
                }
            }
        })
    }
}
