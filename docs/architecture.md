# Sentinel Phase 2: Autonomous Mesh Architecture

Sentinel has evolved from a basic client-server model into a **Symmetric Peer-to-Peer Mesh**. In this architecture, there is no "Server" or "Client" role; every node acts as both a consumer and a provider of data.



## 1. The Trustless Mesh Model
Sentinel Phase 2 implements a **Leaderless Mesh** with **Trust-on-First-Use (TOFU)** verification.
* **Symmetric Handshaking**: Whether you dial out or receive an inbound connection, both parties perform an identical cryptographic identity exchange.
* **Identity Pinning**: Nodes are identified by their Ed25519 Public Keys. Once a key is verified, it is "pinned" to that peer's address in the `DashMap` state.
* **Autonomous Discovery**: Nodes use mDNS (Multicast DNS) to actively shout their presence and browse for others, removing the need for static IP configuration.

## 2. Updated Node Stack
The node is now an integrated engine built on four specialized crates:

1.  **Identity Layer (`sentinel-crypto`)**: 
    - **Ed25519**: Generates high-entropy keypairs.
    - **Deterministic ID**: Node IDs are hex-encoded fingerprints of the Public Key.
2.  **Transport Layer (`sentinel-transport`)**: 
    - **Danger-Verifier TLS**: Uses TLS 1.3 for wire-encryption while bypassing CA-checks to support P2P self-signed identities.
    - **Asynchronous IO**: Powered by `tokio-rustls`.
3.  **Protocol Layer (`sentinel-protocol`)**:
    - **Length-Prefixed Framing**: Prevents TCP stream fragmentation.
    - **Cryptographic Envelopes**: Every message is signed by the sender's private key.
4.  **Engine & Storage Layer (`sentinel-node`)**:
    - **Sled DB**: Embedded ACID-compliant database for message and peer persistence.
    - **Gossip Service**: Periodically synchronizes state across the mesh.

## 3. The Lifecycle of a Peer Connection
1.  **Discovery**: `discovery.rs` hears an mDNS packet and triggers `engine::dial_peer`.
2.  **Encryption**: `sentinel-transport` establishes an encrypted TLS 1.3 tunnel.
3.  **Handshake**: Nodes exchange `MessageContent::Handshake` containing their Public Keys.
4.  **Verification**: The Engine verifies the digital signature of the handshake. If valid, the peer is added to the active `DashMap`.
5.  **Gossip**: The new peer receives a broadcast of any messages missed during downtime.