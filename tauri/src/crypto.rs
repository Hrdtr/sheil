use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use rand::Rng;
use std::path::Path;

pub const NONCE_SIZE: usize = 12;
pub const MASTER_KEY_SIZE: usize = 32;

/// Encrypt `plaintext` with AES-256-GCM using the provided master key.
/// Returns `(ciphertext, nonce)` as raw byte vectors.
#[allow(deprecated)]
pub fn encrypt(
    master_key: &[u8; MASTER_KEY_SIZE],
    plaintext: &[u8],
) -> (Vec<u8>, [u8; NONCE_SIZE]) {
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .expect("AES-256-GCM encryption should not fail");
    (ciphertext, nonce_bytes)
}

/// Decrypt `ciphertext` with AES-256-GCM using the provided `nonce` and master key.
#[allow(deprecated)]
pub fn decrypt(
    master_key: &[u8; MASTER_KEY_SIZE],
    ciphertext: &[u8],
    nonce_bytes: &[u8; NONCE_SIZE],
) -> Result<Vec<u8>, String> {
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("decryption failed: {e}"))
}

/// Load the master key from `{app_data_dir}/.master_key`, or generate a new
/// random key and persist it if the file does not yet exist.
///
/// On Unix, the key file is created with `0o600` permissions (owner-only
/// read/write). On Windows the file inherits the directory ACL.
pub fn load_or_create_master_key(app_data_dir: &Path) -> Result<[u8; MASTER_KEY_SIZE], String> {
    let key_path = app_data_dir.join(".master_key");

    if key_path.exists() {
        let bytes =
            std::fs::read(&key_path).map_err(|e| format!("failed to read master key: {e}"))?;
        let key: [u8; MASTER_KEY_SIZE] = bytes
            .try_into()
            .map_err(|_| "master key has wrong length — expected 32 bytes".to_string())?;
        Ok(key)
    } else {
        let mut key = [0u8; MASTER_KEY_SIZE];
        rand::rng().fill_bytes(&mut key);
        std::fs::write(&key_path, key).map_err(|e| format!("failed to write master key: {e}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600)).ok();
        }

        log::info!("Generated new master key at {}", key_path.display());
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let mut key = [0u8; MASTER_KEY_SIZE];
        rand::rng().fill_bytes(&mut key);

        let plaintext = b"hunter2";
        let (ciphertext, nonce) = encrypt(&key, plaintext);

        let decrypted = decrypt(&key, &ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn decrypt_wrong_key_fails() {
        let mut key_a = [0u8; MASTER_KEY_SIZE];
        let mut key_b = [0u8; MASTER_KEY_SIZE];
        rand::rng().fill_bytes(&mut key_a);
        rand::rng().fill_bytes(&mut key_b);
        assert_ne!(key_a, key_b);

        let plaintext = b"secret";
        let (ciphertext, nonce) = encrypt(&key_a, plaintext);

        let result = decrypt(&key_b, &ciphertext, &nonce);
        assert!(result.is_err());
    }

    #[test]
    fn decrypt_wrong_nonce_fails() {
        let mut key = [0u8; MASTER_KEY_SIZE];
        rand::rng().fill_bytes(&mut key);

        let plaintext = b"secret";
        let (ciphertext, _nonce) = encrypt(&key, plaintext);

        let mut wrong_nonce = [0u8; NONCE_SIZE];
        rand::rng().fill_bytes(&mut wrong_nonce);

        let result = decrypt(&key, &ciphertext, &wrong_nonce);
        assert!(result.is_err());
    }

    #[test]
    fn master_key_persistence_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let key = load_or_create_master_key(tmp.path()).unwrap();
        assert_eq!(key.len(), MASTER_KEY_SIZE);

        let key2 = load_or_create_master_key(tmp.path()).unwrap();
        assert_eq!(key, key2);
    }

    #[test]
    #[cfg(unix)]
    fn master_key_file_has_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let _ = load_or_create_master_key(tmp.path()).unwrap();

        let key_path = tmp.path().join(".master_key");
        let meta = std::fs::metadata(&key_path).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }
}
