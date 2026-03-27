use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rust_decimal::Decimal;
use tokio::task::JoinHandle;

use crate::{market_state::MarketState, orderbook::Orderbook};

pub struct SignalDetector;

impl SignalDetector {
    pub fn spawn_singal_detector(market_state: Arc<MarketState>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(10));
            loop {
                interval.tick().await;

                // If logic gets expensive — snapshot first, compute outside locks
                let snapshot: Vec<(
                    String,
                    Option<DateTime<Utc>>,
                    Option<Decimal>,
                    Option<Decimal>,
                )> = market_state
                    .markets
                    .iter()
                    .map(|e| {
                        (
                            e.key().clone(),
                            e.value().contract.expiry_date,
                            e.value().orderbook.yes_ask(),
                            e.value().orderbook.no_ask(),
                        )
                    })
                    .collect(); // locks released here

                // Now compute with no locks held
                for (symbol, exp, yes, no) in snapshot {
                    if let (Some(yes), Some(no)) = (yes, no) {
                        let sum = yes + no;
                        if sum < Decimal::ONE {
                            println!("ARB DETECTED on {symbol}: {sum}");
                        }
                    }

                    if let (Some(exp), Some(yes), Some(no)) = (exp, yes, no) {
                        let time_left = (exp - Utc::now()).num_seconds();

                        println!("{symbol} almost expired ({time_left}): YES: {yes} - NO: {no} ")
                    }
                }
            }
        })
    }
}
