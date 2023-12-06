## Kalshi Market Data Platform

A hot-configurable Redis sink for the Kalshi Websockets API. Achieves sub-millisecond wire-to-wire writes.

This project builds two binaries: the ```kalshi-mdp-server``` that provides the core functionality, and the ```kalshi-mdp-client```, a basic shell tool to retrieve snapshots from Redis.

### Requirements
- A Redis server running locally
- A Kalshi account to connect with, configured in ```src/constants.rs```

### Build

From the root directory, build with Cargo: ```cargo build --release```

To build the debug or development release, run ```cargo build```.

### Server

The server binary accepts space-delineated Kalshi tickers to listen to data on.

Example: ```./target/release/kalshi-mdp-server <ticker 1> <ticker 2> ...```

To see logs, run with the ```RUST_LOG``` environment variable: 

```RUST_LOG=<log level> ./target/release/kalshi-mdp-server ...```

### Client

The client binary pulls and displays the latest snapshot for a single ticker. 

Usage: ```./target/release/kalshi-mdp-client <ticker>```

### Dependencies

The developers of kalshi-mdp feel they should thank the developers of the following crates for they play a pivotal role in this application:

- [Rust-WebSocket](https://github.com/websockets-rs/rust-websocket)
- [redis-rs](https://github.com/redis-rs/redis-rs)
- [redis-derive](https://github.com/kkharji/redis-derive)
- [serde](https://github.com/serde-rs/serde)

### Future

The developers welcome pull requests (and will likely contribute themselves) for the following useful features:

- A TCP server to accept live subscription and unsubscription requests
- Improved post-build configuration via YAML, etc.
- Support for alternative storage or message formats, like Kafka


