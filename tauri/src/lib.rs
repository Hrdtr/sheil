mod commands;
use commands::default::{read, write};

use async_trait::async_trait;
use tauri_plugin_background_service::{
    init_with_service, BackgroundService, ServiceContext, ServiceError,
};

struct PlaceholderService;

#[async_trait]
impl<R: tauri::Runtime> BackgroundService<R> for PlaceholderService {
    async fn init(&mut self, _ctx: &ServiceContext<R>) -> Result<(), ServiceError> {
        Ok(())
    }

    async fn run(&mut self, ctx: &ServiceContext<R>) -> Result<(), ServiceError> {
        ctx.shutdown.cancelled().await;
        Ok(())
    }
}

#[allow(clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_keystore::init())
        .plugin(init_with_service(|| PlaceholderService))
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
        .invoke_handler(tauri::generate_handler![read, write])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
