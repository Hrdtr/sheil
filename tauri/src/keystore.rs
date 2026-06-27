use serde::Deserialize;
use tauri::command;

#[derive(Debug, thiserror::Error)]
enum KeystoreError {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("validation error: {0}")]
    Validation(String),
}

impl From<KeystoreError> for String {
    fn from(e: KeystoreError) -> Self {
        e.to_string()
    }
}

#[derive(Deserialize)]
pub struct Credential {
    service: String,
    username: String,
    password: String,
}

impl Credential {
    fn validate(&self) -> Result<(), KeystoreError> {
        if self.service.trim().is_empty() {
            return Err(KeystoreError::Validation(
                "service must not be empty".into(),
            ));
        }
        if self.username.trim().is_empty() {
            return Err(KeystoreError::Validation(
                "username must not be empty".into(),
            ));
        }
        if self.password.is_empty() {
            return Err(KeystoreError::Validation(
                "password must not be empty".into(),
            ));
        }
        Ok(())
    }
}

fn validate_service_username(service: &str, username: &str) -> Result<(), KeystoreError> {
    if service.trim().is_empty() {
        return Err(KeystoreError::Validation(
            "service must not be empty".into(),
        ));
    }
    if username.trim().is_empty() {
        return Err(KeystoreError::Validation(
            "username must not be empty".into(),
        ));
    }
    Ok(())
}

#[command]
pub fn store_credential(credential: Credential) -> Result<(), String> {
    credential.validate()?;
    let entry = keyring::Entry::new(&credential.service, &credential.username)
        .map_err(KeystoreError::from)?;
    entry
        .set_password(&credential.password)
        .map_err(KeystoreError::from)?;
    Ok(())
}

#[command]
pub fn retrieve_credential(service: String, username: String) -> Result<String, String> {
    validate_service_username(&service, &username)?;
    let entry = keyring::Entry::new(&service, &username).map_err(KeystoreError::from)?;
    let password = entry.get_password().map_err(KeystoreError::from)?;
    Ok(password)
}

#[command]
pub fn delete_credential(service: String, username: String) -> Result<(), String> {
    validate_service_username(&service, &username)?;
    let entry = keyring::Entry::new(&service, &username).map_err(KeystoreError::from)?;
    entry.delete_credential().map_err(KeystoreError::from)?;
    Ok(())
}

#[command]
pub fn credential_exists(service: String, username: String) -> Result<bool, String> {
    validate_service_username(&service, &username)?;
    let entry = keyring::Entry::new(&service, &username).map_err(KeystoreError::from)?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(KeystoreError::Keyring(e).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> String {
        "dev.hrdtr.sheil.test".into()
    }

    fn test_username() -> String {
        "test-user@localhost".into()
    }

    fn test_password() -> String {
        "test-secret-123".into()
    }

    fn cleanup(service: &str, username: &str) {
        let _ = keyring::Entry::new(service, username).and_then(|entry| entry.delete_credential());
    }

    fn assume_keychain_available() -> bool {
        let service = test_service();
        let username: String = "__sheil_keychain_probe__".into();
        let probe = Credential {
            service: service.clone(),
            username: username.clone(),
            password: "probe".into(),
        };
        match store_credential(probe) {
            Ok(()) => {
                let readable = retrieve_credential(service, username.clone()).is_ok();
                cleanup(&test_service(), &username);
                readable
            }
            Err(_) => false,
        }
    }

    fn assume_keychain_available_and_cleanup(service: &str, username: &str) -> bool {
        let probe = Credential {
            service: service.into(),
            username: username.into(),
            password: test_password(),
        };
        match store_credential(probe) {
            Ok(()) => {
                let readable = retrieve_credential(service.into(), username.into()).is_ok();
                cleanup(service, username);
                readable
            }
            Err(_) => false,
        }
    }

    #[test]
    fn store_and_retrieve_roundtrip() {
        let service = test_service();
        let username = test_username();
        let password = test_password();
        cleanup(&service, &username);

        if !assume_keychain_available() {
            eprintln!("skipping: keychain access unavailable for test binary");
            return;
        }

        let cred = Credential {
            service: service.clone(),
            username: username.clone(),
            password: password.clone(),
        };
        store_credential(cred).unwrap();

        let retrieved = retrieve_credential(service.clone(), username.clone()).unwrap();
        assert_eq!(retrieved, password);

        cleanup(&service, &username);
    }

    #[test]
    fn store_and_delete() {
        let service = test_service();
        let username = test_username();
        cleanup(&service, &username);

        if !assume_keychain_available() {
            eprintln!("skipping: keychain access unavailable for test binary");
            return;
        }

        let cred = Credential {
            service: service.clone(),
            username: username.clone(),
            password: test_password(),
        };
        store_credential(cred).unwrap();

        delete_credential(service.clone(), username.clone()).unwrap();

        let result = retrieve_credential(service.clone(), username.clone());
        assert!(result.is_err());

        cleanup(&service, &username);
    }

    #[test]
    fn credential_exists_returns_true_after_store() {
        let service = test_service();
        let username = test_username();
        cleanup(&service, &username);

        if !assume_keychain_available() {
            eprintln!("skipping: keychain access unavailable for test binary");
            return;
        }

        assert!(!credential_exists(service.clone(), username.clone()).unwrap());

        let cred = Credential {
            service: service.clone(),
            username: username.clone(),
            password: test_password(),
        };
        store_credential(cred).unwrap();

        assert!(credential_exists(service.clone(), username.clone()).unwrap());

        cleanup(&service, &username);
    }

    #[test]
    fn credential_exists_returns_false_after_delete() {
        let service = test_service();
        let username = test_username();
        cleanup(&service, &username);

        if !assume_keychain_available() {
            eprintln!("skipping: keychain access unavailable for test binary");
            return;
        }

        let cred = Credential {
            service: service.clone(),
            username: username.clone(),
            password: test_password(),
        };
        store_credential(cred).unwrap();
        delete_credential(service.clone(), username.clone()).unwrap();

        assert!(!credential_exists(service.clone(), username.clone()).unwrap());

        cleanup(&service, &username);
    }

    #[test]
    fn retrieve_nonexistent_credential_fails() {
        let service = "dev.hrdtr.sheil.test.nonexistent";
        let username = "ghost@nowhere";
        cleanup(service, username);

        if !assume_keychain_available_and_cleanup(service, username) {
            eprintln!("skipping: keychain access unavailable for test binary");
            return;
        }

        let result = retrieve_credential(service.to_string(), username.to_string());
        assert!(result.is_err());

        cleanup(service, username);
    }

    #[test]
    fn delete_nonexistent_credential_fails() {
        let service = "dev.hrdtr.sheil.test.nonexistent";
        let username = "ghost@nowhere";
        cleanup(service, username);

        if !assume_keychain_available_and_cleanup(service, username) {
            eprintln!("skipping: keychain access unavailable for test binary");
            return;
        }

        let result = delete_credential(service.to_string(), username.to_string());
        assert!(result.is_err());

        cleanup(service, username);
    }

    #[test]
    fn overwrite_existing_credential() {
        let service = test_service();
        let username = test_username();
        cleanup(&service, &username);

        if !assume_keychain_available() {
            eprintln!("skipping: keychain access unavailable for test binary");
            return;
        }

        let first = Credential {
            service: service.clone(),
            username: username.clone(),
            password: "first-password".into(),
        };
        store_credential(first).unwrap();

        let second = Credential {
            service: service.clone(),
            username: username.clone(),
            password: "second-password".into(),
        };
        store_credential(second).unwrap();

        let retrieved = retrieve_credential(service.clone(), username.clone()).unwrap();
        assert_eq!(retrieved, "second-password");

        cleanup(&service, &username);
    }

    // Validation tests — always run (no keychain access needed)

    #[test]
    fn store_empty_service_fails() {
        let cred = Credential {
            service: String::new(),
            username: test_username(),
            password: test_password(),
        };
        let result = store_credential(cred);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("service must not be empty"));
    }

    #[test]
    fn store_whitespace_only_service_fails() {
        let cred = Credential {
            service: "   ".into(),
            username: test_username(),
            password: test_password(),
        };
        let result = store_credential(cred);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("service must not be empty"));
    }

    #[test]
    fn store_empty_username_fails() {
        let cred = Credential {
            service: test_service(),
            username: String::new(),
            password: test_password(),
        };
        let result = store_credential(cred);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("username must not be empty"));
    }

    #[test]
    fn store_empty_password_fails() {
        let cred = Credential {
            service: test_service(),
            username: test_username(),
            password: String::new(),
        };
        let result = store_credential(cred);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("password must not be empty"));
    }

    #[test]
    fn retrieve_empty_service_fails() {
        let result = retrieve_credential(String::new(), test_username());
        assert!(result.is_err());
    }

    #[test]
    fn retrieve_empty_username_fails() {
        let result = retrieve_credential(test_service(), String::new());
        assert!(result.is_err());
    }

    #[test]
    fn delete_empty_service_fails() {
        let result = delete_credential(String::new(), test_username());
        assert!(result.is_err());
    }

    #[test]
    fn credential_exists_empty_service_fails() {
        let result = credential_exists(String::new(), test_username());
        assert!(result.is_err());
    }
}
