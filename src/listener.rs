use redis;
use serde_json;

use exchange;
use orderbook;
use orderbook::tectonic;

/// Listens on redis for [`Delta`] ticks and writes them to TectonicDB.
/// This function is called and ran in its own thread.
pub fn listen_and_insert(r: &redis::Client, r_password: Option<String>, t: &mut tectonic::TectonicConnection) {
    let mut redis_conn = r.get_connection().unwrap();
    match r_password {
        Some(password) => redis::cmd("AUTH").arg(password)
            .execute(&redis_conn),
        None => ()
    }

    let mut subscription = redis_conn.as_pubsub();

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
            t.insert_into(
                format!("{}_{}", message.get_channel_name(), delta.symbol), 
                delta)
            .unwrap();
        }
    }
}

