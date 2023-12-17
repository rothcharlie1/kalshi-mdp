use reqwest;
use serde_json;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use websocket::header::{Headers, Authorization, Bearer};

use crate::constants;

/// The response from Kalshi to a login request.
#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    member_id: String,
    pub token: String
}

/// The body of data required to login to Kalshi.
#[derive(Serialize, Deserialize)]
pub struct LoginBody {
    email: String,
    password: String
}

impl LoginBody {
    /// Construct a new LoginBody
    pub fn new(email: String, password: String) -> LoginBody {
        LoginBody { email: email, password: password }
    }
}

/// Logs in to Kalshi via HTTP and returns the response from Kalshi as a LoginResponse.
pub async fn login(url: &str, body: LoginBody) -> Result<LoginResponse>  {
    let client = reqwest::Client::new();
    let response_text =  client.post(url)
        .body(serde_json::to_string(&body).unwrap())
        .header("Content-Type", "application/json")
        .header("accept", "application/json")
        .send()
        .await?
        .text()
        .await?;

    Ok(serde_json::from_str(&response_text).unwrap())
}

pub async fn set_websocket_headers(headers: &mut Headers) -> Result<()> {

    let token = login(
        constants::PROD_API, 
        LoginBody::new(
            constants::USER.to_string(), 
            constants::PW.to_string()))
        .await
        .expect("Could not get a token.")
        .token;
    headers.set(Authorization(Bearer {
        token: token.to_owned(),
    }));
    Ok(())
}