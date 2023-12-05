extern crate websocket;
use kalshi_wss::KalshiClientMessageBuilder;
use native_tls::TlsStream;
use websocket::sync::Client;
use websocket::{ClientBuilder, OwnedMessage, CloseData, Message};
use websocket::header::{Headers, Authorization, Bearer};
use redis::Client as RedisClient;
use redis::Connection;
use std::error::Error;
use std::net::TcpStream;
use log::{debug, error, info};

use crate::kalshi_wss::SubscribeSubMessage;
use crate::kalshi_wss::KalshiClientSubMessage as SubMessage;
use crate::kalshi_wss::{OrderbookMessage, Delta, Snapshot};
use crate::redis_utils::RedisOrderbookClient;

mod kalshi_http;
mod constants;
mod kalshi_wss;
mod redis_utils;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

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
    let sub_sub_msg = SubscribeSubMessage::new_default(vec!["INXDU-23DEC05-T4574.99".into()]);

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
            OwnedMessage::Text(s) => handle_received_text(s, &mut redis_conn)?,
            OwnedMessage::Binary(b) => debug!("Received and ignored binary data."),
            OwnedMessage::Close(close_data) => {
                info!("Websocket closed by server for reason: {}", close_data.unwrap().reason);
                break;
            },
            OwnedMessage::Ping(data) => match client.send_message(&Message::pong(data)) {
                Ok(()) => (),
                Err(e) => panic!("Failed to send pong with error {e:?}") 
            },
            OwnedMessage::Pong(data) => {} // as a client, we do not expect to receive pongs
        }
    }
    Ok(())
}

/// Handle text messages received from the server
fn handle_received_text(text: String, redis_conn: &mut RedisOrderbookClient) -> Result<(), anyhow::Error> {
    let wrapper_msg = match serde_json::from_str::<OrderbookMessage>(&text) {
        Ok(msg) => msg,
        Err(e) => {
            debug!("Ignoring non-OrderbookMessage text data.");
            return Ok(())
        }
    };

    redis_conn.write(wrapper_msg.msg)
}
