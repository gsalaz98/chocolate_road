use std::collections::HashMap;
use std::thread;
use std::ops::Deref;
use std::sync::{Arc, atomic, Mutex, RwLock};

use chrono::prelude::*;
use redis::{self, Commands};
use reqwest;
use serde_json;
use ws;
use ws::{Error, Handler, Handshake, Message, Sender};

use exchange::{self, Asset, AssetExchange, Exchange};
use orderbook;

/// Exchange related metadata. The fields are used to establish
/// a successful connection with the exchange via websockets.
#[derive(Clone)]
pub struct WSExchange {
    /// Host - Can be domain name or IP address
    pub host: String,
    /// Port - Optional value. If no value is provided, the final URL won't have a port specified
    pub port: Option<u16>,
    /// Custom path for connection. Is appended at the end of a URL if present. Do not add trailing forward-slash.
    pub conn_path: Option<String>,

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
        let mut settings = Self {
            host: "wss://ws-feed.pro.coinbase.com".into(),
            port: None,
            conn_path: None,

            snapshot_received: false,

            metadata: MetaData {
                exchange: Arc::new("gdax".into()),
                asset_pair: Some(vec![
                    [Asset::BTC, Asset::USD],]),
                start_date: None,
                end_date: None,
            },

            single_channels: vec!["level2".into()],

            tectonic: orderbook::tectonic::TectonicConnection::new(None, None).expect("Unable to connect to TectonicDB"),
            r: redis::Client::open("redis://localhost").unwrap(),
            r_password: None,
        };

        Ok(Box::new(settings))
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
        let mut connect_url = String::new();
        // Try to use the settings the user passes before resorting to default settings.
        let mut settings = settings.cloned().unwrap_or(*WSExchange::default_settings().unwrap());

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
            
            tectonic: settings.tectonic.clone(),
            r: Arc::new(Mutex::new(settings.init_redis().expect("Failed to connect to Redis server."))),

            out,
        }).expect("Failed to establish websocket connection");
    }
}

#[derive(Serialize, Deserialize)]
struct SubscribeMessage {
    #[serde(rename = "type")]
    type_: String,

    product_ids: Vec<String>,
    channels: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct L2Message {
    #[serde(rename = "type")]
    type_: String,
    product_id: String,
    time: String,
    changes: Vec<(String, String, String)>,
}

impl Handler for WSExchangeSender {
    fn on_open(&mut self, shake: Handshake) -> Result<(), Error> {
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
            match serde_json::from_slice::<L2Message>(&msg.into_data()) {
                Ok(message) => {
                    let mut deltas = Vec::with_capacity(message.changes.len());
                    // Begin sequence counting at 1 in order to reconstruct a proper sequence count 
                    let mut seq = 1;

                    for update in message.changes {
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
                },
                Err(e) => println!("Error: {}", e),
            };
        });

        Ok(())
    }
}