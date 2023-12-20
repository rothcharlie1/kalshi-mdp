#[allow(unused_imports)]

mod messages;
mod sinks;
mod auth;
mod constants;
mod views;

extern crate websocket;
use native_tls::TlsStream;
use websocket::sync::Client;
use websocket::{ClientBuilder, OwnedMessage, Message};
use websocket::header::Headers;
use std::net::TcpStream;
use tracing::{info, debug, error, trace, Level};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::util::SubscriberInitExt;

use crate::messages::kalshi::SubscribeSubMessage;
use crate::messages::kalshi::KalshiClientSubMessage as SubMessage;
use crate::messages::kalshi::MarketDataMessage;
use crate::messages::kalshi::KalshiClientMessageBuilder;
use crate::sinks::redis::RedisClient;
use crate::auth::kalshi;
use crate::views::clap;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    //env_logger::Builder::from_default_env().format_timestamp_micros().init();
    let file_appender = tracing_appender::rolling::daily(constants::LOG_PATH, "kalshi-mdp.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG) // Set the max log level
        .with_span_events(FmtSpan::CLOSE) // Include span close events
        .with_writer(non_blocking)
        .finish()
        .init();

    let mut custom_headers = Headers::new();
    kalshi::set_websocket_headers(&mut custom_headers).await?;
    let mut client = ClientBuilder::new(constants::PROD_WSS)
        .unwrap()
        .custom_headers(&custom_headers)
        .connect_secure(None) // Connect with TLS
        .unwrap();

    let mut msg_builder = KalshiClientMessageBuilder::new();

    let ticker_set = clap::get_argument_tickers()?;
    let subscribe_sub_messages = SubscribeSubMessage::new_snapshot_and_trades(ticker_set);

    for message in subscribe_sub_messages.into_iter() {
        let to_send = msg_builder.content(SubMessage::SubscribeSubMessage(message))
            .build();
        match client.send_message(&to_send.to_websocket_message()) {
            Ok(()) => {},
            Err(e) => panic!("Failed to send subscribe message with error {e:?}") 
        }
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
            error!("Failed to write to Redis for unknown reason");
            Ok(())
        }
    }
}
