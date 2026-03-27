use std::sync::Arc;

use dashmap::DashMap;

use crate::{api::response::Contract, orderbook::Orderbook};

#[derive(Debug)]
pub struct Market {
    pub contract: Contract,
    pub orderbook: Orderbook,
}

impl Market {
    pub fn contract(&self) -> &Contract {
        &self.contract
    }

    pub fn orderbook(&self) -> &Orderbook {
        &self.orderbook
    }
}

impl Market {
    pub fn new(contract: Contract) -> Self {
        Self {
            contract,
            orderbook: Orderbook::new(),
        }
    }
}

#[derive(Debug)]
pub struct MarketState {
    pub markets: DashMap<String, Market>,
}

impl MarketState {
    pub fn new() -> Self {
        Self {
            markets: DashMap::new(),
        }
    }
}
