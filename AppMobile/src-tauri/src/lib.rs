//! Coquille Tauri du relais mobile (MOB ③) — Android via Tauri 2.
//! Toute la logique vit dans le front Vue (../src) ; côté natif, seul le
//! plugin relais (service de premier plan + notifications) est enregistré.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_relais::init());
    if cfg!(debug_assertions) {
        builder = builder.plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        );
    }
    if let Err(e) = builder.run(tauri::generate_context!()) {
        log::error!("relais : erreur fatale tauri — {e}");
    }
}
