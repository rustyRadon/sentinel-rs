# Sentinel Protocol Specification

## 1. Framing
Sentinel uses a length-prefixed binary framing mechanism to prevent "split-packet" issues in TCP.

| Field | Size | Type | Description |
| :--- | :--- | :--- | :--- |
| MAGIC | 4B | `[u8; 4]` | Always `0x53 0x4E 0x54 0x4C` ("SNTL") |
| VERSION| 1B | `u8` | Current Version: `0x01` |
| LENGTH | 4B | `u32` | Size of the following payload (Big-Endian) |
| PAYLOAD| Var | `bytes` | Bincode-serialized `SentinelMessage` |
| CRC32  | 4B | `u32` | Integrity check of the payload |

## 2. Serialization (Bincode)
The payload follows this logical structure:
- `id`: UUID (16 bytes)
- `sender`: String (Public key fingerprint)
- `timestamp`: u64 (Unix nanos)
- `content`: Enum (Chat, Ping, Handshake)

## 3. Security Handshake
1. **TCP**: Handshake on port 8443.
2. **ALPN**: Negotiation of `sentinel-v1`.
3. **mTLS**: Optional mutual authentication via X.509.