//! Plugin relais (MOB ③) : service Android au premier plan qui interroge
//! /api/mobile/* toutes les 10 min et poste les notifications système hors
//! muette 21h→8h Paris. Toute la logique vit en Kotlin (RelaisService) ;
//! ce module n'est que le pont JS ↔ natif (3 commandes).
//! Sur desktop (dev navigateur/vite), les commandes répondent une erreur
//! propre « Android uniquement ».

use tauri::{
    plugin::{Builder, PluginHandle, TauriPlugin},
    AppHandle, Manager, Runtime,
};

#[cfg(target_os = "android")]
const JAVA_PACKAGE: &str = "fr.relais.trading.relais";

mod commands;
pub use commands::*;

/// Handle mobile du plugin — chaque appel traverse run_mobile_plugin
/// vers la classe Kotlin RelaisPlugin.
pub struct Relais<R: Runtime>(PluginHandle<R>);

/// Initialisation — enregistré par l'app via `.plugin(tauri_plugin_relais::init())`.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("relais")
        .invoke_handler(tauri::generate_handler![demarrer, arreter, etat])
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle = api.register_android_plugin(JAVA_PACKAGE, "RelaisPlugin")?;
                app.manage(Relais(handle));
            }
            #[cfg(not(target_os = "android"))]
            {
                let _ = (app, api);
            }
            Ok(())
        })
        .build()
}
