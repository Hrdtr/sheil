mod commands;
mod keystore;
use commands::default::{read, write};
use keystore::{delete_credential, retrieve_credential, store_credential};

#[allow(clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read,
            write,
            store_credential,
            retrieve_credential,
            delete_credential
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
