use crate::orderbook::Orderbook;

struct Gimme;

impl Gimme {
    fn evaluate(orderbook: &Orderbook) {
        if let (Some(y), Some(n)) = (orderbook.yes_ask(), orderbook.no_ask()) {
            let side = if y.gt(&n) { "yes" } else { "no" };
        }
    }
}
