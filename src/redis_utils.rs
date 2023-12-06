use log::debug;
use redis::Client;
use redis::Commands;
use redis::Connection;

use crate::kalshi_wss::OrderbookSubMessage;
use crate::kalshi_wss::{Snapshot, Delta};

/// A wrapper for a Redis client with orderbook-specific functions.
pub struct RedisOrderbookClient {
    conn: Connection
}

impl RedisOrderbookClient {

    /// Construct a new client by connecting to Redis at the provided host
    pub fn new(host: &str) -> Result<RedisOrderbookClient, anyhow::Error> {
        let client = Client::open(host)?;
        let conn = client.get_connection()?;
        Ok(
            RedisOrderbookClient {
                conn: conn
            }
        )
    }

    /// Delegating writing of snapshots or deltas to the correct functionality
    pub fn write(&mut self, msg: OrderbookSubMessage) -> Result<(), anyhow::Error> {
        match msg {
            OrderbookSubMessage::Delta(d) => self.write_delta(d),
            OrderbookSubMessage::Snapshot(s) => self.write_snapshot(s)
        }
    }

    /// Write an orderbook snapshot to Redis
    fn write_snapshot(&mut self, snap: Snapshot) -> Result<(), anyhow::Error> {
        let ticker: &str = &snap.market_ticker.clone();

        self.clear_key(ticker);
        match redis::cmd("HSET")
            .arg(ticker)
            .arg(&snap)
            .query::<Snapshot>(&mut self.conn) {
                Ok(_t) => {
                    debug!("Wrote snapshot for {} to Redis", ticker);
                    Ok(())
                },
                Err(_e) => {
                    debug!("Ignoring Redis error.");
                    Ok(())
                }
            }
    }

    /// Retrieve an existing snapshot, apply the delta, and rewrite to Redis
    fn write_delta(&mut self, delta: Delta) -> Result<(), anyhow::Error> {
        let curr_snapshot: Snapshot = self.conn.hgetall(&delta.market_ticker)?;
        let next_snapshot: Snapshot = curr_snapshot + delta;
        self.write_snapshot(next_snapshot)
    }

    /// Delete Redis entry under the provided key
    fn clear_key(&mut self, key: &str) {
        let _ = redis::cmd("DEL")
            .arg(key)
            .query::<Snapshot>(&mut self.conn);
    }
}