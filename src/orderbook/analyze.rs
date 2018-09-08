use ndarray;

use exchange::Asset;

struct SnapshotAnalyze {
    market: Asset,
    asset: Asset,

    bids: ndarray::Array<f32, f32>,
    bids_T: ndarray::Array<f32, f32>,
    asks: ndarray::Array<f32, f32>,
    asks_T: ndarray::Array<f32, f32>,

    best_bid: f32,
    best_ask: f32,

}

impl SnapshotAnalyze {
    ///

    pub fn bid_ask_spread(&self) -> f32 {
        self.best_ask - self.best_bid
    }
    pub fn mid_price(&self) -> f32 {
        (self.best_ask + self.best_bid) / 2.0
    }
    pub fn bid_relative_price(&self, price: f32) -> f32 {
        self.best_bid - price
    }
    pub fn ask_relative_price(&self, price: f32) -> f32 {
        price - self.best_ask
    }
    pub fn bid_depth(&self, _price: f32) -> f32 {
        0.0
    }
}