# Sentinel Protocol Specification (V1)

## 1. Framing (Binary)
All communication uses a fixed-header binary frame to ensure memory safety and fast parsing.

| Offset | Field | Size | Description |
| :--- | :--- | :--- | :--- |
| 0 | MAGIC | 4B | Always `SNTL` |
| 4 | VERSION | 1B | Protocol version (Currently 1) |
| 5 | FLAGS | 1B | Bitmask for message priority/type |
| 6 | LENGTH | 4B | Payload size (Big-Endian) |
| 10 | PAYLOAD | var | The serialized `SentinelMessage` |
| 10+N | CRC32 | 4B | Checksum of Version + Flags + Payload |

## 2. Message Schema (Serialization)
The payload is serialized using **Bincode** for minimal overhead.

### Message Structure:
* `id`: UUID (Used for deduplication/idempotency).
* `sender`: String (The Public Key Hash/Node ID).
* `timestamp`: u64 (Unix nanos for causal ordering).
* `content`: Enum (Chat, Handshake, Ping, Pong).

## 3. Handshake Sequence
1. **TCP Connection**: Peer A connects to Peer B.
2. **TLS Upgrade**: Negotiate `sentinel-v1` via ALPN.
3. **Identity Exchange**: Both peers send a `Handshake` frame containing their Public Key.
4. **Verification**: Peers verify that the TLS certificate matches the signed Handshake.