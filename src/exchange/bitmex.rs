use std::collections::HashMap;

use chrono::prelude::*;
use reqwest;
use serde_json;
use url::Url;
use ws;
use ws::{Error, Handler, Handshake, Message, Sender};

use orderbook;
use super::AssetExchange;

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

    // /// Optional function that can be called as a callback per message received.
    //callback: Option<Box<Fn(&orderbook::Delta)>>,

    /// Collection metadata
    metadata: MetaData,

    /// Channel name with no argument we want to subscribe to
    single_channels: Vec<String>,
    /// Channel name as map key/value pair
    dual_channels: HashMap<String, String>,

    /// BitMEX requires asset indexes to calculate asset price
    asset_indexes: HashMap<String, u64>,
    /// Allows us to calculate the price of a given asset in combination with [`asset_indexes`]
    asset_tick_size: HashMap<String, f32>,

    /// TectonicDB connection
    tectonic: orderbook::tectonic::TectonicConnection,
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

    /// BitMEX requires asset indexes to calculate asset price
    asset_indexes: HashMap<String, u64>,
    /// Allows us to calculate the price of a given asset in combination with [`asset_indexes`]
    asset_tick_size: HashMap<String, f32>,

    /// TectonicDB connection
    tectonic: orderbook::tectonic::TectonicConnection,

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
    id: Option<u64>,
    /// Order size. If not present, then it is a level removal
    size: Option<f32>,
    /// Only present on insert and snapshot events
    price: Option<f32>
}

#[derive(Serialize, Deserialize)]
struct AssetInformation {
    symbol: String,
    timestamp: String,
    tickSize: f32,
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

            single_channels: vec![],
            dual_channels: HashMap::new(),

            asset_indexes: HashMap::new(),
            asset_tick_size: HashMap::new(),

            tectonic: orderbook::tectonic::TectonicConnection::new(None, None).expect("Unable to connect to TectonicDB"),
        };
        settings.dual_channels.insert("trade".into(), "XBTUSD".into());
        settings.dual_channels.insert("orderBookL2".into(), "XBTUSD".into());
        settings
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
            
            asset_indexes: settings.asset_indexes.clone(),
            asset_tick_size: settings.asset_tick_size.clone(),

            tectonic: settings.tectonic.clone(),

            out

        }).expect("Failed to establish websocket connection");
    }
}

impl Handler for WSExchangeSender {
    fn on_open(&mut self, _: Handshake) -> Result<(), Error> {
        let mut msg = String::new();

        msg.push_str(r#"{"op": "subscribe", "args": ["#);

        for (idx, channel) in self.single_channels.iter().enumerate() {
            msg.push('"');
            msg.push_str(channel.as_str());
            msg.push('"');

            // Adds comma if we have another value in this iterator or in `dual_channels`
            if idx != channel.len() - 1 || self.dual_channels.len() > 0 {
                msg.push_str(", ");
            }
        }

        for (idx, (key, value)) in self.dual_channels.iter().enumerate() {
            msg.push('"');
            msg.push_str(key.as_str());
            msg.push(':');
            msg.push_str(value.as_str());
            msg.push('"');

            // Add comma if we haven't hit our final pair
            if idx != self.dual_channels.len() - 1 {
                msg.push_str(", ");
            }
        }
        msg.push_str("]}");

        // Now that we've built our message, let's get the indicies of the assets we can trade
        let response: Vec<AssetInformation> = reqwest::get("https://www.bitmex.com/api/v1/instrument?columns=symbol,tickSize&start=0&count=500")
            .expect("Failed to send request")
            .json()
            .expect("Failed to serialize response to JSON");

        for (index, asset) in response.iter().enumerate() {
            self.asset_indexes.insert(asset.symbol.clone(), index as u64);
            self.asset_tick_size.insert(asset.symbol.clone(), asset.tickSize);

            if !self.tectonic.exists(format!("bitmex_{}", asset.symbol.clone()))? {
                // Create tectonic database if it doesn't exist yet. This avoids many issues
                // relating to inserting to a non-existant database.
                let _ = self.tectonic.create(format!("bitmex_{}", asset.symbol.clone()));
            }
        }

        // Send our constructed message to the server
        self.out.send(msg)
    }

    fn on_message(&mut self, msg: Message) -> Result<(), Error> {
        match serde_json::from_str::<BitMEXMessage>(&msg.into_text().expect("Failed to convert message to text")) {
            Ok(message) => {
                if message.table == "" || message.table == "partial" {
                    return Ok(())
                }
                // Define a timestamp for the messages received
                let ts = Utc::now().timestamp_millis() as f64 * 0.001f64;
                let msg_length = message.data.len();
                let mut deltas: Vec<orderbook::Delta> = Vec::with_capacity(msg_length);
                let mut symbol: String = String::new();

                println!("{:?}", message.data);

                for update in message.data {
                    // Let's make sure we don't parse any values with no ID
                    if update.id.is_none() {
                        continue;
                    }

                    let is_bid = match update.side == "Buy" {
                        true => orderbook::BID,
                        false => orderbook::ASK,
                    };
                    let is_trade = match message.action == "Trade" {
                        true => orderbook::TRADE,
                        false => orderbook::UPDATE,
                    };

                    let delta = if update.symbol == "XBTUSD" {
                        orderbook::Delta {
                            price: (8800000000 - update.id.unwrap()) as f32 * 0.01,
                            size: update.size.unwrap_or(0.0),
                            seq: 0,
                            event: is_bid ^ is_trade,
                            ts,
                        }
                    } else {
                        orderbook::Delta {
                            price: ((100000000 * self.asset_indexes[&update.symbol]) - update.id.unwrap()) as f32 * self.asset_tick_size[&update.symbol],
                            size: update.size.unwrap_or(0.0),
                            seq: 0,
                            event: is_bid ^ is_trade,
                            ts,
                        }
                    };
                    
                    deltas.push(delta);
                    symbol = update.symbol;
                }

                // TODO: consider using a ZeroMQ server here to offload the work to another service
                let _ = self.tectonic.bulk_add_into(format!("bitmex_{}", symbol), &deltas)
                    .expect(format!("Failed to write to db. Symbol = bitmex_{}", symbol).as_str());
                
                return Ok(())
            },

            Err(e) => {
                println!("Error encountered: {}", e);
                return Ok(())
            },
        }
    }
}