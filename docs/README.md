# 🛡️ Sentinel-rs

**A high-performance, decentralized P2P VPN and communication engine built in Rust.**

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Sentinel is a peer-to-peer (P2P) networking stack designed to bypass the "Cloud Tax." It enables direct, encrypted communication between nodes even when they are behind restrictive NATs (Network Address Translation) or firewalls, without relying on expensive centralized relay servers.



## 🚀 Key Features

* **Fighter Sockets:** Custom socket logic using `SO_REUSEADDR` and `SO_REUSEPORT` to perform TCP Hole Punching, allowing nodes to dial out and listen on the same identity port.
* **Encrypted by Default:** Every connection is upgraded to a TLS 1.3 tunnel using `rustls`, ensuring total privacy and forward secrecy.
* **Decentralized Discovery:** Combines local mDNS discovery with a lightweight Signaler (Matchmaker) for global connectivity.
* **Gossip Protocol:** Nodes share peer information automatically, building a resilient mesh network that heals itself.
* **Embedded Persistence:** High-performance message logging and state management using the `sled` Key-Value store.

## 🛠️ Technical Deep Dive: The "Fighter Socket"

Most P2P applications fail because home routers block incoming connections. Sentinel overcomes this using **TCP Simultaneous Open**. 

By hijacking the local listening port and initiating outbound connection attempts via non-blocking I/O, Sentinel "punches" a hole through the NAT. The router is tricked into believing the incoming peer connection is a response to our own outbound request.



## 🚦 Getting Started

### Prerequisites
* Rust (latest stable)
* OpenSSL (for certain cryptographic dependencies)

### Installation
```bash
git clone [https://github.com/rustyRadon/sentinel-rs.git](https://github.com/rustyRadon/sentinel-rs.git)
cd sentinel-rs
cargo build --release

## Running a Node
###Start the Signaler (The Matchmaker):
cargo run -p sentinel-signaler

### Start Node A:
cargo run -p sentinel-node -- --port 8443 --data-dir ./.nodeA

### Start Node B and Dial Node A:
cargo run -p sentinel-node -- --port 8444 --data-dir ./.nodeB
# Inside the terminal:
/dial <NODE_A_ID>


🤝 Contributing
Contributions are welcome! If you're interested in low-level networking, VPN protocols, or distributed systems, feel free to fork the repo and submit a PR.

Built with ❤️ and 🦀 by rustyRadon