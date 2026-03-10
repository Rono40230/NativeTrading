/// Native Trading AI — Application Tauri
/// Fenêtre native Linux, aucun navigateur requis.
/// Le backend Actix-Web tourne séparément sur port 8080.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de l'application Tauri");
}
