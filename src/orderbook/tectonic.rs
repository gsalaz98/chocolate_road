use std::net::TcpStream;
use std::io::{Error, Read, Write};
use std::time::Duration;
use std::str;

use orderbook::{self, Delta};

/// Contains all fields necessary for a successful connection to TectonicDB.
pub struct TectonicConnection {
    /// TectonicDB host
    host: String,
    /// Port
    port: u16,
    
    /// TCP client connection for internal use
    pub connection: TcpStream,

    /// Currently selected database
    pub db: Option<String>,
}

impl TectonicConnection {
    /// Clones the structure
    pub fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            port: self.port.clone(),

            connection: self.connection.try_clone().expect("Failed to clone tectonic TcpStream"),

            db: Some(self.db.as_ref().unwrap_or(&String::from("")).clone())
        }
    }
    /// Creates a new TectonicDB connection. If no host or port are provided, the connection defaults to `localhost:9001`
    pub fn new(host: Option<String>, port: Option<u16>) -> Result<TectonicConnection, Error>{
        let host = host.unwrap_or("127.0.0.1".into());
        let port = port.unwrap_or(9001);

        let connect_address = format!("{}:{}", host, port);

        // Set socket timeout to 1s
        let connection = TcpStream::connect_timeout(&connect_address.parse().unwrap(), Duration::new(1,0))?;
        
        return Ok(TectonicConnection {
            host,
            port,

            connection,

            db: None,
        })
    }

    /// Sends a message to the TectonicDB server
    pub fn cmd(&mut self, message: String) -> Result<String, Error> { 
        // Create buffer to store our message in. Use vector to store variable length message
        let mut buf: Vec<u8> = Vec::new();

        // Convert the message into bytes using the `.as_bytes()` method
        let _ = self.connection.write((message + "\n".into()).as_bytes())?;
        let _ = self.connection.read(&mut buf)?;

        Ok(str::from_utf8(&buf).unwrap().to_string())
    }
    /// Return help dialog
    pub fn help(&mut self) -> Result<String, Error> {
        self.cmd("HELP".into())
    }
    /// Ping the server
    pub fn ping(&mut self) -> Result<String, Error> {
        self.cmd("PING".into())
    }
    /// Get server metrics and information
    pub fn info(&mut self) -> Result<String, Error> {
        self.cmd("INFO".into())
    }
    /// Get server performance metrics
    pub fn perf(&mut self) -> Result<String, Error> {
        self.cmd("PERF".into())
    }
    /// Write data in database to disk
    pub fn flush(&mut self) -> Result<String, Error> {
        self.cmd("FLUSH".into())
    }
    /// Write all data in every database to disk
    pub fn flush_all(&mut self) -> Result<String, Error> {
        self.cmd("FLUSH ALL".into())
    }
    /// Clear the current database of all entries
    pub fn clear(&mut self) -> Result<String, Error> {
        self.cmd("CLEAR".into())
    }
    /// Clear every database of all entries
    pub fn clear_all(&mut self) -> Result<String, Error> {
        self.cmd("CLEAR ALL".into())
    }
    /// Count entries in current database TODO: make it return an int value
    pub fn count(&mut self) -> Result<String, Error> {
        self.cmd("COUNT".into())
    }
    /// Count entries in all databases
    pub fn count_all(&mut self) -> Result<String, Error> {
        self.cmd("COUNT ALL".into())
    }
    /// Checks if `db_name` exists
    pub fn exists(&mut self, db_name: String) -> Result<bool, Error> {
        let result = self.cmd(format!("EXISTS {}", db_name))?;

        Ok(result.chars().next().unwrap_or('0') == '1')
    }
    /// Bulk-add deltas to the tectonic server
    pub fn bulk_add(&mut self, deltas: &Vec<Delta>) -> Result<String, Error> {
        let _ = self.cmd("BULKADD".into());

        for event in deltas {
            let is_trade: String = if event.event & orderbook::TRADE == orderbook::TRADE {"t".into()} else {"f".into()};
            let is_bid: String = if event.event & orderbook::BID == orderbook::BID {"t".into()} else {"f".into()};

            let _ = self.cmd(format!("{:.3}, {}, {}, {}, {}, {};", event.ts, event.seq, is_trade, is_bid, event.price, event.size));
        }

        self.cmd("DDAKLUB".into())
    }
    /// Bulk-add deltas into a specified database `db_name`
    pub fn bulk_add_into(&mut self, db_name: String, deltas: &Vec<Delta>) -> Result<String, Error> {
        let _ = self.cmd(format!("BULKADD INTO {}", db_name));

        for event in deltas {
            let _ = self.cmd(format!("{:.3}, {}, {}, {}, {}, {};", 
                event.ts, 
                event.seq, 
                if event.event & orderbook::TRADE == orderbook::TRADE {String::from("t")} else {String::from("f")},
                if event.event & orderbook::BID == orderbook::BID {String::from("t")} else {String::from("f")},
                event.price, 
                event.size));
        }

        self.cmd("DDAKLUB".into())
    }
    /// Create new database `db_name`
    pub fn create(&mut self, db_name: String) -> Result<String, Error> {
        self.cmd(format!("CREATE {}", db_name))
    }
    /// Insert into the currently selected database
    pub fn insert(&mut self, delta: &Delta) -> Result<String, Error> {
        self.cmd(format!("INSERT {:.3}, {}, {}, {}, {}, {};", 
            delta.ts, 
            delta.seq, 
            if delta.event & orderbook::TRADE == orderbook::TRADE {String::from("t")} else {String::from("f")},
            if delta.event & orderbook::BID == orderbook::BID {String::from("t")} else {String::from("f")}, 
            delta.price, 
            delta.size))
    }
    /// Insert into the database `db_name`
    pub fn insert_into(&mut self, db_name: String, delta: &Delta) -> Result<String, Error> {
        self.cmd(format!("INSERT {:.3}, {}, {}, {}, {}, {}; INTO {}", 
            delta.ts, 
            delta.seq, 
            if delta.event & orderbook::TRADE == orderbook::TRADE {String::from("t")} else {String::from("f")},
            if delta.event & orderbook::BID == orderbook::BID {String::from("t")} else {String::from("f")}, 
            delta.price, 
            delta.size,
            db_name))
    }
}

impl Clone for TectonicConnection {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            port: self.port.clone(), 

            connection: self.connection
                .try_clone()
                .expect("Failed to clone Tectonic TCP Connection"),

            db: self.db.clone(),
        }
    }
}