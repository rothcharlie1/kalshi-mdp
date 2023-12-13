#[allow(unused_imports)]

extern crate websocket;
use clap::{App, Arg};
use native_tls::TlsStream;
use websocket::sync::Client;
use websocket::{ClientBuilder, OwnedMessage, Message};
use websocket::header::{Headers, Authorization, Bearer};
use std::net::TcpStream;
use std::env;
use log::{debug, info, trace};

use crate::kalshi_wss::SubscribeSubMessage;
use crate::kalshi_wss::KalshiClientSubMessage as SubMessage;
use crate::kalshi_wss::MarketDataMessage;
use crate::redis_utils::RedisClient;

mod kalshi_http;
mod constants;
mod kalshi_wss;
mod redis_utils;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_default_env().format_timestamp_micros().init();

    let matches = App::new("Kalshi MDP Server")
        .version("1.1")
        .about("Subscribe to market data on the Kalshi websocket & relay to local redis instance")
        .arg(
            Arg::new("snapshots")
                .long("snapshots")
                .help("List of tickers to subscribe to for orderbook snapshots")
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("trades")
                .long("trades")
                .help("List of tickers to subscribe to for trades")
                .takes_value(true)
                .multiple_values(true),
        )
        .get_matches();

    let snapshot_tickers = matches.values_of("snapshots").unwrap_or_default().collect::<Vec<_>>();
    let trade_tickers = matches.values_of("trades").unwrap_or_default().collect::<Vec<_>>();

    let token = kalshi_http::login(
        constants::PROD_API, 
        kalshi_http::LoginBody::new(
            constants::USER.to_string(), 
            constants::PW.to_string()))
        .await
        .expect("Could not get a token.")
        .token;

    let mut custom_headers = Headers::new();
    custom_headers.set(Authorization(Bearer {
        token: token.to_owned(),
    }));

    let mut client = ClientBuilder::new(constants::PROD_WSS)
        .unwrap()
        .custom_headers(&custom_headers)
        .connect_secure(None) // Connect with TLS
        .unwrap();

    let mut msg_builder = kalshi_wss::KalshiClientMessageBuilder::new();

    // Subscribe to snapshots on user-specified tickers
    if !snapshot_tickers.is_empty() {
        let snapshot_sub_msg = SubscribeSubMessage::new_default(snapshot_tickers);
        let init_sub_msg = msg_builder.content(SubMessage::SubscribeSubMessage(snapshot_sub_msg))
            .build();

        info!("Sending initial snapshot subscription message: {:?}", serde_json::to_string(&init_sub_msg).unwrap());
        client.send_message(&init_sub_msg.to_websocket_message())?;
    }

    // Subscribe to trades on user-specified tickers
    if !trade_tickers.is_empty() {
        let trade_sub_msg = SubscribeSubMessage::new_trades(trade_tickers);
        let init_sub_msg = msg_builder.content(SubMessage::SubscribeSubMessage(trade_sub_msg))
            .build();

        info!("Sending initial trades subscription message: {:?}", serde_json::to_string(&init_sub_msg).unwrap());
        client.send_message(&init_sub_msg.to_websocket_message())?;
    }

    receive_loop(client)
}

/// Loop, wait for messages from the websocket server, and handle each.
fn receive_loop(mut client: Client<TlsStream<TcpStream>>) -> Result<(), anyhow::Error> {

    let mut redis_conn = RedisClient::new("redis://127.0.0.1")?;

    loop {
        match client.recv_message().unwrap() {
            OwnedMessage::Text(s) => {
                trace!("Handling incoming text");
                trace!("{s}");
                handle_received_text(s, &mut redis_conn)?
            },
            OwnedMessage::Binary(_b) => debug!("Received and ignored binary data."),
            OwnedMessage::Close(close_data) => {
                info!("Websocket closed by server for reason: {}", close_data.unwrap().reason);
                break;
            },
            OwnedMessage::Ping(data) => match client.send_message(&Message::pong(data)) {
                Ok(()) => trace!("Sent pong in response to ping"),
                Err(e) => panic!("Failed to send pong with error {e:?}") 
            },
            OwnedMessage::Pong(_data) => {} // as a client, we do not expect to receive pongs
        }
    }
    Ok(())
}

/// Handle text messages received from the server
fn handle_received_text(text: String, redis_conn: &mut RedisClient) -> Result<(), anyhow::Error> {
    let wrapper_msg = match serde_json::from_str::<MarketDataMessage>(&text) {
        Ok(msg) => msg,
        Err(_e) => {
            debug!("Ignoring non-MarketDataMessage text data.");
            return Ok(())
        }
    };

    match redis_conn.write(wrapper_msg.msg) {
        Ok(()) => Ok(()),
        Err(_e) => {
            debug!("Failed to write to Redis for unknown reason");
            Ok(())
        }
    }
}
