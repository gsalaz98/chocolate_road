/// BitMEX exchange module
pub mod bitmex;

use strum::AsStaticRef;

/// Complete list of all the exchanges we support. This is also used as a unique
/// identifier to differentiate where the data originated. Is used in the `orderbook` module.
pub enum Exchange {
    /// Poloniex exchange
    Poloniex,
    /// GDAX exchange
    GDAX,
    /// BitMEX exchange
    BitMEX,
}

impl Exchange {
    /// Useful method to identify how exactly the market/asset pair is constructed.
    /// Some exchanges place the market first (i.e. USD-BTC) whereas others don't (BTC-USD).
    pub fn market_first(&self) -> bool {
        match &self {
            Exchange::Poloniex => true,
            Exchange::GDAX => true,
            Exchange::BitMEX => false,
        }
    }
    /// Returns the separator present in the market/asset pair. Some exchanges don't include
    /// any string or separator, so we represent that with an empty string.
    pub fn asset_separator(&self) -> &'static str {
        match &self {
            Exchange::Poloniex => "-",
            Exchange::GDAX => "-",
            Exchange::BitMEX => "",
        }
    }
}

#[derive(AsStaticStr, Clone)]
/// Assets that are currently supported. We plan on standardizing all token names across multiple exchanges,
/// so having an enum of supported assets is quite... the asset ᕕ( ᐛ )ᕗ
pub enum Asset {
    /// Bitcoin
    BTC = 0,
    /// Ethereum
    ETH,
    /// Litecoin
    LTC,
}

/// Helper function that takes in the assets you want to trade as a `MARKET, ASSET` vector pair.
/// Depending on the exchange and whether the exchange chooses to flip around these values, we
/// format it according to the exchange's configuration
pub fn get_asset_pair(assets: &Vec<Asset>, exch: Exchange) -> String {
    match exch.market_first() {
        true => {
            let mut pair = String::with_capacity(9);
            pair.push_str(assets[0].as_static());
            pair.push_str(exch.asset_separator());
            pair.push_str(assets[1].as_static());

            pair
        },
        false => {
            let mut pair = String::with_capacity(9);
            pair.push_str(assets[1].as_static());
            pair.push_str(exch.asset_separator());
            pair.push_str(assets[1].as_static());

            pair
        }
    }
}

/// Same as function `get_asset_pair`, but with the added benefit of batch processing.
pub fn get_batch_asset_pairs(assets: &Vec<[Asset; 2]>, exch: Exchange) -> Vec<String> {
    assets.into_iter().map(|asset_pair| {
        match exch.market_first() {
            true => {
                let mut pair = String::with_capacity(9);
                pair.push_str(asset_pair[0].as_static());
                pair.push_str(exch.asset_separator());
                pair.push_str(asset_pair[1].as_static());

                pair
            },
            false => {
                let mut pair = String::with_capacity(9);
                pair.push_str(asset_pair[1].as_static());
                pair.push_str(exch.asset_separator());
                pair.push_str(asset_pair[0].as_static());

                pair
            }
        }
    }).collect::<Vec<_>>()
}