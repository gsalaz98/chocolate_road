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
//!
//! # Environment Variables
//! `AWS_ACCESS_KEY_ID`: AWS Access Key
//! `AWS_SECRET_ACCESS_KEY`: AWS Access Key Secret.
//! `S3_UPLOAD`: Determines whether we upload to s3. "true" and "false" are valid values. Defaults to "true"
//! `S3_BUCKET`: Amazon S3 Bucket to upload to. Defaults to "cuteq"
//! `S3_STORAGE_CLASS`: Amazon S3 Storage class type. Defaults to "STANDARD_IA"
//! `UPLOAD_PERIOD`: Sets the amount of time in seconds we should wait before dumping the
//!     tectonicdb database and uploading it. Defaults to 86400 seconds (one day)
//! `REDIS_AUTH`: Redis password.
//! `DTF_DB_PATH`: TectonicDB Database where files are written to. Defaults to `$HOME/tectonicdb/target/release/db`

#![deny(missing_docs)]
#![feature(custom_attribute)]
#![feature(vec_remove_item)]
#![feature(nll)]

extern crate chrono;
extern crate futures;
extern crate ndarray;
extern crate rayon;
extern crate redis;
extern crate reqwest;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate serde_json;
extern crate strum;
extern crate tar;
extern crate url;
extern crate ws;
extern crate xz2;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate strum_macros;

/// Exchanges and exchange-related methods and modules
pub mod exchange;
/// Methods to listen on redis/ZeroMQ sockets.
pub mod listener;
/// Handles uploading DTF compressed archives to the cloud
pub mod uploader;
/// Orderbook analytics and state management data structures
pub mod orderbook;
/// Unit tests for various parts of this project
pub mod tests;

use std::env;
use std::thread;

use exchange::{Asset, AssetExchange, bitmex, gdax_l2};
use orderbook::tectonic;

fn main() {
    // Redis client is setup here so that we can provide it a host, password, and database
    let r = redis::Client::open("redis://127.0.0.1:6379/0").unwrap();
    // TODO: Consider moving this to the `redis_init` function?
    let r_password = match env::var_os("REDIS_AUTH") {
        Some(password) => Some(password.into_string().unwrap()),
        None => None
    };

    // Begin connection setup to exchange websockets
    // =====================================================

    let mut bitmex_settings = *bitmex::WSExchange::default_settings().unwrap();
    bitmex_settings.metadata.asset_pair = Some(vec![
        [Asset::BTC, Asset::USD],]);
    bitmex_settings.r = r.clone();
    bitmex_settings.r_password = r_password.as_ref().cloned();

    let mut gdax_settings = *gdax_l2::WSExchange::default_settings().unwrap();
    gdax_settings.metadata.asset_pair = Some(vec![
        [Asset::BTC, Asset::USD],
        [Asset::ETH, Asset::USD],
        [Asset::LTC, Asset::USD],
        [Asset::BTC, Asset::USDC],
    ]);
    gdax_settings.r = r.clone();
    gdax_settings.r_password = r_password.as_ref().cloned();

    // =====================================================

    let mut exchanges = vec![];

    // Push exchange instance threads to vector
    exchanges.push(thread::spawn(move ||
        bitmex::WSExchange::run(Some(&bitmex_settings))));

    exchanges.push(thread::spawn(move ||
        gdax_l2::WSExchange::run(Some(&gdax_settings))));

    // Start a listener to insert ticks into tectonicdb
    exchanges.push(thread::spawn(move ||
        listener::redis_listen_and_insert(
            &r,
            r_password,
            &mut tectonic::TectonicConnection::new(None, None)
                .expect("Failed to connect to TectonicDB"))));

    for exchange in exchanges {
        let _ = exchange.join();
    }
}
