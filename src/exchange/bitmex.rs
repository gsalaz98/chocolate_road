use std::collections::HashMap;

use chrono::prelude::*;
use serde_json;
use url::Url;
use ws;
use ws::{Error, Handler, Handshake, Message, Sender};

use orderbook;
use super::AssetExchange;

const UPDATE_STR: &str = "update";
const PARTIAL_STR: &str = "partial";
const XBTUSD: &str = "XBTUSD";

/// Exchange related metadata. The fields are used to establish
/// a successful connection with the exchange via websockets.
pub struct WSExchange {
    /// Host - Can be domain name or IP address
    host: String,
    /// Port - Optional value. If no value is provided, the final URL won't have a port specified
    port: Option<u16>,
    /// Custom path for connection. Is appended at the end of a URL if present. Do not add trailing forward-slash.
    conn_path: Option<String>,

    /// Indicate whether or not we've received the snapshot message yet
    snapshot_received: bool,

    /// Optional function that can be called as a callback per message received.
    //callback: Option<Box<Fn(&orderbook::Delta)>>,

    /// Collection metadata
    metadata: MetaData,

    /// Channel name with no argument we want to subscribe to
    single_channels: Vec<String>,
    /// Channel name as map key/value pair
    dual_channels: HashMap<String, String>,
}

/// Create two identical structs and transfer the data over when we start the websocket.
pub struct WSExchangeSender {
    /// Host - Can be domain name or IP address
    host: String,
    /// Port - Optional value. If no value is provided, the final URL won't have a port specified
    port: Option<u16>,
    /// Custom path for connection. Is appended at the end of a URL if present. Do not add trailing forward-slash.
    conn_path: Option<String>,

    /// Indicate whether or not we've received the snapshot message yet
    snapshot_received: bool,

    /// Optional function that can be called as a callback per message received.
    /// Usually, this will send a delta, but we will make it generic to allow for flexability
    //callback: Option<Box<Fn(&orderbook::Delta)>>,

    /// Collection metadata
    metadata: MetaData,

    /// Channel name with no argument we want to subscribe to
    single_channels: Vec<String>,
    /// Channel name as map key/value pair
    dual_channels: HashMap<String, String>,

    /// Websocket sender
    out: Sender,
}

/// Meta data for our data source. This is useful for data warehousing and accessing the data.
/// All types contained within are considered optional. This may be expanded in the future.
#[derive(Clone)]
pub struct MetaData {
    /// Vector of asset pairs we're going to warehouse
    asset_pair: Option<Vec<[super::Asset; 2]>>,

    /// Starting datetime of our data collection
    start_date: Option<DateTime<Utc>>,

    /// Ending datetime of our data collection
    end_date: Option<DateTime<Utc>>,
}

/// Master bitmex message. This may contain a delta or a snapshot
#[derive(Serialize, Deserialize, Debug)]
struct BitMEXMessage {
    /// Specifies where update originates from (i.e. channel)
    table: String,
    /// Tells if action is a snapshot or delta
    action: String,
    /// Snapshot or delta data
    data: Vec<BitMEXData>,
}

/// BitMEX websocket data. All deltas and snapshot updates are sent as such
#[derive(Serialize, Deserialize, Debug)]
struct BitMEXData {
    /// Asset-pair name
    symbol: String,
    /// Orderbook side (bid/ask)
    side: String,
    /// Price comes encoded in this value.
    id: u64,
    /// Order size. If not present, then it is a level removal
    size: Option<u64>,
    /// Only present on insert and snapshot events
    price: Option<f32>
}

impl AssetExchange for WSExchange {
    fn default_settings() -> Self {
        let mut settings = Self {
            host: "wss://www.bitmex.com".into(),
            port: None,
            conn_path: Some("realtime".into()),

            snapshot_received: false,

            //callback: None,

            metadata: MetaData {
                asset_pair: None,
                start_date: None,
                end_date: None,
            },

            single_channels: vec!["trade".into()],
            dual_channels: HashMap::new(),
        };
        settings.dual_channels.insert("orderBookL2".into(), "XBTUSD".into());

        settings
    }

    fn snapshot<S>(&self, snap: S) {
    }

    fn run(settings: Option<&Self>) {
        let default_ws = WSExchange::default_settings();
        let mut connect_url = String::new();
        let settings = settings.unwrap_or(&default_ws);

        connect_url.push_str(settings.host.as_str());
        
        if !settings.port.is_none() {
            connect_url.push(':');
            connect_url.push_str(settings.port.unwrap().to_string().as_str());
        }
        if !settings.conn_path.is_none() {
            connect_url.push('/');
            connect_url.push_str(settings.conn_path.as_ref().unwrap().as_str());
        }

        ws::connect(connect_url, |out| WSExchangeSender {
            host: settings.host.clone(),
            port: settings.port.clone(),
            conn_path: settings.conn_path.clone(),

            snapshot_received: settings.snapshot_received.clone(),
            metadata: settings.metadata.clone(),

            single_channels: settings.single_channels.clone(),
            dual_channels: settings.dual_channels.clone(),
            
            out

        }).unwrap();
    }
}

impl Handler for WSExchangeSender {
    fn on_open(&mut self, _: Handshake) -> Result<(), Error> {
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<(), Error> {
        match serde_json::from_str::<BitMEXMessage>(&msg.into_text().unwrap()) {
            Ok(message) => {
                if message.action == "partial" {
                    return Ok(())
                }
                for update in message.data {
                    match update.symbol.as_str() {
                        XBTUSD => {
                            let is_bid = update.side == "Buy";
                            let is_trade = 
                            orderbook::Delta {
                                price: ((100000000 * 88) - update.id) as f32 * 0.01,
                                size: update.size.unwrap_or(0) as f32,
                                seq: 0,
                                ts: 0,
                                event: orderbook::TRADE,
                            };
                        },
                        _ => {}
                    }
                }
                return Ok(())
            },

            Err(e) => {
                println!("{}", e);
                return Ok(())
            },
        }
    }
}