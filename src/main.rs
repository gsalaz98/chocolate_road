//! Orderbook analytics and orderbook event gathering tool. Much akin to
//! [CCXT](https://github.com/ccxt/ccxt), we plan to make this tool three things:
//!     1. Efficient
//!     2. Easy to use
//!     3. Portable across exchanges
//! 
//! The point of this project isn't to make an execution engine (although a fast one would be very nice), but rather
//! gather data for market analysis. We may be able to make use of [LEAN](https://github.com/QuantConnect/LEAN) instead to construct our trading strategies and 
//! plug it in to various exchanges. CCXT may be another option for this as well, as it already has built-in support for lots of crypto exchanges
//! 
//! This project makes use of [TectonicDB](https://github.com/rickyhan/tectonicdb) to store orderbook data
//! in a database efficiently. We also make use of LZMA2 to compress that data further to allow for more data storage.

#![deny(missing_docs)]
#![feature(vec_remove_item)]
#![feature(nll)]

extern crate chrono;
extern crate ndarray;
extern crate rayon;
extern crate redis;
extern crate reqwest;
extern crate serde_json;
extern crate strum;
extern crate url;
extern crate ws;

#[macro_use]
extern crate serde_derive;
#[macro_use] 
extern crate strum_macros;

/// Exchanges and exchange-related methods and modules
pub mod exchange;
/// Orderbook analytics and state management data structures
pub mod orderbook;
/// Unit tests for various parts of this project
pub mod tests;

use std::env;
use std::thread;
use exchange::AssetExchange;
use exchange::{binance, bitmex};

fn main() {
    let r = redis::Client::open("redis://127.0.0.1:6379/0").unwrap();
    let r_password = Some(env::var("REDIS_AUTH").expect("Failed to get REDIS_AUTH environment variable"));
    let mut exchanges = vec![];

    let mut bitmex_settings = *bitmex::WSExchange::default_settings().unwrap();
    bitmex_settings.r = r.clone();
    bitmex_settings.r_password = Some(r_password.as_ref().unwrap().clone());

    exchanges.push(thread::spawn(move || bitmex::WSExchange::run(Some(&bitmex_settings))));

    for exchange in exchanges {
        let _ = exchange.join();
    }
}
