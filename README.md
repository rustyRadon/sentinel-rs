# Sentinel-rs  (V1: Handshake)

**Sentinel-rs** is a decentralized, peer-to-peer (P2P) communication node. Unlike traditional apps, there is no central server. Every node is equal, discovering its peers via local radio (mDNS) and establishing encrypted tunnels for secure data exchange.

> **Status: Phase 1 (Complete)** - Local Discovery & Encrypted Messaging

##  Phase 1 Achievements
- **Zero-Config Discovery**: Using mDNS to "shout" presence on the local network. No IP addresses required.
- **Mutual TLS 1.3**: Every connection is encrypted with industrial-grade TLS, ensuring only nodes with valid certificates can talk.
- **Identity-First Addressing**: Nodes are identified by their unique Public Key fingerprints, not transient IP addresses.
- **Persistent Memory**: Integrated `Sled` database to store chat history locally.
- **Modular Engine**: Split into `engine`, `discovery`, `handlers`, and `transport` for high scalability.

##  Architecture


Sentinel V1 follows a **Leaderless Mesh** architecture:
1. **The Brain (`engine.rs`)**: Manages the peer map and database.
2. **The Ears (`discovery.rs`)**: Listens for mDNS signals from other nodes.
3. **The Voice (`handlers.rs`)**: Manages user input (stdin) and broadcasts to the network.
4. **The Shield (`sentinel-transport`)**: Handles the TLS 1.3 handshakes.

##  Getting Started

### 1. Prerequisites
Ensure you have the Rust toolchain installed and `openssl` for certificate generation.

### 2. Setup Identity
Each node needs a certificate to identify itself:
```bash
openssl req -x509 -newkey rsa:4096 -keyout node.key -out node.crt -days 365 -nodes -subj "/CN=sentinel-node"
mkdir -p .sentinel
mv node.key node.crt .sentinel/