use std::thread;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;
use redis::{self, Commands};
use serde_json;
use ws;
use ws::util::Token;
use ws::{Error, Handler, Handshake, Message, Sender};

use exchange::{self, Asset, AssetExchange, Exchange};
use orderbook;

const EXPIRE: Token = Token(1);

/// Exchange related metadata. The fields are used to establish
/// a successful connection with the exchange via websockets.
#[derive(Clone)]
pub struct WSExchange {
    /// Full URL to connect to. Example: `wss://www.bitmex.com/realtime
    pub host: String,

    /// Indicate whether or not we've received the snapshot message yet
    pub snapshot_received: bool,

    /// Collection metadata
    pub metadata: MetaData,

    /// Channel name with no argument we want to subscribe to
    pub single_channels: Vec<String>,

    /// TectonicDB connection
    pub tectonic: orderbook::tectonic::TectonicConnection,

    /// Redis client (before connection)
    pub r: redis::Client,
    /// Redis password: If this is present, we will send an AUTH message to the server on connect
    pub r_password: Option<String>,
}

/// Create two identical structs and transfer the data over when we start the websocket.
pub struct WSExchangeSender {
    /// Full URL to connect to. Example: `wss://www.bitmex.com/realtime`
    host: String,

    /// Indicate whether or not we've received the snapshot message yet
    snapshot_received: bool,

    /// Collection metadata
    metadata: MetaData,

    /// Channel name with no argument we want to subscribe to
    single_channels: Vec<String>,

    /// TectonicDB connection
    tectonic: orderbook::tectonic::TectonicConnection,
    /// Redis client (used to send deltas as PUBSUB)
    r: Arc<Mutex<redis::Connection>>,

    /// Websocket sender
    out: Sender,
}

/// Meta data for our data source. This is useful for data warehousing and accessing the data.
/// All types contained within are considered optional. This may be expanded in the future.
#[derive(Clone)]
pub struct MetaData {
    /// Exchange name. We will warehouse the data under this name
    pub exchange: Arc<String>,

    /// Vector of asset pairs we're going to warehouse
    pub asset_pair: Option<Vec<[exchange::Asset; 2]>>,

    /// Starting datetime of our data collection
    start_date: Option<DateTime<Utc>>,

    /// Ending datetime of our data collection
    end_date: Option<DateTime<Utc>>,
}

impl AssetExchange for WSExchange {
    fn default_settings() -> Result<Box<Self>, String> {
        Ok(Box::new(Self {
            host: "wss://ws-feed.pro.coinbase.com".into(),

            snapshot_received: false,

            metadata: MetaData {
                exchange: Arc::new("gdax".into()),
                asset_pair: Some(vec![
                    [Asset::BTC, Asset::USD],]),
                start_date: None,
                end_date: None,
            },

            single_channels: vec![
                "level2".into(), 
                "matches".into()],

            tectonic: orderbook::tectonic::TectonicConnection::new(None, None).expect("Unable to connect to TectonicDB"),
            r: redis::Client::open("redis://localhost").unwrap(),
            r_password: None,
        }))
    }

    fn init_redis(&mut self) -> Result<redis::Connection, redis::RedisError> {
        let redis_connection = self.r.clone()
            .get_connection()
            .unwrap();

        // Send an auth message if we have a password
        match &self.r_password {
            Some(password) => {
                redis::cmd("AUTH").arg(password)
                    .execute(&redis_connection);
            },
            None => (),
        };

        Ok(redis_connection)
    }

    fn run(settings: Option<&Self>) {
        // Try to use the settings the user passes before resorting to default settings.
        let mut settings = settings.cloned().unwrap_or(*WSExchange::default_settings().unwrap());

        ws::connect(settings.host.clone(), |out| WSExchangeSender {
            host: settings.host.clone(),

            snapshot_received: settings.snapshot_received.clone(),
            metadata: settings.metadata.clone(),

            single_channels: settings.single_channels.clone(),
            
            tectonic: settings.tectonic.clone(),
            r: Arc::new(Mutex::new(settings.init_redis().expect("Failed to connect to Redis server."))),

            out,
        }).unwrap();
    }
}

#[derive(Serialize, Deserialize)]
struct SubscribeMessage {
    #[serde(rename = "type")]
    type_: String,

    product_ids: Vec<String>,
    channels: Vec<String>,
}

/// Every event that gets transmitted on websockets can have this form. We have
/// generalized the struct to accomodate snapshots, matches, and level 2 orderbook updates.
#[derive(Serialize, Deserialize)]
struct EventMessage {
    #[serde(rename = "type")]
    /// Message type
    type_: String,
    /// Asset symbol message applies to
    product_id: String,
    /// Message timestamp (from GDAX)
    time: String,

    // level2 channel fields
    // Fields are optional because we may end up processing
    // a trade/tick event.

    /// Snapshot bids
    bids: Option<Vec<(String, String)>>,
    /// Snapshot asks
    asks: Option<Vec<(String, String)>>,

    /// Orderbook deltas
    changes: Option<Vec<(String, String, String)>>,

    // Match channel fields
    /// Trade ID. Unsure what this field does...
    trade_id: Option<u64>,
    /// Sequence count. Useful for determining order of events
    sequence: Option<u128>,
    /// Maker Order ID (not user profile ID)
    maker_order_id: Option<String>,
    /// Taker ID (not user profile ID)
    taker_order_id: Option<String>,

    /// Order size (in quantity of asset)
    size: Option<String>,
    /// Order price (in quantity of market)
    price: Option<String>,
    /// Order side (bid/ask)
    side: Option<String>,
}

impl Handler for WSExchangeSender {
    fn on_open(&mut self, _: Handshake) -> Result<(), Error> {
        // Set a timeout for 5 seconds of inactivity
        self.out.timeout(5_000, EXPIRE).unwrap();

        for pair in self.metadata.asset_pair.as_ref().expect("No asset pairs passed to GDAX structure") {
            let db_name = format!("{}_{}", self.metadata.exchange.deref(), exchange::get_asset_pair(pair, Exchange::GDAX));

            if !self.tectonic.exists(db_name.clone())? {
                let _ = self.tectonic.create(db_name);
            }
        }

        let mut msg = SubscribeMessage {
            type_: "subscribe".into(),
            product_ids: vec![],
            channels: vec![],
        };

        for pair in self.metadata.asset_pair.as_ref().expect("No asset pair provided to GDAX struct") {
            // Formats the asset pairs into the exchange's asset pair notation
            let normalized_pair = exchange::get_asset_pair(pair, Exchange::GDAX);
            msg.product_ids.push(normalized_pair);
        }

        for channel in &self.single_channels {
            msg.channels.push(channel.to_string());
        }

        println!("Sending message {}", serde_json::to_string(&msg).unwrap());
        self.out.send(serde_json::to_string(&msg).unwrap())
        /*if !self.tectonic.exists(format!("bitmex_{}", asset.symbol.clone()))? {
         *        // Create tectonic database if it doesn't exist yet. This avoids many issues
         *        // relating to inserting to a non-existant database.
         *        let _ = self.tectonic.create(format!("bitmex_{}", asset.symbol.clone()));
         *}
         */
    }

    fn on_message(&mut self, msg: Message) -> Result<(), Error> {
        let redis_ref = self.r.clone();
        let exchange = self.metadata.exchange.clone();

        thread::spawn(move || {
            match serde_json::from_slice::<EventMessage>(&msg.into_data()) {
                Ok(message) => {
                    // Begin sequence counting at 1 in order to reconstruct a proper sequence count 
                    if message.changes.is_some() {
                        let mut seq = 1;
                        let mut deltas: Vec<orderbook::Delta> = Vec::with_capacity(32);

                        for update in message.changes.unwrap() {
                            deltas.push(orderbook::Delta {
                                // TODO: See if there's a way to avoid using clone
                                symbol: message.product_id.clone(),
                                price: update.1.parse::<f32>().unwrap(),
                                size: update.2.parse::<f32>().unwrap(),
                                seq: seq,
                                event: if update.0 == "buy" {
                                        orderbook::BID
                                    } else { 
                                        orderbook::ASK 
                                    } ^ if update.2.parse::<f32>().unwrap() == 0.0 {
                                        orderbook::REMOVE
                                    } else {
                                        orderbook::UPDATE
                                    },
                                ts: Utc.datetime_from_str(&message.time, "%Y-%m-%dT%H:%M:%S.%3fZ")
                                    .unwrap()
                                    .timestamp_millis() as f64 * 0.001f64
                            });

                            seq += 1;
                        }

                        // Lock the connection until we are able to aquire it
                        let _ = redis_ref.as_ref()
                            .lock()
                            .unwrap()
                            .publish::<&str, &str, u8>(exchange.deref(), &serde_json::to_string(&deltas).unwrap())
                            .expect("Failed to publish message to redis PUBSUB");

                    } else if message.type_ == "match" || message.type_ == "last_match" {
                        let _ = redis_ref.as_ref()
                            .lock()
                            .unwrap()
                            .publish::<&str, &str, u8>(
                                exchange.deref(), 
                                &serde_json::to_string(&[orderbook::Delta{
                                    symbol: message.product_id,
                                    price: message.price.unwrap().parse::<f32>().unwrap(),
                                    size: message.size.unwrap().parse::<f32>().unwrap(),
                                    seq: message.sequence.unwrap() as u32,
                                    event: if message.side.unwrap() == "buy" {
                                        orderbook::BID
                                    } else { 
                                        orderbook::ASK 
                                    } ^ orderbook::TRADE,

                                    ts: Utc.datetime_from_str(&message.time, "%Y-%m-%dT%H:%M:%S.%6fZ")
                                        .expect("Failed to parse DateTime from string")
                                        .timestamp_millis() as f64 * 0.001f64 
                                }])
                                .unwrap())
                            .expect("Failed to publish GDAX 'match' to Redis");
                    } else {
                        // Message is snapshot. Save to disk and upload to s3 or google cloud 

                    }
                },
                Err(e) => println!("Error: {}", e),
            };
        });

        Ok(())
    }

    fn on_close(&mut self, _: ws::CloseCode, _: &str) {
        // TODO: Have proper handling of disconnect events. We should be handling disconnects more gracefully
        // instead of just reconnecting. We need to be prepared for them and handle data accordingly.
        println!("GDAX Socket is closing. Opening a new connection...");

        ws::connect(self.host.clone(), |out| WSExchangeSender{
            host: self.host.clone(),
            snapshot_received: false,
            metadata: self.metadata.clone(),

            single_channels: self.single_channels.clone(),

            tectonic: self.tectonic.clone(),
            r: self.r.clone(),

            out,
        }).unwrap();
    }

    fn on_timeout(&mut self, event: Token) -> Result<(), ws::Error> {
        // TODO: Have proper handling of disconnect events. We should be handling disconnects more gracefully
        // instead of just reconnecting. We need to be prepared for them and handle data accordingly.
        println!("GDAX Socket timed out (5s of inactivity). Opening a new connection...");

        ws::connect(self.host.clone(), |out| WSExchangeSender{
            host: self.host.clone(),
            snapshot_received: false,
            metadata: self.metadata.clone(),

            single_channels: self.single_channels.clone(),

            tectonic: self.tectonic.clone(),
            r: self.r.clone(),

            out,
        }).unwrap();

        Ok(())
    }
}