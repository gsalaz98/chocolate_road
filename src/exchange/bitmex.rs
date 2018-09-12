use chrono::prelude::*;
use ws;
use ws::{Error, Handler, Handshake, Message, Sender};
use orderbook;
use serde_json;
use super::AssetExchange;

/// Exchange related metadata. The fields are used to establish
/// a successful connection with the exchange via websockets.
pub struct WSExchangeInfo<'a, 'b, T: 'a> {
    /// Host - Can be domain name or IP address
    host: String,
    /// Port - Optional value. If no value is provided, the final URL won't have a port specified
    port: Option<u16>,
    /// Custom path for connection. Is appended at the end of a URL if present. Do not add trailing forward-slash.
    conn_path: Option<String>,

    /// Message that gets sent on connection
    connect_message: Option<&'a String>,

    /// Indicate whether or not we've received the snapshot message yet
    snapshot_received: bool,

    /// Optional function that can be called as a callback per message received.
    /// Usually, this will send a delta, but we will make it generic to allow for flexability
    callback: Option<&'a Fn(T)>,

    /// Message sender and receiver for websockets
    out: Option<&'a Sender>,

    /// Collection metadata
    metadata: MetaData<'b>,
}

/// Meta data for our data source. This is useful for data warehousing and accessing the data.
/// All types contained within are considered optional. This may be expanded in the future.
pub struct MetaData<'a> {
    /// Vector of asset pairs we're going to warehouse
    asset_pair: Option<&'a Vec<[super::Asset; 2]>>,

    /// Starting datetime of our data collection
    start_date: Option<DateTime<Utc>>,

    /// Ending datetime of our data collection
    end_date: Option<DateTime<Utc>>,
}

/// Master bitmex message. This may contain a delta or a snapshot
#[derive(Serialize, Deserialize, Debug)]
struct BitMEXMessage {
    table: String,
    action: String,
    data: Vec<BitMEXData>,
}

/// BitMEX websocket data. All deltas and snapshot updates are sent as such
#[derive(Serialize, Deserialize, Debug)]
struct BitMEXData {
    symbol: String,
    side: String,
    id: u64,
    size: Option<String>,
    price: Option<f32>,
}

impl<'a, 'b, T> AssetExchange for WSExchangeInfo<'a, 'b, T> {
    fn default_settings(&self) -> Self {
        Self {
            host: "wss://bitmex.com".into(),
            port: None,
            conn_path: Some("realtime".into()),

            connect_message: None,
            snapshot_received: false,

            callback: None,

            out: None,

            metadata: MetaData {
                asset_pair: None,
                start_date: None,
                end_date: None,
            }
        }
    }

    fn error_code(&self, code: u16) -> Result<(), String> {
        match code {
            101 => Ok(()),
            200 => Ok(()),
            404 => Err("Error 404: Resource not found.".into()),
            500 => Err("Error 500: Service Unavailable".into()),
            _ => Err(format!("Error {}: Unknown error encountered", code)),
        }
    }

    fn snapshot<S>(&self, snap: S) {
    }
}

impl<'a, 'b, T> Handler for WSExchangeInfo<'a, 'b, T> {
    fn on_open(&mut self, _: Handshake) -> Result<(), Error> {
        match self.connect_message {
            Some(message) => self.out.unwrap().send(message.as_str()),
            None => Ok(())
        }
    }

    fn on_message(&mut self, msg: Message) -> Result<(), Error> {
        match serde_json::from_str::<BitMEXMessage>(&msg.into_text().unwrap()) {
            Ok(p_msg) => println!("{:?}", p_msg),
            Err(e) => println!("{}", e),
        }
        Ok(())
    }
}