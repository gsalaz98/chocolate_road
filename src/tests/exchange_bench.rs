#[test]
fn bitmex_bench() {
    use std::env;
    use std::thread;

    use redis;

    use exchange::{Asset, AssetExchange};
    use exchange::bitmex;

    // Redis client is setup here so that we can provide it a host, password, and database
    let r = redis::Client::open("redis://127.0.0.1:6379/0").unwrap();
    let r_password = match env::var_os("REDIS_AUTH") {
        Some(password) => Some(password.into_string().unwrap()),
        None => None   
    };

    let mut bitmex_settings = *bitmex::WSExchange::default_settings().unwrap();
    bitmex_settings.metadata.asset_pair = Some(vec![
        [Asset::BTC, Asset::USD],]);

    bitmex_settings.r = r.clone();
    bitmex_settings.r_password = r_password.as_ref().cloned();

    let exchange = thread::spawn(move || bitmex::WSExchange::run(Some(&bitmex_settings)));
    let _ = exchange.join();
}

#[test]
fn gdax_bench() {
    use std::env;
    use std::thread;

    use redis;

    use exchange::{Asset, AssetExchange};
    use exchange::gdax_l2;

    // Redis client is setup here so that we can provide it a host, password, and database
    let r = redis::Client::open("redis://127.0.0.1:6379/0").unwrap();
    let r_password = match env::var_os("REDIS_AUTH") {
        Some(password) => Some(password.into_string().unwrap()),
        None => None   
    };

    let mut gdax_settings = *gdax_l2::WSExchange::default_settings().unwrap();
    gdax_settings.metadata.asset_pair = Some(vec![
        [Asset::BTC, Asset::USD],
        [Asset::ETH, Asset::USD],
        [Asset::LTC, Asset::USD],
        [Asset::BTC, Asset::USDC],
    ]);
    gdax_settings.r = r.clone();
    gdax_settings.r_password = r_password.as_ref().cloned();

    let exchange = thread::spawn(move || gdax_l2::WSExchange::run(Some(&gdax_settings)));
    let _ = exchange.join();
}