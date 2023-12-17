#[allow(unused_imports)]
mod messages;
mod sinks;
mod constants;

use crate::messages::kalshi::{Snapshot, Trade};
use crate::sinks::redis::RedisClient;

fn main() -> Result<(), anyhow::Error> {

    let args: Vec<String> = std::env::args().collect();

    // Check if user has provided the correct number of arguments
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
        let trades: Vec<Trade> = redis_client.read_trades(&ticker).expect("No trade data found for ticker.");
        println!("{trades:?}");
    } else {
        println!("Invalid type provided. Please select either --snapshot or --trade.");
    }

    Ok(())
}