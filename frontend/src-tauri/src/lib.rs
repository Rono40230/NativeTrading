/// Native Trading AI — Application Tauri
/// Fenêtre native Linux, aucun navigateur requis.
/// Le backend Actix-Web tourne séparément sur port 8080.

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Démarre `ollama serve` en arrière-plan si le serveur n'est pas déjà actif.
/// Utilise nohup + redirection stdin/stdout/stderr pour un détachement complet.
fn demarrer_ollama_si_absent() {
    // Test rapide : connexion HTTP directe plus fiable que `ollama list`
    let deja_actif = std::net::TcpStream::connect("127.0.0.1:11434").is_ok();

    if deja_actif {
        eprintln!("[Tauri] Ollama déjà actif — pas de démarrage");
        return;
    }

    // Spawn via shell avec nohup pour détachement complet du processus
    // OLLAMA_MODELS corrige le chemin après remontage du disque (/mnt/IA au lieu de /run/media/rono/IA)
    let resultat = Command::new("sh")
        .args(["-c", "nohup ollama serve > /tmp/ollama.log 2>&1 &"])
        .env("OLLAMA_MODELS", "/mnt/IA/ollama/models")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match resultat {
        Ok(_) => {
            eprintln!("[Tauri] Ollama démarré, attente initialisation...");
            // Laisser le temps à Ollama de s'initialiser avant l'UI
            thread::sleep(Duration::from_millis(1500));
        }
        Err(e) => eprintln!("[Tauri] Impossible de démarrer Ollama: {}", e),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    demarrer_ollama_si_absent();

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de l'application Tauri");
}
