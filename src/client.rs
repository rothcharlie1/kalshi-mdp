#[allow(unused_imports)]

use kalshi_wss::Snapshot;
use redis_utils::RedisOrderbookClient;

pub mod redis_utils;
pub mod kalshi_wss;
pub mod constants;

fn main() -> Result<(), anyhow::Error> {

    let mut redis_client = RedisOrderbookClient::new("redis://127.0.0.1")?;
    let ticker = std::env::args().nth(1).expect("No ticker provided.");

    let snap: Snapshot = redis_client.read(&ticker).expect("No data found for ticker.");

    println!("{snap:?}");
    Ok(())
}