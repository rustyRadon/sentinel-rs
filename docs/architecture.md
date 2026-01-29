# Sentinel V1: Decentralized Architecture

Sentinel V1 moves away from the Client-Server model toward a **Leaderless Replication** system (inspired by DDIA Chapter 5.4). Every node is an equal peer.

## 1. The Peer-to-Peer Model
Unlike V0, where a client talks to a central server, V1 nodes form a **Partial Mesh Network**. 
* **Replica Equality**: Every node stores a full copy of the chat history.
* **Fault Tolerance**: If any node goes offline, the system continues.
* **Eventual Consistency**: Nodes sync missed messages once they reconnect.

## 2. Layered Node Stack
Each node consists of four distinct layers:

1. **Identity Layer (`sentinel-crypto`)**: 
   - Uses Ed25519 keys for node identification.
   - Provides digital signatures for message authenticity.
2. **Transport Layer (`sentinel-transport`)**: 
   - Secures the stream via TLS 1.3.
   - Handles TCP connection pooling and handshakes.
3. **Protocol Layer (`sentinel-protocol`)**:
   - Frames raw bytes into structured packets.
   - Handles CRC32 integrity checks.
4. **Storage Layer (Sled DB)**:
   - A Log-Structured storage engine.
   - Persists messages and peer information to disk.

## 3. Data Flow (The Write Path)
1. **Local Write**: User types a message; it is saved to the local `sled` database.
2. **Gossip/Broadcast**: The node iterates through all active peer connections.
3. **Async Replication**: The message is sent as a `Frame` over TLS to all peers.
4. **Peer Acknowledgment**: Remote peers receive, verify, and persist the message to their own `sled` instances.