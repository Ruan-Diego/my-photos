//! F37: updates, the only network connection the app ever makes (F36), and only when
//! the person asks ("Procurar agora") or turned on "Procurar atualizações ao abrir".
//! Release builds carry the updater's public key and address (`tauri.release.conf.json`);
//! a development build has neither and never looks.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Config, Runtime, State};

/// The update found by the last check, kept until "Instalar e reiniciar".
#[derive(Default)]
pub struct UpdateState(pub Mutex<Option<tauri_plugin_updater::Update>>);

/// The release config has the updater's key and address.
pub fn configured(config: &Config) -> bool {
    config
        .plugins
        .0
        .get("updater")
        .and_then(|updater| updater.get("pubkey"))
        .and_then(|key| key.as_str())
        .is_some_and(|key| !key.trim().is_empty())
}

/// Registers the updater plugin in release builds only.
pub fn register<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    if configured(app.config()) {
        app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Available {
    pub version: String,
    pub notes: Option<String>,
}

/// "Procurar atualizações": `None` when this is the newest version.
#[tauri::command]
pub async fn app_update_check(app: AppHandle, state: State<'_, UpdateState>) -> Result<Option<Available>, String> {
    use tauri_plugin_updater::UpdaterExt;
    if !configured(app.config()) {
        return Err("esta versão do app não procura atualizações".to_string());
    }
    let update = app.updater().map_err(|error| error.to_string())?.check().await.map_err(|error| error.to_string())?;
    let available = update.as_ref().map(|update| Available { version: update.version.clone(), notes: update.body.clone() });
    *state.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = update;
    Ok(available)
}

/// "Instalar e reiniciar": downloads, checks the signature against the public key,
/// runs the installer and restarts. The library database closes cleanly first.
#[tauri::command]
pub async fn app_update_install(app: AppHandle, state: State<'_, UpdateState>) -> Result<(), String> {
    let update = state
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take()
        .ok_or("procure a atualização antes de instalar")?;
    update.download_and_install(|_, _| {}, || {}).await.map_err(|error| error.to_string())?;
    if let Some(db) = tauri::Manager::try_state::<super::health::DbState>(&app) {
        db.shutdown();
    }
    app.restart();
}
