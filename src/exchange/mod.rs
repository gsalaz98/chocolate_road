/// BitMEX exchange module
pub mod bitmex;

use strum::AsStaticRef;
use orderbook;

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
    pub fn asset_separator(&self) -> String {
        match &self {
            Exchange::Poloniex => "-".into(),
            Exchange::GDAX => "-".into(),
            Exchange::BitMEX => "".into(),
        }
    }

    /// This function takes the asset, and converts it to its representation on an exchange.
    /// Example: Bitcoin is annotated as `BTC` on Poloniex, but appears as `XBT` in BitMEX.
    pub fn normalize_asset(&self, asset: Asset) -> Option<String> {
        match self {
            Exchange::Poloniex => match asset {
                Asset::BTC => Some("BTC".into()),
                Asset::ETH => Some("ETH".into()),
                Asset::LTC => Some("LTC".into()),
                Asset::USDT => Some("USDT".into()),
                _ => None
            },
            Exchange::GDAX => match asset {
                Asset::BTC => Some("BTC".into()),
                Asset::ETH => Some("ETH".into()),
                Asset::LTC => Some("LTC".into()),
                _ => None
            },
            Exchange::BitMEX => match asset {
                Asset::BTC => Some("XBT".into()),
                Asset::ETH => Some("ETH".into()),
                Asset::LTC => Some("LTC".into()),

                Asset::USD => Some("USD".into()),
                _ => None
            }
        }
    }
    /// Indicates whether or not the exchange supports standard buyer/seller transactions without any sort of contracts.
    /// Normal buy/sell like equities market
    pub fn supports_normal(&self) -> bool {
        match self {
            Exchange::BitMEX => false,
            Exchange::GDAX => true,
            Exchange::Poloniex => true,
        }
    }
    /// Exchanges that support options
    pub fn supports_options(&self) -> bool {
        match self {
            Exchange::BitMEX => true,
            Exchange::GDAX => false,
            Exchange::Poloniex => false,
        }
    }
    /// Exchanges that support futures
    pub fn supports_futures(&self) -> bool {
        match self {
            Exchange::BitMEX => true,
            Exchange::GDAX => false,
            Exchange::Poloniex => false,
        }
    }
}

/// Skeleton methods that we expect all exchanges to implement
pub trait AssetExchange {
    /// Require that each asset exchange we define have defaults
    fn default_settings() -> Self;
    /// Parses the snapshot passed as a generic T type
    fn snapshot<T>(&self, snap: T);
    /// Start and run the websocket data collection
    fn run(settings: Option<&Self>);
}

/// Assets that are currently supported. We plan on standardizing all token names across multiple exchanges,
/// so having an enum of supported assets is quite... the asset ᕕ( ᐛ )ᕗ. We've included fiat as well in here,
/// as they are considered a valid market on many websites
#[derive(AsStaticStr, Clone)]
pub enum Asset {
    /// Bitcoin
    BTC = 0,
    /// Ethereum
    ETH,
    /// Litecoin
    LTC,
    /// Tether
    USDT,

    // FIAT
    //
    /// United States Dollar
    USD,
    /// Japanese Yen
    JPY,
    /// Chinese Yuan
    CNY,
    /// Korean Won
    KRW,
    /// Euro
    EUR,
    /// Great British Pound-Sterling
    GBP,
    /// Canadian Dollar
    CAD,
    /// Australian Dollar
    AUD
}

/// Options are by nature much more different from other assets. For one, very few assets
/// will have options support, so it would make sense to separate the asset classes into two 
/// distinct groups, which is what we've done here.
pub enum OptionsAsset {
    /// Bitcoin options
    BTC = 0,
    /// Ethereum options
    ETH,
}

/// Same reasoning as options. The exclusivity of futures warrants its own group of assets.
pub enum FuturesAsset {
    /// Bitcoin Futures
    BTC = 0,
    /// Ethereum options
    ETH,
}

/// Helper function that takes in the assets you want to trade as a `MARKET, ASSET` vector pair.
/// Depending on the exchange and whether the exchange chooses to flip around these values, we
/// format it according to the exchange's configuration
pub fn get_asset_pair(assets: &Vec<Asset>, exch: Exchange) -> String {
    match exch.market_first() {
        true => {
            let mut pair = String::with_capacity(9);
            pair.push_str(assets[0].as_static());
            pair.push_str(exch.asset_separator().as_str());
            pair.push_str(assets[1].as_static());

            pair
        },
        false => {
            let mut pair = String::with_capacity(9);
            pair.push_str(assets[1].as_static());
            pair.push_str(exch.asset_separator().as_str());
            pair.push_str(assets[0].as_static());

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
                pair.push_str(exch.asset_separator().as_str());
                pair.push_str(asset_pair[1].as_static());

                pair
            },
            false => {
                let mut pair = String::with_capacity(9);
                pair.push_str(asset_pair[1].as_static());
                pair.push_str(exch.asset_separator().as_str());
                pair.push_str(asset_pair[0].as_static());

                pair
            }
        }
    }).collect::<Vec<_>>()
}