use ed25519_dalek::{SigningKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;

pub struct NodeIdentity {
    pub signing_key: SigningKey,
}

impl NodeIdentity {
    /// Generate a brand new identity
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Load identity from disk, or generate a new one if missing
    pub fn load_or_generate<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        if path.as_ref().exists() {
            let bytes = fs::read(path)?;
            let signing_key = SigningKey::from_bytes(bytes.as_slice().try_into()?);
            Ok(Self { signing_key })
        } else {
            let new_identity = Self::generate();
            fs::write(path, new_identity.signing_key.to_bytes())?;
            Ok(new_identity)
        }
    }

    pub fn node_id(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_bytes())
    }
}
