use clap::{App, Arg};
use anyhow::Result;

pub fn get_argument_tickers() -> Result<(Vec<String>, Vec<String>)> {

    let matches = App::new("Kalshi MDP Server")
        .version("1.1")
        .about("Subscribe to market data on the Kalshi websocket & relay to local redis instance")
        .arg(
            Arg::new("snapshots")
                .long("snapshots")
                .help("List of tickers to subscribe to for orderbook snapshots")
                .takes_value(true)
                .multiple_values(true),
        )
        .arg(
            Arg::new("trades")
                .long("trades")
                .help("List of tickers to subscribe to for trades")
                .takes_value(true)
                .multiple_values(true),
        )
        .get_matches();

    let snapshot_tickers = matches.values_of("snapshots").unwrap_or_default().map(|s| s.to_string()).collect::<Vec<String>>();
    let trade_tickers = matches.values_of("trades").unwrap_or_default().map(|s| s.to_string()).collect::<Vec<String>>();

    Ok((snapshot_tickers, trade_tickers))
}