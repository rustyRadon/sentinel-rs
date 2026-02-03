# Sentinel Protocol Specification (Phase 2)

## 1. Binary Framing (The Shield)
To ensure data integrity and prevent stream merging, all packets follow this binary layout:

| Field   | Size | Type     | Description |
| :------ | :--- | :------- | :---------- |
| VERSION | 1B   | `u8`     | Protocol version (currently `0x01`) |
| FLAGS   | 1B   | `u8`     | Reserved for compression/encryption flags |
| LENGTH  | 4B   | `u32`    | Payload size (Big-Endian) |
| PAYLOAD | Var  | `bytes`  | The serialized `SentinelMessage` |



## 2. Message Schema (Bincode)
The payload is a Bincode-serialized `SentinelMessage` struct.

### Header Fields:
- `sender_id`: `String` (Hex fingerprint of the sender's Ed25519 Public Key)
- `signature`: `Vec<u8>` (Ed25519 signature of the content + timestamp)
- `timestamp`: `u64` (Unix epoch in milliseconds)

### Content Enum (`MessageContent`):
- **Handshake**: `{ public_key: Vec<u8>, node_name: String }` - Used to establish identity.
- **Chat**: `String` - Standard encrypted text message.
- **Gossip**: `Vec<Uuid>` - A summary of known message IDs for sync.

## 3. Cryptographic Verification
Before a message is processed or saved to `Sled`, it must pass the following check:
$$Verify(Signature, SenderPublicKey, MessageContent + Timestamp)$$
If the verification fails, the connection is immediately terminated to prevent spoofing.

## 4. Connection State Machine
1.  **PENDING**: Socket connected, TLS established, waiting for `Handshake`.
2.  **VERIFYING**: `Handshake` received, cryptographic signature being checked.
3.  **ESTABLISHED**: Identity confirmed, peer added to routing table, chat allowed.
4.  **CLOSED**: Connection dropped; peer moved to "Offline" status in DB.