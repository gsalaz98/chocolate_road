use std::env;
use std::thread;
use std::time::Duration;

use chrono::prelude::*;
use redis;
use serde_json;

use exchange;
use orderbook;
use orderbook::tectonic;
use uploader;

/// Initializes redis connection. Takes care of authentication if a password is present
pub fn redis_init(r: &redis::Client, r_password: Option<&String>) -> redis::Connection {
    let redis_conn = r.get_connection().unwrap();

    match r_password {
        Some(password) => redis::cmd("AUTH").arg(password)
            .execute(&redis_conn),
        None => ()
    };

    redis_conn
}
/// Listens on redis for [`Delta`] ticks and writes them to TectonicDB.
/// This function is called and ran in its own thread.
pub fn redis_listen_and_insert(r: &redis::Client, r_password: Option<String>,
                         t: &mut tectonic::TectonicConnection) {

    let mut redis_conn = self::redis_init(r, r_password.as_ref());
    let mut subscription = redis_conn.as_pubsub();
    let mut ticks = 0;

    for exch in exchange::get_supported_exchanges() {
        subscription.subscribe(exch).expect("Failed to subscribe to channel");
    }

    loop {
        // Sleep while ticks are accumulated. This will ensure that the database
        // can be written to every `n` periods. This parameter can be configured
        // by the environment variable `UPLOAD_PERIOD`, set in seconds.
        thread::sleep(Duration::from_secs(match env::var("UPLOAD_PERIOD") {
            Ok(var) => var.parse::<u64>().unwrap(),
            Err(_) => 86400u64,
        }));

        // Begin by reading from redis
        let message = subscription.get_message().unwrap();
        let payload: String = message.get_payload().unwrap();

        // Deserialize and load into delta struct for insertion to tectonicdb
        let deltas = serde_json::from_str::<Vec<orderbook::Delta>>(&payload);

        if deltas.is_err() {
            println!("Log Error: {}", deltas.err().unwrap());
            continue;
        }

        for delta in &deltas.unwrap() {
            let _ = t
                .insert_into(format!("{}_{}", message.get_channel_name(), delta.symbol), delta)
                .unwrap();
        }

        // TODO: Write files to AWS before flushing new files to disk
        print!("Flushing TectonicDB data to disk... ");
        let _ = t.flush_all().unwrap();
        let t = Utc::now().to_rfc3339() + ".tar.xz".into();

        uploader::compress_database_and_delete(&t, None).unwrap();
        uploader::s3_upload(&t, None, None, None).unwrap();

        println!("Success");
    }
}
