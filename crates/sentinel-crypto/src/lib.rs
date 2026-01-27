use anyhow::{Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey, SECRET_KEY_LENGTH};
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
        
        let exists_and_not_empty = path.exists() && fs::metadata(path)?.len() > 0;

        if exists_and_not_empty {
            let bytes = fs::read(path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            
            if bytes.len() != SECRET_KEY_LENGTH {
                anyhow::bail!("Invalid key length: expected 32, got {}", bytes.len());
            }
            
            let array: [u8; 32] = bytes.try_into().expect("Length checked");
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

    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.signing_key.verifying_key().verify(message, signature).is_ok()
    }

    pub fn sign_detached(&self, message: &[u8]) -> [u8; 64] {
        self.sign(message).to_bytes()
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, self.signing_key.to_bytes())
            .with_context(|| format!("Failed to write {}", path.display()))?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(path, perms)?;
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

        let id1 = NodeIdentity::load_or_generate(path).expect("Should generate new if empty");
        let id1_str = id1.node_id();

        let id2 = NodeIdentity::load_or_generate(path).expect("Should load existing");
        let id2_str = id2.node_id();

        assert_eq!(id1_str, id2_str, "IDs must persist across loads");

        let message = b"Hello Sentinel!";
        let signature = id1.sign(message);
        assert!(id2.verify(message, &signature));
    }

    #[test]
    fn test_invalid_key_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        fs::write(path, b"wrong_length_data").unwrap();

        let result = NodeIdentity::load_or_generate(path);
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_file_permissions() {
        use std::os::unix::fs::PermissionsExt;
        
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let identity = NodeIdentity::load_or_generate(path).unwrap();
        identity.save(path).unwrap();

        let metadata = fs::metadata(path).unwrap();
        let mode = metadata.permissions().mode();

        assert_eq!(mode & 0o777, 0o600, "Permissions should be 0600");
    }

    #[test]
    fn test_generate_new() {
        let id = NodeIdentity::generate();
        let node_id = id.node_id();
        assert_eq!(node_id.len(), 64);
        
        let message = b"test message";
        let signature = id.sign(message);
        assert!(id.verify(message, &signature));
    }
}

// - generate()          // New identity
// - load_or_generate()  // Load or create
// - node_id()           // Hex identifier  
// - public_key()        // Get public key
// - sign() / verify()   // Crypto operations
// - save()              // Persist to disk