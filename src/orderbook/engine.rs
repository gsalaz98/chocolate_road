    fn _matching_engine(&mut self, is_bid: bool, market_order: bool, price: u64, size: f32) {}
        /*// Save our price as usize to avoid having to cast it everytime we want to access a vector element
        let price_usize = price as usize;
        // Allocate variable here to avoid re-reading it in the future
        let level_size: f32 = self.state[price as usize].unwrap_or(0.0);

        if is_bid {
            // TODO: Implement best bid/ask calculations into this whole section
            if price < self.best_ask {
                if market_order {
                    // Handles market order in the case that the price is too low
                    return
                }
                // Limit Order: An order that is placed on the orderbook queue and does not affect
                // the ask side of the orderbook (in most cases).

                if level_size == 0.0 {
                    // We can just set our price on the state of the orderbook
                    // without worrying about having to maintain previous orders
                    self.state[price_usize] = Some(size);

                    if price > self.best_bid {
                        self.best_bid = price;
                        self.best_bid_size = size;
                    }
                    
                } else if size == 0.0 {
                    // Process cancelation event. We can't ensure that an optionwill
                    // be put in place, so we have to take it upon ourselves to see that it will.
                    if price == self.best_bid {
                        // Walk backwards to find the next best bid information
                        // TODO: Use range(step) function to get the next possible entry in the
                        // orderbook more quickly. This is quick and dirty and will just work. Rework in future.
                        for (idx, bid) in self.state[ .. self.best_bid as usize].iter().enumerate().rev() {
                            if !bid.is_none() {
                                // Once we encounter a real order, we can update our variables
                                self.best_bid = price - (idx + 1) as u64;
                                self.best_bid_size = bid.unwrap();
                                break;
                            }
                        }
                    }
                    // Void the best level bid after having handled best-bid updates (if any)
                    self.state[price_usize] == None;

                } else {
                    // Updates limit order
                    // Now we have to worry about making sure we don't overwrite the previous state's level,
                    // but rather increment it. Make sure to handle `best_bid` case scenario
                    let previous_size = self.state[price_usize].unwrap();
                    let new_size = Some(previous_size + size);
                    self.state[price_usize] = new_size;

                    if price == self.best_bid {
                        // Update the `best_bid_size` member to the new size
                        self.best_bid_size = new_size.unwrap();
                    }
                }
            } else {
                // Otherwise known as a market order, this type of order should execute immediately
                // We will deincrement this number as we walk down the orderbook. Once it reaches zero,
                // our execution is complete.
                let best_ask_usize = self.best_ask as usize;
                let mut size_executed = size;
                // TODO: This is very very inefficient, but I can't figure out a way to work with the borrow
                // checker. Definitely revisit this in the future once I have more experience.
                let mut book_clone = self.state[price_usize ..= best_ask_usize].to_vec().clone();

                // Walk the orderbook from the starting asking price to the bid order price
                for (idx, entry_level_size) in book_clone.iter().enumerate() {
                    if entry_level_size.is_none() {
                        // We will enevitably encounter empty orderbook positions, so skip
                        // and "search" for another one.
                        continue;
                    }
                    // Avoid having to unwrap it everytime we want to access the value.
                    let entry_size = entry_level_size.unwrap();

                    if entry_size > size_executed { // DONE
                        // If we've cleared this condition, that means that we're at the end of the
                        // execution stage and the current ask-side order is larger than the bid market order
                        // submitted. We can mutate the ask-side size and mark the order as "filled"
                        self.state[best_ask_usize + idx] = Some(entry_size - size_executed);
                        self.best_ask += idx as u64;
                        break;

                    } else if entry_size < size_executed && idx == best_ask_usize + book_clone.len() - 1 { // DONE
                        // We've reached the end of execution, but our order was still larger
                        // than the "market" order it wanted. Let's place it as the new top-level bid
                        // and increment the `best_ask` variable.

                        // Deletes the highest executed order
                        self.state[best_ask_usize + idx] = Some(0.00);

                        // Final size of our new "bid" order
                        size_executed = size_executed - entry_size;
                        self.state[price_usize] = Some(size_executed);

                        // Set the price of our new "ask" to one higher than the one we previously held
                        self.best_ask += idx as u64 + 1;

                        break;

                    } else {
                        // First, calculate how much we're going to execute at this price level
                        size_executed -= entry_size;
                        // Then, delete the lowest-level ask
                        self.state[best_ask_usize + idx] = Some(0.00);
                        // Afterwards, update our ask-price starting point.
                        self.best_ask += idx as u64 + 1;
                    }
                }
            }
        } else {
            // TODO: Finish ask area
            // If you've reached this area, we're dealing with an ask order. We're going to deal with the situation down here
            // the same as we did in the bid section.

            // First, let's handle vanilla ask orders
            if price > self.best_ask {
                self.state[best_ask_usize + ]
            }
        }

        //self.state[price] = size;
    }*/