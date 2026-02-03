# Sentinel-rs (Phase 2: The Secure Mesh)

**Sentinel-rs** is a decentralized, leaderless peer-to-peer (P2P) communication engine. In this architecture, identity is not granted by a central authority or a username—it is derived from pure mathematics. Every node is its own sovereign identity, discovering peers via local radio (mDNS) and establishing end-to-end encrypted tunnels.

> **Status: Phase 2 (Complete)** - Cryptographic Handshakes & Autonomous Discovery

## 🚀 Phase 2 Achievements
- **Cryptographic Sovereignty**: Replaced traditional CA-based TLS with **Ed25519 Public Key Pinning**. Trust is based on keys, not third-party issuers.
- **Autonomous Discovery**: Integrated mDNS (Multicast DNS) for "Zero-Config" connectivity. Nodes find each other on the local network automatically.
- **Hybrid Security Model**:
    * **TLS 1.3 Layer**: Provides high-speed stream encryption to prevent eavesdropping.
    * **Ed25519 Layer**: Provides identity verification, ensuring the person you are talking to is the owner of the Private Key.
- **Persistence Layer**: Integrated `Sled` (embedded database) for high-performance message logging and identity storage.
- **Concurrent Engine**: Fully asynchronous architecture using `Tokio`, allowing a single node to manage dozens of peer connections simultaneously.

## 🛠 Architecture



Sentinel-rs is organized into a modular crate system:
1.  **`sentinel-crypto`**: The root of trust. Handles Ed25519 key generation, signing, and verification.
2.  **`sentinel-transport`**: The secure pipe. Implements the "Dangerous" TLS verifier to allow peer-to-peer encrypted tunnels.
3.  **`sentinel-protocol`**: The shared language. Defines the framing and message structures for handshakes and chat.
4.  **`sentinel-node (Engine)`**: The coordinator. Manages the state, coordinates between discovery and the UI, and handles message persistence.

## 🚦 Getting Started

### 1. Prerequisites
Ensure you have the Rust toolchain installed.
```bash
curl --proto '=https' --tlsv1.2 -sSf [https://sh.rustup.rs](https://sh.rustup.rs) | sh

### 2. Running a Local Mesh
To simulate a network on a single machine, open three separate terminal windows and run each command:

**Node 1 (Port 8081):**
```bash
cargo run --bin sentinel-node -- -d ./node1 -p 8081
cargo run --bin sentinel-node -- -d ./node2 -p 8082
cargo run --bin sentinel-node -- -d ./node3 -p 8083

### 3. Usage Commands
Once the nodes are running, they will automatically discover and verify each other via mDNS. You can also interact with the node via the following CLI commands:

/dial <ip>:<port>: Manually initiate a connection to a specific peer.
/peers: List all currently verified cryptographic identities and their connection status.
/history: Retrieve and display the last 10 messages stored in the local Sled database.

##  🔐 Security Note
This project utilizes Self-Signed TLS Certificates strictly for transport-layer encryption, wrapped inside a custom Ed25519 Handshake for identity.

While standard browsers or OS tools might flag the TLS as "Insecure" due to the lack of a central Certificate Authority (CA), the SentinelNode engine performs its own verification by pinning the public_key exchanged during the handshake. This implements a Trust on First Use (TOFU) security model, providing cryptographic certainty similar to SSH without the need for centralized intermediaries.