use anyhow::{Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;
use zeroize::Zeroize;

#[derive(Debug)]
pub struct NodeIdentity {
    signing_key: SigningKey,
}

impl Drop for NodeIdentity {
    fn drop(&mut self) {
        let mut key_bytes = self.signing_key.to_bytes();
        key_bytes.zeroize();
    }
}

impl NodeIdentity {
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    pub fn load_or_generate<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let exists = path.exists() && fs::metadata(path)?.len() > 0;

        if exists {
            let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
            let array: [u8; 32] = bytes.try_into().map_err(|_| anyhow::anyhow!("Invalid key length"))?;
            let signing_key = SigningKey::from_bytes(&array);
            Ok(Self { signing_key })
        } else {
            let new_identity = Self::generate();
            new_identity.save(path)?;
            Ok(new_identity)
        }
    }

    pub fn node_id(&self) -> String {
        hex::encode(self.signing_key.verifying_key().to_bytes())
    }

    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    /// Static verification for use in async contexts where self is not available
    pub fn verify(message: &[u8], signature_bytes: &[u8], pubkey_bytes: &[u8]) -> bool {
        if let (Ok(sig), Ok(pubkey)) = (
            Signature::from_slice(signature_bytes),
            VerifyingKey::from_bytes(pubkey_bytes.try_into().unwrap_or(&[0u8; 32])),
        ) {
            return pubkey.verify(message, &sig).is_ok();
        }
        false
    }

    /// Helper for tests to verify against this identity
    pub fn verify_internal(&self, message: &[u8], signature_bytes: &[u8]) -> bool {
        Self::verify(message, signature_bytes, &self.public_key_bytes())
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, self.signing_key.to_bytes())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_identity_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let id1 = NodeIdentity::load_or_generate(path).unwrap();
        let sig = id1.sign(b"test");
        let id2 = NodeIdentity::load_or_generate(path).unwrap();
        assert!(id2.verify_internal(b"test", &sig));
    }

    #[test]
    fn test_generate_new() {
        let id = NodeIdentity::generate();
        let sig = id.sign(b"test");
        assert!(id.verify_internal(b"test", &sig));
    }
}

// - generate()          // New identity
// - load_or_generate()  // Load or create
// - node_id()           // Hex identifier  
// - public_key()        // Get public key
// - sign() / verify()   // Crypto operations
// - save()              // Persist to disk