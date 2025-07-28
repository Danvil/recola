use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Symbol of a market product
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol(pub u64);

/// Credits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Creds(pub Decimal);

/// Price of an item last time it was checked
pub struct LastCheckedPrice(pub Decimal);

#[derive(Serialize, Deserialize)]
pub struct Market {
    book: HashMap<Symbol, MarketListing>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    pub symbol: Symbol,
    pub price: Creds,
    pub quantity: u32,
    pub side: OrderSide,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Default, Serialize, Deserialize)]
pub struct MarketListing {
    orders: Vec<Order>,
}

impl MarketListing {
    pub fn create_order(&mut self, _order: Order) {
        // TODO
    }
}

impl Market {
    pub fn new() -> Self {
        Self {
            book: HashMap::new(),
        }
    }

    pub fn create_order(&mut self, order: Order) {
        self.book
            .entry(order.symbol)
            .or_default()
            .create_order(order);
    }
}
