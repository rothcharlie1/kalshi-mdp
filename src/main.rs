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
use log::debug;

use crate::kalshi_wss::SubscribeSubMessage;
use crate::kalshi_wss::KalshiClientSubMessage as SubMessage;
use crate::kalshi_wss::OrderbookDeltaMessage;
use crate::kalshi_wss::OrderbookSnapshotMessage;

mod kalshi_http;
mod constants;
mod kalshi_wss;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let token = kalshi_http::login(
        constants::DEMO_API, 
        kalshi_http::LoginBody::new(
            constants::USER.to_string(), 
            constants::DEMO_PW.to_string()))
        .await
        .expect("Could not get a token.")
        .token;

    let mut custom_headers = Headers::new();
    custom_headers.set(Authorization(Bearer {
        token: token.to_owned(),
    }));

    let mut client = ClientBuilder::new(constants::DEMO_WSS)
        .unwrap()
        .custom_headers(&custom_headers)
        .connect_secure(None) // Connect with TLS
        .unwrap();

    let mut msg_builder = kalshi_wss::KalshiClientMessageBuilder::new();
    let sub_sub_msg = SubscribeSubMessage::new_default(vec!["INXDU-23DEC04-T4574.99".into()]);

    let init_sub_msg = msg_builder.content(SubMessage::SubscribeSubMessage(sub_sub_msg))
        .build();

    debug!("{:?}", init_sub_msg.to_websocket_message());

    client.send_message(&init_sub_msg.to_websocket_message())?;

    receive_loop(client)
}

/// Loop, wait for messages from the websocket server, and handle each.
fn receive_loop(mut client: Client<TlsStream<TcpStream>>) -> Result<(), Box<dyn Error>> {

    // let redis_client = match RedisClient::open("redis://127.0.0.1") {
    //     Ok(client) => client,
    //     Err(e) => panic!("Could not connect to Redis with {e:?}!")
    // };
    // let redis_connection = redis_client.get_connection().unwrap();

    loop {
        match client.recv_message().unwrap() {
            OwnedMessage::Text(s) => handle_received_text(s)?,
            OwnedMessage::Binary(b) => println!("got binary"),
            OwnedMessage::Close(close_data) => {
                println!("closed by server for reason: {}", close_data.unwrap().reason);
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
fn handle_received_text(text: String) -> Result<(), Box<dyn Error>> {
    match serde_json::from_str::<OrderbookSnapshotMessage>(&text) {
        Ok(snapshot) => {},
        Err(e) => {},
    }

    println!("{text}");
    Ok(())
}


