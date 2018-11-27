#[test]
fn orderbook_initialize() {
    use orderbook;

    let fake_bids = vec![
        (302.0, 50.0),
        (303.0, 100.0),
        (304.0, 11111.0),
    ];

    let fake_asks = vec![
        (305.0, 20.5),
        (306.0, 1.0),
        (307.0, 154.25),
    ];

    let fake_snapshot = orderbook::Snapshot {
        market: None,
        asset: None,

        bids: fake_bids,
        asks: fake_asks,
    };

    let mut new_ob = orderbook::Book {
        tick_size: 0.5,
        ..Default::default()
    };

    new_ob.initialize(&fake_snapshot);

    // Orderbook state tests
    assert_eq!(new_ob.state[604], Some(50.0));
    assert_eq!(new_ob.state[606], Some(100.0));
    assert_eq!(new_ob.state[608], Some(11111.0));

    assert!(new_ob.state[302].is_none());
    assert!(new_ob.state[303].is_none());
    assert!(new_ob.state[305].is_none());
    assert!(new_ob.state[605].is_none());
    assert!(new_ob.state[615].is_none());

    assert_eq!(new_ob.state[610], Some(20.5));
    assert_eq!(new_ob.state[612], Some(1.0));
    assert_eq!(new_ob.state[614], Some(154.25));

    assert_eq!(new_ob.best_bid, 608);
    assert_eq!(new_ob.best_bid_size, 11111.0);

    assert_eq!(new_ob.best_ask, 610);
    assert_eq!(new_ob.best_ask_size, 20.5);

    // Add in a new order to the mix to see if it holds up
    let orders = vec![
        ((304.5 / new_ob.tick_size) as u64, 400.523, true), // New bid order at price = 304.5
        ((305.0 / new_ob.tick_size) as u64, 0.0, false),    // Cancelation of best ask
    ];

    // mutate the orderbook with the new orders
    new_ob.new_state(&orders);

    assert_eq!(new_ob.best_bid, (304.5 / new_ob.tick_size) as u64); // new updated best bid
    assert_eq!(new_ob.best_ask, (306.0 / new_ob.tick_size) as u64); // new updated best ask

    // Use a negative to force a failed test in the case that the best bid/ask sizes didn't get updated
    assert_eq!(new_ob.best_bid_size, new_ob.state[(304.5 / new_ob.tick_size) as usize].unwrap_or(-1.0));
    assert_eq!(new_ob.best_ask_size, new_ob.state[(306.0 / new_ob.tick_size) as usize].unwrap_or(-1.0));

    let orders = vec![
        ((304.5 / new_ob.tick_size) as u64, 0.00, true),    // Void the best bid
        ((304.5 / new_ob.tick_size) as u64, 2500.0, false), // Make the previous best bid our best ask
    ];

    // mutate orderbook state
    new_ob.new_state(&orders);

    assert_eq!(new_ob.best_bid, (304.0 / new_ob.tick_size) as u64);
    assert_eq!(new_ob.best_ask, (304.5 / new_ob.tick_size) as u64);

    // Assert that the best bid is the one it previously was when we first initialized it all,
    // and also that our ask has also been updated to the previous bid
    assert_eq!(new_ob.best_bid_size, new_ob.state[(304.0 / new_ob.tick_size) as usize].unwrap_or(-1.0));
    assert_eq!(new_ob.best_ask_size, new_ob.state[(304.5 / new_ob.tick_size) as usize].unwrap_or(-1.0));

    // And finally, one last go around just to be sure I didn't cheat around the tests

    let orders = vec![
        ((304.0 / new_ob.tick_size) as u64, 0.00, true),
        ((304.0 / new_ob.tick_size) as u64, 20.5, false),
    ];

    // Final orderbook mutation
    new_ob.new_state(&orders);

    assert_eq!(new_ob.best_bid, (303.0 / new_ob.tick_size) as u64);
    assert_eq!(new_ob.best_ask, (304.0 / new_ob.tick_size) as u64);

    // Assert that the best bid is the one it previously was when we first initialized it all,
    // and also that our ask has also been updated to the previous bid
    assert_eq!(new_ob.best_bid_size, new_ob.state[(303.0 / new_ob.tick_size) as usize].unwrap_or(-1.0));
    assert_eq!(new_ob.best_ask_size, new_ob.state[(304.0 / new_ob.tick_size) as usize].unwrap_or(-1.0));
}
