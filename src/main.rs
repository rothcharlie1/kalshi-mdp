use reqwest;
use serde_json;
use serde::{Deserialize, Serialize};

use std::error::Error;

mod constants;


#[derive(Serialize, Deserialize)]
struct LoginResponse {
    member_id: String,
    token: String
}

#[derive(Serialize, Deserialize)]
struct LoginBody {
    email: String,
    password: String
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let token = get_token().await.expect("Could not get a token.").token;
    println!("{}", token);

}

async fn get_token() -> Result<LoginResponse, Box<dyn Error>>  {
    let client = reqwest::Client::new();
    let data = LoginBody {
        email: constants::USER.into(), 
        password: constants::PW.into()
    };
    let response_text =  client.post(constants::PROD_API)
        .body(serde_json::to_string(&data).unwrap())
        .header("Content-Type", "application/json")
        .header("accept", "application/json")
        .send()
        .await?
        .text()
        .await?;

    Ok(serde_json::from_str(&response_text).unwrap())
}
