use serde::Deserialize;
use tauri::command;

#[derive(Debug, thiserror::Error)]
enum KeystoreError {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),
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

#[command]
pub fn store_credential(credential: Credential) -> Result<(), String> {
    let entry = keyring::Entry::new(&credential.service, &credential.username)
        .map_err(KeystoreError::from)?;
    entry
        .set_password(&credential.password)
        .map_err(KeystoreError::from)?;
    Ok(())
}

#[command]
pub fn retrieve_credential(service: String, username: String) -> Result<String, String> {
    let entry = keyring::Entry::new(&service, &username).map_err(KeystoreError::from)?;
    let password = entry.get_password().map_err(KeystoreError::from)?;
    Ok(password)
}

#[command]
pub fn delete_credential(service: String, username: String) -> Result<(), String> {
    let entry = keyring::Entry::new(&service, &username).map_err(KeystoreError::from)?;
    entry.delete_credential().map_err(KeystoreError::from)?;
    Ok(())
}
