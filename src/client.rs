#[allow(unused_imports)]

use kalshi_wss::Snapshot;
use kalshi_wss::Trade;
use redis_utils::RedisClient;
use clap::{App, Arg};

pub mod redis_utils;
pub mod kalshi_wss;
pub mod constants;

fn main() -> Result<(), anyhow::Error> {

    let args: Vec<String> = std::env::args().collect();

    /// Check if user has provided the correct number of arguments
    if args.len() != 3 {
        eprintln!("Error: Please provide a data type flag (--snapshot, --ticker) and ticker.");
        std::process::exit(1);
    }

    let mode = &args[1];
    let ticker = &args[2];

    let mut redis_client = RedisClient::new("redis://127.0.0.1")?;

    if mode == "--snapshot" {
        let snap: Snapshot = redis_client.read_snapshot(&ticker).expect("No snapshot data found for ticker.");
        println!("{snap:?}");
    } else if mode == "--trade" {
        let trade: Trade = redis_client.read_trade(&ticker).expect("No trade data found for ticker.");
        println!("{trade:?}");
    } else {
        println!("Invalid type provided. Please select either --snapshot or --trade.");
    }

    Ok(())
}