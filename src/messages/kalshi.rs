use tracing::debug;
use redis_derive::{ToRedisArgs, FromRedisValue};
use serde_json;
use serde::{Deserialize, Serialize};
use websocket::Message;
use anyhow::anyhow;
use std::ops;

use kalshi_mdp_derive::SetTimestamp;

/// Represents a message to send to the Kalshi websocket server.
#[derive(Serialize, Deserialize)]
pub struct KalshiClientMessage {
    id: u32,
    cmd: String,
    params: KalshiClientSubMessage
}

impl KalshiClientMessage {

    /// Convert this client message to a websocket Message
    pub fn to_websocket_message(&self) -> Message {
        Message::text(serde_json::to_string(&self).unwrap())
    }
}

/// A builder for KalshiClientMessages that keeps track of the 
/// id to use for each subsequent message.
pub struct KalshiClientMessageBuilder {
    next_id: u32,
    cmd: Option<String>,
    params: Option<KalshiClientSubMessage>
}

impl KalshiClientMessageBuilder {

    /// Construct a new builder
    pub fn new() -> KalshiClientMessageBuilder {
        KalshiClientMessageBuilder {
            next_id: 1, 
            cmd: None,
            params: None
        }
    }

    /// Set the SubMessage for the next message to build.
    pub fn content(&mut self, submsg: KalshiClientSubMessage) -> &mut Self {
        match submsg {
            KalshiClientSubMessage::SubscribeSubMessage(ref _msg) => {
                self.cmd = Some("subscribe".into());
                self.params = Some(submsg);
            },
            _ => {}
        }
        self
    }

    /// Construct a KalshiClientMessage from self's current state
    pub fn build(&mut self) -> KalshiClientMessage {
        let message = KalshiClientMessage {
            id: self.next_id,
            cmd: self.cmd.clone().unwrap_or_default(),
            params: self.params.take().unwrap(),
        };
        self.next_id += 1;
        message
    }
}

/// 
/// A sub-message of a KalshiClientMessage.
/// Kalshi messages are structured so that each type of message has 
/// identical structure but for what is under the 'params' field.
/// 
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum KalshiClientSubMessage {
    SubscribeSubMessage(SubscribeSubMessage),
    UnsubscribeSubMessage(UnsubscribeSubMessage),
    UpdateSubMessage(UpdateSubMessage)
}

/// A sub-message representing a subscription request.
#[derive(Serialize, Deserialize)]
pub struct SubscribeSubMessage {
    channels: Vec<String>,
    market_tickers: Vec<String>
}

impl SubscribeSubMessage {

    /// Construct a new subscription message with the default 'orderbook_delta' 
    /// and 'ticker' channels.
    pub fn new_deltas(tickers: Vec<String>) -> SubscribeSubMessage {
        SubscribeSubMessage { 
            channels: vec!["orderbook_delta".into(), "ticker".into()], 
            market_tickers: tickers 
        }
    }

    /// Construct a new subscription message for trades on the given 'ticker'
    /// channels.
    pub fn new_trades(tickers: Vec<String>) -> SubscribeSubMessage {
        SubscribeSubMessage { 
            channels: vec!["trade".into(), "ticker".into()], 
            market_tickers: tickers 
        }
    }

    pub fn new_snapshot_and_trades((snaps, trades): (Vec<String>, Vec<String>)) -> Vec<SubscribeSubMessage> {
        let snapshot_sub = SubscribeSubMessage::new_deltas(snaps);
        let trade_sub = SubscribeSubMessage::new_trades(trades);
        vec![snapshot_sub, trade_sub]
    }
}

/// A sub-message representing a request to unsubscribe a previous subscription.
#[derive(Serialize, Deserialize)]
pub struct UnsubscribeSubMessage {
    sids: Vec<u32>
}

/// A sub-message representing a request to update an existing subscription.
#[derive(Serialize, Deserialize)]
pub struct UpdateSubMessage {
    sids: Vec<u32>,
    market_tickers: Vec<String>,
    action: String
}

/// A kalshi market data message
#[derive(Serialize, Deserialize)]
pub struct MarketDataMessage {
    #[serde(rename="type")]
    msg_type: String,
    sid: u32,
    seq: u32,
    pub msg: MarketDataSubMessage
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MarketDataSubMessage {
    Snapshot(Snapshot),
    Delta(Delta),
    Trade(Trade)
}

pub trait SetTimestamp {
    fn set_timestamp(&self) -> Self;
}

/// A delta message, i.e. a message containing a change in quantity 
/// offered at a specific price level
#[derive(Serialize, Deserialize, Debug, Clone, SetTimestamp)]
pub struct Delta {
    pub market_ticker: String,
    pub ts: Option<u64>,
    pub price: i32,
    pub delta: i32,
    pub side: Side
}

#[derive(Serialize, Deserialize, ToRedisArgs, FromRedisValue, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    YES,
    NO
}

/// A snapshot message, i.e. a view of the full book on a ticker
#[derive(Serialize, Deserialize, ToRedisArgs, FromRedisValue, Debug, Clone, SetTimestamp)]
pub struct Snapshot {
    pub market_ticker: String,
    pub ts: Option<u64>,
    pub yes: Vec<(i32, i32)>,
    pub no: Vec<(i32, i32)>,
}

/// A trade message, i.e. a message containing a trade that has
/// occurred on a ticker
#[derive(Serialize, Deserialize, Debug, Clone, SetTimestamp)]
pub struct Trade {
    pub market_ticker: String,
    pub ts: Option<u64>,
    pub yes_price: i32,
    pub no_price: i32,
    pub count: i32,
    pub taker_side: Side
}

/// Overloads '+' for Snapshot + Delta
impl ops::Add<Delta> for Snapshot {
    type Output = Snapshot;

    /// Adds an orderbook Delta to this Snapshot
    fn add(self, rhs: Delta) -> Self::Output {

        /// Add 'delta' to the quantity at 'price' in 'levels' 
        fn apply_delta(levels: Vec<(i32, i32)>, price: i32, delta: i32) -> Vec<(i32, i32)> {
            let mut new = levels.clone();
            for (idx, level) in levels.iter().enumerate() {
                if level.0 == price {
                    new[idx] = (price, level.1 + delta);
                    return new;
                }
            }

            // push and re-sort if price is not yet present
            new.push((price, delta));
            new.sort_by(|a, b| a.1.cmp(&b.1));
            new
        }

        debug!("Adding {rhs:?} to {self:?}");
        match rhs.side {
            Side::YES => Snapshot { 
                market_ticker: self.market_ticker, 
                yes: apply_delta(self.yes, rhs.price, rhs.delta), 
                no: self.no,
                ts: rhs.ts // change snapshot's ts to delta's ts
            },
            Side::NO => Snapshot { 
                market_ticker: self.market_ticker, 
                yes: self.yes, 
                no: apply_delta(self.no, rhs.price, rhs.delta),
                ts: rhs.ts
            }
        }
    }
}
