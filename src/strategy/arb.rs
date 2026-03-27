use std::str::FromStr;

use rust_decimal::{Decimal, prelude::Signed};

pub struct Arb;

impl Arb {
    pub fn exists(yes_ask: Decimal, no_ask: Decimal) {
        let profit = Decimal::ONE - (yes_ask + no_ask);

        if profit.is_sign_positive() {
            println!("PROFIT FOUND: {profit}");
        }
    }
}
