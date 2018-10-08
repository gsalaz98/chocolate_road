use chrono::prelude::*;
//use ndarray;
use rayon::prelude::*;
use exchange::Asset;

/// TectonicDB client bindings
pub mod tectonic;

/// Insertion event (i.e. new order)
pub const INSERT: u8 = 1;
/// Order cancelation
pub const REMOVE: u8 = 1 << 1;
/// Order update (i.e. updates size)
pub const UPDATE: u8 = 1 << 2; 
/// Trade event
pub const TRADE: u8 = 1 << 3;
/// Ask side order
pub const ASK: u8 = 1 << 4;
/// Bid side order
pub const BID: u8 = 1 << 5;


/// Contains all the necessary parts to reconstruct an orderbook. Deltas are the incremental changes
/// that happen to the orderbook over time. Deltas are the primary way that orderbooks are updated.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Delta {
    /// Level price
    pub price: f32,
    /// Level size
    pub size: f32,
    /// Sequence count
    pub seq: u32,
    /// Encodes two pieces of information using bitwise flags -- The order side (bid/ask), and the event that occured.
    pub event: u8,
    /// Timestamp -- This is `u32` because `tectonicdb` expects `u32` for timestamp as UNIX epoch time
    pub ts: f64,

}

/// Before we can start applying deltas, we must have a snapshot to build off of. This is the initial state of the
/// orderbook that we build off of, and will use to analyze the orderbook.
#[derive(Clone)]
pub struct Snapshot {
    /// Market asset
    pub market: Option<Asset>,
    /// Secondary asset
    pub asset: Option<Asset>,

    /// Bid side orders
    pub bids: Vec<(f32, f32)>,
    /// Ask side orders
    pub asks: Vec<(f32, f32)>,
}

/// Orderbook state and related fields. This struct encodes all information related to the orderbook 
/// that we maintain. A few fields have been added for performance reasons and convienience, such as `best_bid`,
/// `best_bid_size`, `best_ask`, `best_ask_size`. 
#[derive(Clone)]
pub struct Book {
    /// Market asset
    pub market: Option<Asset>,
    /// Secondary asset
    pub asset: Option<Asset>,
    /// Starting timestamp
    pub start_ts: DateTime<Utc>,

    /// Minimum increment in price that we allow
    pub tick_size: f32,
    /// Minimum increment in size that we allow
    pub lot_size: f32,

    /// Start sequence count
    pub start_seq: u64,

    /// Best bid (as array index/non-normalized)
    pub best_bid: u64,
    /// Best ask (as array index)
    pub best_ask: u64,
    /// Best bid size
    pub best_bid_size: f32,
    /// Best ask size
    pub best_ask_size: f32,

    /// Entries here are indexes for size pairs. To be used with `state`
    pub bid_price_points: Vec<u64>,
    /// Indexes for ask-side pairs. Same as `bid_price_points`
    pub ask_price_points: Vec<u64>,

    /// Stores an orderbook order as size pair with price stored as array index. Fast and efficient.
    pub state: Vec<Option<f32>>,
}

impl Default for Book {
    fn default() -> Self {
        Book { 
            market: None,
            asset: None,

            tick_size: 0.0001,
            lot_size: 0.0000_0001, // Default for crypto

            start_seq: 0,
            start_ts: Utc::now(),

            best_bid: 0,
            best_ask: 0,
            best_bid_size: 0.0,
            best_ask_size: 0.0,

            bid_price_points: Vec::new(),
            ask_price_points: Vec::new(),

            state: Vec::new(),
        }
    }
}

impl Book {
    /// Initializes the orderbook from a given snapshot. Most exchanges will send a snapshot of the
    /// orderbook before sending deltas. With that in mind, we can setup the orderbook without much pain
    pub fn initialize(&mut self, snapshot: &Snapshot) {
        let mut bids: Vec<(u64, f32)> = snapshot.bids
            .iter()
            .map(|bid| ((bid.0 / self.tick_size) as u64, bid.1))
            .collect();

        let mut asks: Vec<(u64, f32)> = snapshot.asks
            .iter()
            .map(|ask| ((ask.0 / self.tick_size) as u64, ask.1))
            .collect();

        // Run these here because they return nothing.
        // I made the mistake of trying to do this in a for loop -_-
        bids.sort_by_key(|bid| bid.0);
        asks.sort_by_key(|ask| ask.0);

        // Initialize "empty" vector full of `None` values
        self.state = vec![None; (1.0 / self.tick_size) as usize * 100_000];

        for (idx, (price, size)) in bids.iter().enumerate() {
            // Because we have already set the price to our "standardized format" above, we
            // don't need to perform arithmetic on the price variable.
            self.state[*price as usize] = Some(*size);
            self.bid_price_points.push(*price);

            if idx == bids.len() - 1 {
                // Once we've reached the end of our sorted array, we can declare the best bid and bid size
                self.best_bid = *price;
                self.best_bid_size = *size;
            }
        }
        for (idx, (price, size)) in asks.iter().enumerate() {
            self.state[*price as usize] = Some(*size);
            self.ask_price_points.push(*price);
            if idx == 0 {
                self.best_ask = *price;
                self.best_ask_size = *size;
            }
        }
    }
    /// Handles new orders to be inputted into the orderbook.
    /// Orders can mutate the state of the orderbook. All (normal) orders are
    /// handled through here as a vector, in case multiple elements get passed at once.
    /// 
    /// Order cancelation events can be invoked as follows:
    /// ```
    /// let ob = Book { ..Default::default() };
    /// ob.initialize(&some_deltas);
    /// 
    /// // Create cancelation by nullifying the size of the price level
    /// ob.new_state(vec![
    ///     (20943942, 0.0, true) ]);
    /// ```
    /// TODO: consider putting caches at levels 25%, 50%, and 75% to use as indexing tools for each respective side.
    /// TODO: also consider adding a vector to `Book` that contains price allocations present in the array.
    pub fn new_state(&mut self, updates: &Vec<(u64, f32, bool)>) {
        for (price, size, is_bid) in updates {
            // Save our price as usize to avoid having to cast it everytime we want to access a vector element
            let price_usize = *price as usize;

            if *is_bid {
                // Limit Order: An order that is placed on the orderbook queue and does not affect
                // the ask side of the orderbook (in most cases).

                if *size == 0.0 {
                    // Process cancelation event. We can't ensure that an option will
                    // be put in place, so we have to take it upon ourselves to see that it will.
                    if *price == self.best_bid {
                        // Walk backwards to find the next best bid information.
                        // But first, let's sort the "bid_price_point" vector and get the result below the best bid
                        self.bid_price_points.sort();

                        let level_price = self.bid_price_points[self.bid_price_points.len() - 2];
                        let bid_level_size = self.state[level_price as usize];

                        self.best_bid = level_price;
                        self.best_bid_size = bid_level_size.unwrap();

                        // Remove the best bid from price points
                        self.bid_price_points.pop();

                        // Void the best level bid after having handled best-bid updates (if any)
                        self.state[price_usize] = None;

                    } else {
                        // Void the best level bid after having handled best-bid updates (if any)
                        self.state[price_usize] = None;
                        self.bid_price_points.remove_item(price);
                    }

                } else {
                    // Updates limit order

                    let new_size = Some(*size);
                    self.state[price_usize] = new_size;

                    // Check for duplicates before adding anything to the vector
                    if !self.bid_price_points.iter().any(|p| *p == *price) {
                        self.bid_price_points.push(*price);
                    }

                    if *price == self.best_bid {
                        // Update the `best_bid_size` member to the new size
                        self.best_bid_size = new_size.unwrap();
                        return
                    }
                    if *price > self.best_bid {
                        // If price greater than the best bid, we have a new best bid.
                        self.best_bid = *price;
                        self.best_bid_size = new_size.unwrap();
                    }
                }
            } else {
                // Handle ask order here
                if *size == 0.0 {
                    // Process cancelation event. We can't ensure that an option will
                    // be put in place, so we have to take it upon ourselves to see that it will.
                    if *price == self.best_ask {
                        // First, sort the `ask_price_points` vector
                        self.ask_price_points.sort();
                        
                        let level_price = self.ask_price_points[1];
                        
                        self.best_ask = level_price;
                        self.best_ask_size= self.state[level_price as usize].unwrap();

                        // TODO: This may be inefficient...
                        self.ask_price_points = self.ask_price_points[1..].to_vec();

                        // Void the best level bid after having handled best-bid updates (if any)
                        self.state[price_usize] = None;

                    } else {
                        // Void the best level bid after having handled best-bid updates (if any)
                        self.state[price_usize] = None;
                        self.ask_price_points.remove_item(price);
                    }

                } else {
                    // Updates limit order. Make sure to handle `best_ask` case scenario.
                    let new_size = Some(*size);

                    self.state[price_usize] = new_size;

                    if !self.ask_price_points.iter().any(|p| *p == *price) {
                        self.ask_price_points.push(*price);
                    }

                    if *price == self.best_ask {
                        // Update the `best_bid_size` member to the new size
                        self.best_ask_size = new_size.unwrap();
                        return
                    }
                    if *price < self.best_ask {
                        // If our current ask's price is less than the best, then that becomes the new best ask.
                        self.best_ask = *price;
                        self.best_ask_size = new_size.unwrap();
                    }
                }
            }
        }
    }

    /// Returns a snapshot of the orderbook at the current state. This is very useful for analyzing the orderbook
    /// as it evolves. From snapshot, we can then begin to transform the snapshot into a more meaningful format more
    /// suitable for analysis, such as `SnapshotAnalysis`.
    /// TODO: Consider just making it return a `SnapshotAnalysis` from the get-go instead
    pub fn get_snapshot(&self) -> Snapshot {
        Snapshot {
            market: self.market.as_ref().cloned(),
            asset:  self.asset.as_ref().cloned(),

            bids: { let bids: Vec<(f32, f32)> = self.bid_price_points[..]
                .par_iter()
                .map(|level_price| {
                    (*level_price as f32, self.state[*level_price as usize].unwrap_or(0.0))
                })
                .collect();

                bids
            },

            asks: { let asks: Vec<(f32, f32)> = self.ask_price_points[..]
                .par_iter()
                .map(|level_price| {
                    (*level_price as f32, self.state[*level_price as usize].unwrap_or(0.0))
                })
                .collect();

                asks
            }
        }
    }

    /// Return the "real" price of an asset instead of the array index (i.e. normalize price)
    pub fn real_price(&self, fake_price: u64) -> f32 {
        fake_price as f32 * self.tick_size
    }
    /// Gets bid-ask spread (i.e. `best_ask - best_bid`)
    pub fn bid_ask_spread(&self) -> f32 {
        self.real_price(self.best_ask) - self.real_price(self.best_bid)
    }
    /// Gets mid price (i.e. `(best_ask + best_bid) / 2`)
    pub fn mid_price(&self) -> f32 {
        (self.real_price(self.best_ask) + self.real_price(self.best_bid)) / 2.0
    }
    /// Gets bid-relative price. This tells you how far a given `price` is from the best bid
    pub fn bid_relative_price(&self, price: f32) -> f32 {
        self.real_price(self.best_bid) - price
    }
    /// Gets ask-relative price. This tells you how far a given `price` is from the best ask
    pub fn ask_relative_price(&self, price: f32) -> f32 {
        price - self.real_price(self.best_ask)
    }
}