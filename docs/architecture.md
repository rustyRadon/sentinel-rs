# Architecture Overview: Sentinel-rs

Sentinel-rs is a high-performance, secure telemetry and file-transfer system implemented in Rust. It utilizes a layered architecture to decouple the communication protocol from the underlying transport security.

## 1. System Components

The project is organized as a Cargo workspace with four primary crates:

###  `sentinel-protocol`
* **Purpose**: Defines the data serialization format.
* **Key Logic**: Implements a `SentinelCodec` using `tokio-util`. It handles frame delimitation and integrity.
* **Data Unit**: The `Frame` struct, consisting of a version byte, flag byte (command ID), and a variable-length payload.

###  `sentinel-transport`
* **Purpose**: Manages the encrypted communication channel.
* **Key Logic**: Wraps `tokio-rustls` to provide a generic `TlsTransport<S>`. 
* **Security**: Handles X.509 certificate validation and the TLS 1.3 handshake. It provides the `SentinelAcceptor` used by the server to secure raw TCP streams.

###  `sentinel-client` (sntl)
* **Purpose**: Command-line interface for remote interaction.
* **Key Logic**: Manages the client-side TLS connector, performs identity verification against a Root CA, and executes asynchronous command/response cycles.

###  `sentinel-server`
* **Purpose**: The central processing node.
* **Key Logic**: A multi-threaded asynchronous task runner. It utilizes `Arc<Mutex<T>>` patterns to share system state (via the `sysinfo` crate) across concurrent TLS sessions.

---

## 2. Communication Lifecycle



The interaction between a client and server follows a strict four-stage lifecycle:

1.  **Transport Establishment**: A TCP connection is opened on port 8443.
2.  **Identity Verification**: A TLS 1.3 handshake is performed. The client verifies the server's certificate against a local Root CA. ALPN is used to negotiate the `sentinel-v1` protocol.
3.  **Framing**: The `SentinelCodec` is layered over the `TlsStream`. This transforms the stream of bytes into discrete `Frame` objects.
4.  **Command Execution**: 
    * The client sends a `Frame` with a specific `flag` (e.g., `0x01` for Status).
    * The server's command router dispatches the request to the appropriate handler.
    * The server responds with a new `Frame` containing the requested data.

---

## 3. Security Model

Sentinel-rs assumes a Zero-Trust network environment.

* **Encryption**: All data in transit is encrypted using authenticated encryption with associated data (AEAD) via Rustls.
* **Authentication**: The server must present a certificate signed by a CA trusted by the client.
* **Integrity**: The protocol framing ensures that truncated or malformed packets are rejected at the codec level before reaching the application logic.

---

## 4. Technical Stack

| Component | Library |
| :--- | :--- |
| Runtime | `tokio` (Multi-threaded) |
| TLS | `rustls` / `tokio-rustls` |
| Serialization | `tokio-util` (Codec/Framed) |
| Metrics | `sysinfo` |
| CLI | `clap` |
| Logging | `tracing` |

---

## 5. Directory Structure



```text
.
├── certs/                   # Generated X.509 certificates and keys
├── crates/
│   ├── sentinel-protocol/   # Frame and Codec definitions
│   ├── sentinel-transport/  # TLS wrapping and Acceptor logic
│   ├── client/              # Client binary and library
│   └── server/              # Server binary and metrics logic
├── scripts/                 # Automation scripts (gen-certs.sh)
└── target/                  # Compiled artifacts