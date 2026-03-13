/// Native Trading AI — Application Tauri
/// Fenêtre native Linux, aucun navigateur requis.
/// Le backend Actix-Web est démarré automatiquement au lancement.

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Démarre le backend API Rust sur le port 8080 si non actif.
fn demarrer_backend_si_absent() {
    if std::net::TcpStream::connect("127.0.0.1:8080").is_ok() {
        eprintln!("[Tauri] Backend API déjà actif sur :8080");
        return;
    }

    // Chercher le binaire release relatif à l'exécutable courant, puis fallback dev
    let binaire = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .map(|p| p.join("../backend/target/release/api"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            std::path::PathBuf::from("/mnt/IA/native-trading-ai/backend/target/release/api")
        });

    let db_path = "/mnt/IA/native-trading-ai/data/trading.db";

    match Command::new(&binaire)
        .env("DATABASE_PATH", db_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => {
            eprintln!("[Tauri] Backend API démarré ({})", binaire.display());
            // Attendre qu'il soit prêt (max 5s)
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(500));
                if std::net::TcpStream::connect("127.0.0.1:8080").is_ok() {
                    eprintln!("[Tauri] Backend prêt ✅");
                    return;
                }
            }
            eprintln!("[Tauri] Backend démarré mais pas encore prêt après 5s");
        }
        Err(e) => eprintln!("[Tauri] Échec démarrage backend ({}): {}", binaire.display(), e),
    }
}

/// Démarre `ollama serve` en arrière-plan si le serveur n'est pas déjà actif.
fn demarrer_ollama_si_absent() {
    if std::net::TcpStream::connect("127.0.0.1:11434").is_ok() {
        eprintln!("[Tauri] Ollama déjà actif — pas de démarrage");
        return;
    }

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
            thread::sleep(Duration::from_millis(1500));
        }
        Err(e) => eprintln!("[Tauri] Impossible de démarrer Ollama: {}", e),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    demarrer_backend_si_absent();
    demarrer_ollama_si_absent();

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de l'application Tauri");
}
