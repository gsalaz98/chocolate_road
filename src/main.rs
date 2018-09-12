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

extern crate chrono;
extern crate ndarray;
extern crate rayon;
extern crate serde_json;
extern crate strum;
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

fn main() {
}
