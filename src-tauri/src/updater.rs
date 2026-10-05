use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_updater::UpdaterExt;
use tauri_specta::Event;

use crate::error::{AppError, AppResult};

const PERIOD: Duration = Duration::from_secs(24 * 60 * 60);
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize, specta::Type, tauri_specta::Event,
)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAvailable {
    pub version: String,
    pub notes: Option<String>,
}

fn failed(e: impl std::fmt::Display) -> AppError {
    AppError::new("UPDATE_FAILED", e.to_string())
}

async fn check<R: Runtime>(app: &AppHandle<R>) -> AppResult<Option<UpdateAvailable>> {
    let update = app
        .updater()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)?;
    Ok(update.map(|u| UpdateAvailable {
        version: u.version,
        notes: u.body,
    }))
}

#[tauri::command]
#[specta::specta]
pub async fn update_check(app: AppHandle) -> AppResult<Option<UpdateAvailable>> {
    let found = check(&app).await?;
    if let Some(update) = &found {
        let _ = update.clone().emit(&app);
    }
    Ok(found)
}

#[tauri::command]
#[specta::specta]
pub async fn update_install(app: AppHandle) -> AppResult<()> {
    let Some(update) = app
        .updater()
        .map_err(failed)?
        .check()
        .await
        .map_err(failed)?
    else {
        return Err(AppError::new("UPDATE_NONE", "no update is available"));
    };
    crate::tray::stop_engine(&app);
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(failed)?;
    app.restart();
}

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        tokio_sleep(FIRST_CHECK_DELAY).await;
        loop {
            match check(&app).await {
                Ok(Some(update)) => {
                    log::info!("update available: {}", update.version);
                    let _ = update.emit(&app);
                }
                Ok(None) => {}
                Err(e) => log::info!("update check skipped: {}", e.message),
            }
            tokio_sleep(PERIOD).await;
        }
    });
}

async fn tokio_sleep(duration: Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(duration)).await;
}
