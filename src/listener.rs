use redis;
use serde_json;

use exchange;
use orderbook;
use orderbook::tectonic;

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
pub fn listen_and_insert(r: &redis::Client, r_password: Option<String>, t: &mut tectonic::TectonicConnection) {
    let mut redis_conn = self::redis_init(r, r_password.as_ref());
    let mut subscription = redis_conn.as_pubsub();
    let mut insertions = 0;

    for exch in exchange::get_supported_exchanges() {
        subscription.subscribe(exch).expect("Failed to subscribe to channel");
    }

    loop {
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

            insertions += 1;
        }

        // Flush to disk every 10,000 ticks
        if insertions % 10_000 == 0 {
            print!("Flushing TectonicDB data to disk... ");
            let _ = t.flush_all().unwrap();
            println!("Success");
        }
    }
}

