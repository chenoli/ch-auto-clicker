//! Profile library commands (TASKS T8.1–T8.2, T8.7): every change goes through
//! `ch_core::profiles`, is persisted, re-applies shortcuts when needed and notifies every window.

use ch_core::chprofile::{self, ImportNote};
use ch_core::model::Profile;
use ch_core::profiles::{self as library, ProfileError};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;

use crate::commands::next_version;
use crate::error::{AppError, AppResult};
use crate::state::{AppData, AppState, emit_profiles, emit_settings};
use crate::thumbs;

impl From<ProfileError> for AppError {
    fn from(e: ProfileError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<chprofile::ChprofileError> for AppError {
    fn from(e: chprofile::ChprofileError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

/// An imported profile and what had to change to fit this library.
#[derive(Debug, Clone, PartialEq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub profile: Profile,
    pub notes: Vec<ImportNote>,
}

fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Persists what changed and tells every window; shortcuts are re-registered when profiles that
/// own one appear or disappear.
fn publish<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppState,
    data: &AppData,
    changed: &[&Profile],
    removed: Option<&str>,
    hotkeys: bool,
) {
    for p in changed {
        state.persister.save_profile(p);
    }
    if let Some(id) = removed {
        state.persister.delete_profile(id);
    }
    state.persister.save_settings(&data.settings);
    emit_settings(app, &data.settings);
    emit_profiles(app, data);
    if hotkeys {
        state.hotkeys().apply(app, data);
    }
}

/// "+ Novo": a default profile at the end of the list.
#[tauri::command]
#[specta::specta]
pub fn profile_create(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> AppResult<Profile> {
    let id = new_id();
    let (profile, data) = state.with_data(|data| {
        let p = library::create(
            &mut data.settings,
            &mut data.profiles,
            &id,
            &name,
            &next_version(),
        )?;
        Ok::<_, AppError>((p, data.clone()))
    })?;
    publish(&app, &state, &data, &[&profile], None, false);
    Ok(profile)
}

#[tauri::command]
#[specta::specta]
pub fn profile_duplicate(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Profile> {
    let new = new_id();
    let (profile, data) = state.with_data(|data| {
        let p = library::duplicate(
            &mut data.settings,
            &mut data.profiles,
            &id,
            &new,
            &next_version(),
        )?;
        Ok::<_, AppError>((p, data.clone()))
    })?;
    // The scan thumbnail belongs to the area, which the copy keeps.
    if let (Some(from), Some(to)) = (thumbs::path_for(&id), thumbs::path_for(&new))
        && from.exists()
    {
        let _ = std::fs::copy(from, to);
    }
    publish(&app, &state, &data, &[&profile], None, false);
    Ok(profile)
}

#[tauri::command]
#[specta::specta]
pub fn profile_rename(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> AppResult<Profile> {
    let (profile, data) = state.with_data(|data| {
        let p = library::rename(&mut data.profiles, &id, &name, &next_version())?;
        Ok::<_, AppError>((p, data.clone()))
    })?;
    publish(&app, &state, &data, &[&profile], None, false);
    Ok(profile)
}

/// Deletes a profile (and its thumbnail). Deleting the one that is running stops the run first.
#[tauri::command]
#[specta::specta]
pub fn profile_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let running_it = state.data().settings.active_profile_id.as_deref() == Some(id.as_str())
        && state.engine.shared().state().is_active();
    if running_it && state.data().profiles.len() > 1 {
        state.engine.stop();
        let _ = state
            .engine
            .wait_idle(std::time::Duration::from_millis(500));
    }
    let (removed, data) = state.with_data(|data| {
        let removed = library::delete(&mut data.settings, &mut data.profiles, &id)?;
        Ok::<_, AppError>((removed, data.clone()))
    })?;
    if let Some(path) = thumbs::path_for(&id) {
        let _ = std::fs::remove_file(path);
    }
    publish(
        &app,
        &state,
        &data,
        &[],
        Some(&id),
        removed.hotkey.is_some(),
    );
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn profile_reorder(
    app: AppHandle,
    state: State<'_, AppState>,
    order: Vec<String>,
) -> AppResult<()> {
    let data = state.with_data(|data| {
        library::reorder(&mut data.settings, &data.profiles, order)?;
        Ok::<_, AppError>(data.clone())
    })?;
    publish(&app, &state, &data, &[], None, false);
    Ok(())
}

/// Makes a profile active. Switching while a run is going stops it (decision D12).
#[tauri::command]
#[specta::specta]
pub fn profile_activate(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let current = state.data().settings.active_profile_id;
    if current.as_deref() != Some(id.as_str()) && state.engine.shared().state().is_active() {
        state.engine.stop();
    }
    let data = state.with_data(|data| {
        library::activate(&mut data.settings, &data.profiles, &id)?;
        Ok::<_, AppError>(data.clone())
    })?;
    publish(&app, &state, &data, &[], None, false);
    Ok(())
}

/// Asks where to save and writes the `.chprofile`. Resolves with the path, or null if cancelled.
#[tauri::command]
#[specta::specta]
pub async fn profile_export(app: AppHandle, id: String) -> AppResult<Option<String>> {
    let state = app.state::<AppState>();
    let profile = state
        .data()
        .profile(&id)
        .cloned()
        .ok_or_else(|| AppError::new("PROFILE_NOT_FOUND", format!("no profile {id}")))?;
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("CH. Clicker", &[chprofile::EXTENSION])
        .set_file_name(chprofile::file_name(&profile))
        .blocking_save_file()
        .and_then(|p| p.into_path().ok())
    else {
        return Ok(None);
    };
    let exported_by = format!("{} {}", ch_core::PRODUCT_NAME, app.package_info().version);
    std::fs::write(&path, chprofile::export(&profile, &exported_by))
        .map_err(|e| AppError::new("EXPORT_FAILED", e.to_string()))?;
    Ok(Some(path.display().to_string()))
}

/// Asks for a `.chprofile` and imports it. Resolves with null if cancelled.
#[tauri::command]
#[specta::specta]
pub async fn profile_import(app: AppHandle) -> AppResult<Option<ImportResult>> {
    let Some(path) = app
        .dialog()
        .file()
        .add_filter("CH. Clicker", &[chprofile::EXTENSION])
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
    else {
        return Ok(None);
    };
    // Check the size before reading: the dialog lets any file through.
    let size = std::fs::metadata(&path)
        .map(|m| m.len())
        .unwrap_or(u64::MAX);
    if size > chprofile::MAX_BYTES as u64 {
        return Err(chprofile::ChprofileError::TooLarge.into());
    }
    let text =
        std::fs::read(&path).map_err(|e| AppError::new("IMPORT_UNREADABLE", e.to_string()))?;
    let text = String::from_utf8(text)
        .map_err(|_| AppError::new("IMPORT_NOT_JSON", "the file is not UTF-8 text"))?;
    import_text(&app, &text).map(Some)
}

/// Imports the text of a dropped file (Tela 05.B).
#[tauri::command]
#[specta::specta]
pub fn profile_import_text(app: AppHandle, text: String) -> AppResult<ImportResult> {
    import_text(&app, &text)
}

fn import_text<R: Runtime>(app: &AppHandle<R>, text: &str) -> AppResult<ImportResult> {
    let state = app.state::<AppState>();
    let id = new_id();
    let (imported, data) = state.with_data(|data| {
        let imported = chprofile::import(
            text,
            &id,
            &next_version(),
            &data.settings.hotkeys,
            &data.profiles,
        )?;
        data.profiles.push(imported.profile.clone());
        data.settings.profile_order.push(id.clone());
        Ok::<_, AppError>((imported, data.clone()))
    })?;
    let has_hotkey = imported.profile.hotkey.is_some();
    publish(app, &state, &data, &[&imported.profile], None, has_hotkey);
    Ok(ImportResult {
        profile: imported.profile,
        notes: imported.notes,
    })
}
