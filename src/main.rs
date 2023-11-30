extern crate websocket;
use websocket::{ClientBuilder, OwnedMessage};
use websocket::header::{Headers, Authorization, Bearer};

mod kalshi_http;
mod constants;

#[tokio::main]
async fn main() {
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

    match client.recv_message().unwrap() {
        OwnedMessage::Text(s) => println!("{}", s),
        OwnedMessage::Ping(ping_data) => println!("got pinged"),
        _ => println!("not text")
    }

}


