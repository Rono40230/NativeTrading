//! Les 3 commandes JS ↔ natif du plugin relais.

use super::Relais;
use tauri::{AppHandle, Manager, Runtime};

#[tauri::command]
pub(crate) async fn demarrer<R: Runtime>(
    app: AppHandle<R>,
    serveur: String,
    token: String,
) -> Result<(), String> {
    match app.try_state::<Relais<R>>() {
        Some(p) => p
            .0
            .run_mobile_plugin::<serde_json::Value>(
                "demarrer",
                serde_json::json!({ "serveur": serveur, "token": token }),
            )
            .map(|_| ())
            .map_err(|e| e.to_string()),
        None => Err("Relais : Android uniquement".into()),
    }
}

#[tauri::command]
pub(crate) async fn arreter<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    match app.try_state::<Relais<R>>() {
        Some(p) => p
            .0
            .run_mobile_plugin::<serde_json::Value>("arreter", serde_json::json!({}))
            .map(|_| ())
            .map_err(|e| e.to_string()),
        None => Err("Relais : Android uniquement".into()),
    }
}

#[tauri::command]
pub(crate) async fn etat<R: Runtime>(app: AppHandle<R>) -> Result<serde_json::Value, String> {
    match app.try_state::<Relais<R>>() {
        Some(p) => p
            .0
            .run_mobile_plugin::<serde_json::Value>("etat", serde_json::json!({}))
            .map_err(|e| e.to_string()),
        None => Ok(serde_json::json!({ "actif": false, "android": false })),
    }
}
