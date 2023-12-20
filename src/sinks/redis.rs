use tracing::{debug, error};
use redis::Client;
use redis::Commands;
use redis::Connection;
use anyhow::anyhow;

use crate::messages::kalshi::{MarketDataSubMessage, Snapshot, Delta, Trade, SetTimestamp};

/// A wrapper for a Redis client that supports orderbook snapshot and trade retrieval
pub struct RedisClient {
    conn: Connection
}

impl RedisClient {

    /// Construct a new client by connecting to Redis at the provided host
    pub fn new(host: &str) -> Result<RedisClient, anyhow::Error> {
        let client = Client::open(host)?;
        let conn = client.get_connection()?;
        Ok(
            RedisClient {
                conn: conn
            }
        )
    }

    /// Retrieve the latest snapshot on 'ticker'
    pub fn read_snapshot(&mut self, ticker: &str) -> Result<Snapshot, anyhow::Error> {
        Ok(redis::cmd("HGETALL")
            .arg(ticker)
            .query::<Snapshot>(&mut self.conn)?)
    }

    /// TODO: Retrieve the latest trades on 'ticker'
    pub fn read_trades(&mut self, ticker: &str) -> Result<Vec<Trade>, anyhow::Error> {
        Err(anyhow!("read_trades method not implemented"))
    }

    /// Delegating writing of snapshots or deltas to the correct functionality
    pub fn write(&mut self, msg: MarketDataSubMessage) -> Result<(), anyhow::Error> {
        match msg {
            MarketDataSubMessage::Delta(d) => self.write_delta(d),
            MarketDataSubMessage::Snapshot(s) => self.write_snapshot(s),
            MarketDataSubMessage::Trade(t) => self.write_trade(t)
        }
    }

    /// Write an orderbook snapshot to Redis
    fn write_snapshot(&mut self, snap: Snapshot) -> Result<(), anyhow::Error> {
        let ticker: &str = &snap.market_ticker.clone();
        let to_write = snap.set_timestamp();

        self.clear_key(ticker);
        match redis::cmd("HSET")
            .arg(ticker)
            .arg(&to_write)
            .query::<i32>(&mut self.conn) {
                Ok(_t) => {
                    debug!("Wrote snapshot for {} to Redis", ticker);
                    Ok(())
                },
                Err(_e) => {
                    // error log the line: "Encountered redis error when writing snapshot: {:?}"
                    error!("Encountered redis error when writing snapshot: {:?}", _e);
                    Ok(())
                }
            }
    }

    /// Retrieve an existing snapshot, apply the delta, and rewrite to Redis
    fn write_delta(&mut self, delta: Delta) -> Result<(), anyhow::Error> {
        let curr_snapshot: Snapshot = self.conn.hgetall(&delta.market_ticker)?;
        let next_snapshot: Snapshot = curr_snapshot + delta.set_timestamp();
        self.write_snapshot(next_snapshot)
    }

    /// TODO: Write a trade to Redis
    fn write_trade(&mut self, trade: Trade) -> Result<(), anyhow::Error> {
        let trade = trade.set_timestamp();
        debug!("Logging Trade: {:?}", trade);
        Ok(())
    }

    /// Delete Redis entry under the provided key
    fn clear_key(&mut self, key: &str) {
        let _ = redis::cmd("DEL")
            .arg(key)
            .query::<Snapshot>(&mut self.conn);
    }
}