mod commands;
mod crypto;
mod db;
mod secrets;

use commands::ai::{
    ai_delete_model, ai_download_model, ai_generate, ai_is_loaded, ai_list_models, ai_load_model,
    ai_unload_model, AiState,
};
use commands::hosts::{
    host_create, host_delete, host_export, host_import, host_list, host_resolve,
    host_resolve_password, host_update, SharedPool,
};
use commands::port_forward::{
    port_forward_list, port_forward_start_dynamic, port_forward_start_local,
    port_forward_start_remote, port_forward_stop,
};
use commands::sftp::{
    sftp_canonicalize, sftp_connect, sftp_create_dir, sftp_disconnect, sftp_download, sftp_exists,
    sftp_list_dir, sftp_read_file, sftp_remove_dir, sftp_remove_file, sftp_rename, sftp_stat,
    sftp_upload, sftp_write_file, SftpState,
};
use commands::ssh::{
    known_host_clear_all, ssh_close_channel, ssh_connect, ssh_delete_key, ssh_disconnect,
    ssh_import_key, ssh_list_keys, ssh_list_sessions, ssh_open_channel, ssh_resize, ssh_write,
    SshState,
};
use crypto::load_or_create_master_key;
use tauri::Manager;

/// Wrapper so Tauri can manage the master key as shared state.
pub struct MasterKey(pub [u8; crypto::MASTER_KEY_SIZE]);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::missing_panics_doc)]
pub fn run() {
    std::env::set_var("HF_HUB_DISABLE_IMPLICIT_TOKEN", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");

            let pool = tauri::async_runtime::block_on(db::init(&app_data_dir))?;
            let master_key = load_or_create_master_key(&app_data_dir)
                .expect("failed to load or create master encryption key");

            app.manage(SshState::new(pool.clone()));
            app.manage(SftpState::new());
            app.manage(pool as SharedPool);
            app.manage(MasterKey(master_key));

            let (fwd_state, _tx) =
                commands::port_forward::ForwardingState::new(app.handle().clone());
            app.manage(fwd_state);

            let models_dir = app_data_dir.join("hf-models");
            let ai_state = AiState::new(models_dir).expect("failed to initialize AI state");
            app.manage(ai_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            host_create,
            host_list,
            host_resolve,
            host_update,
            host_delete,
            host_resolve_password,
            host_export,
            host_import,
            ssh_connect,
            ssh_disconnect,
            ssh_import_key,
            ssh_list_keys,
            ssh_delete_key,
            ssh_list_sessions,
            ssh_open_channel,
            ssh_write,
            ssh_resize,
            ssh_close_channel,
            known_host_clear_all,
            port_forward_start_local,
            port_forward_start_remote,
            port_forward_start_dynamic,
            port_forward_stop,
            port_forward_list,
            sftp_connect,
            sftp_disconnect,
            sftp_list_dir,
            sftp_stat,
            sftp_exists,
            sftp_canonicalize,
            sftp_create_dir,
            sftp_remove_file,
            sftp_remove_dir,
            sftp_rename,
            sftp_read_file,
            sftp_write_file,
            sftp_download,
            sftp_upload,
            ai_download_model,
            ai_delete_model,
            ai_list_models,
            ai_load_model,
            ai_unload_model,
            ai_is_loaded,
            ai_generate,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(ai_state) = window.app_handle().try_state::<AiState>() {
                    ai_state.unload();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
