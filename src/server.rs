#[allow(unused_imports)]

extern crate websocket;
use native_tls::TlsStream;
use websocket::sync::Client;
use websocket::{ClientBuilder, OwnedMessage, Message};
use websocket::header::{Headers, Authorization, Bearer};
use std::net::TcpStream;
use std::env;
use log::{debug, info, trace};

use crate::kalshi_wss::SubscribeSubMessage;
use crate::kalshi_wss::KalshiClientSubMessage as SubMessage;
use crate::kalshi_wss::OrderbookMessage;
use crate::redis_utils::RedisOrderbookClient;

mod kalshi_http;
mod constants;
mod kalshi_wss;
mod redis_utils;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::Builder::from_default_env().format_timestamp_micros().init();

    let tickers: Vec<String> = env::args().collect();

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
    let sub_sub_msg = SubscribeSubMessage::new_default(tickers[1..].to_vec());

    let init_sub_msg = msg_builder.content(SubMessage::SubscribeSubMessage(sub_sub_msg))
        .build();

    info!("Sending initial subscription message: {:?}", serde_json::to_string(&init_sub_msg).unwrap());

    client.send_message(&init_sub_msg.to_websocket_message())?;

    receive_loop(client)
}

/// Loop, wait for messages from the websocket server, and handle each.
fn receive_loop(mut client: Client<TlsStream<TcpStream>>) -> Result<(), anyhow::Error> {

    let mut redis_conn = RedisOrderbookClient::new("redis://127.0.0.1")?;

    loop {
        match client.recv_message().unwrap() {
            OwnedMessage::Text(s) => {
                trace!("Handling incoming text");
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
fn handle_received_text(text: String, redis_conn: &mut RedisOrderbookClient) -> Result<(), anyhow::Error> {
    let wrapper_msg = match serde_json::from_str::<OrderbookMessage>(&text) {
        Ok(msg) => msg,
        Err(_e) => {
            debug!("Ignoring non-OrderbookMessage text data.");
            return Ok(())
        }
    };

    redis_conn.write(wrapper_msg.msg)
}
