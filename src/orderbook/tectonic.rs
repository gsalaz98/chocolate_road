use std::net::TcpStream;
use std::io::{Error, Read, Write};
use std::str;

use orderbook::{self, Delta};

pub struct TectonicConnection {
    host: String,
    port: u16,
    
    pub connection: TcpStream,

    pub db: Option<String>,
    pub symbol: Option<String>,
    pub exchange: Option<String>,
}

impl TectonicConnection {
    /// Creates a new TectonicDB connection. If no host or port are provided, the connection defaults to `localhost:9001`
    pub fn new(host: Option<String>, port: Option<u16>) -> Result<TectonicConnection, Error>{
        let host = host.unwrap_or("127.0.0.1".into());
        let port = port.unwrap_or(9001);

        let connect_address = format!("{}:{}", host, port);
        let connection = TcpStream::connect(connect_address)?;
        
        return Ok(TectonicConnection {
            host,
            port,

            connection,

            db: None,
            symbol: None,
            exchange: None,
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

    pub fn help(&mut self) -> Result<String, Error> {
        self.cmd("HELP".into())
    }
    pub fn ping(&mut self) -> Result<String, Error> {
        self.cmd("PING".into())
    }
    pub fn info(&mut self) -> Result<String, Error> {
        self.cmd("INFO".into())
    }
    pub fn perf(&mut self) -> Result<String, Error> {
        self.cmd("PERF".into())
    }
    //pub fn count(&mut self) -> Result<String, Error> {}
    //pub fn count_all(&mut self) -> Result<String, Error> {}
    pub fn exists(&mut self, db_name: String) -> Result<bool, Error> {
        let result = self.cmd(format!("EXISTS {}", db_name))?;

        Ok(result.chars().next().unwrap() == '1')
    }

    pub fn bulk_add(&mut self, deltas: &Vec<Delta>) -> Result<String, Error> {
        let _ = self.cmd("BULKADD".into());

        for event in deltas {
            let is_trade: String = if event.event & orderbook::TRADE == orderbook::TRADE {"t".into()} else {"f".into()};
            let is_bid: String = if event.event & orderbook::BID == orderbook::BID {"t".into()} else {"f".into()};

            let _ = self.cmd(format!("{:.3}, {}, {}, {}, {}, {}", event.ts, event.seq, is_trade, is_bid, event.price, event.size));
        }

        self.cmd("DDAKLUB".into())
    }
    pub fn bulk_add_into(&mut self, db_name: String, deltas: &Vec<Delta>) -> Result<String, Error> {
        let _ = self.cmd(format!("BULKADD INTO {}", db_name));

        for event in deltas {
            let is_trade: String = if event.event & orderbook::TRADE == orderbook::TRADE {"t".into()} else {"f".into()};
            let is_bid: String = if event.event & orderbook::BID == orderbook::BID {"t".into()} else {"f".into()};

            let _ = self.cmd(format!("{:.3}, {}, {}, {}, {}, {}", event.ts, event.seq, is_trade, is_bid, event.price, event.size));
        }

        self.cmd("DDAKLUB".into())
    }
    pub fn create(&mut self, db_name: String) -> Result<String, Error> {
        self.cmd(format!("CREATE {}", db_name))
    }

    //pub fn get_from(&mut self) -> Result<String, Error> {}
    pub fn insert(&mut self) -> Result<String, Error> {

        Ok(("".into()))
    }

    /*
    pub fn insert_into(&mut self) -> Result<String, Error> {}

    //pub fn clear(&mut self) -> Result<String, Error> {}
    //pub fn clear_all(&mut self) -> Result<String, Error> {}
    pub fn flush(&mut self) -> Result<String, Error> {}
    pub fn flush_all(&mut self) -> Result<String, Error> {}

    */
}