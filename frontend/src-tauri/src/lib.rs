/// Native Trading AI — Application Tauri
/// Fenêtre native Linux, aucun navigateur requis.
/// Le backend Actix-Web est démarré automatiquement au lancement.

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

// ─── Commandes Tauri ──────────────────────────────────────────────────────────

/// Affiche une notification OS native via `notify-send` (Linux).
///
/// Appelée depuis le frontend : `invoke('notifier', { titre, corps, urgence })`
/// `urgence` : "low" | "normal" | "critical"
#[tauri::command]
fn notifier(titre: &str, corps: &str, urgence: Option<&str>) {
    let niveau = urgence.unwrap_or("normal");
    let _ = Command::new("notify-send")
        .args([
            "--urgency", niveau,
            "--app-name", "Native Trading AI",
            "--icon", "dialog-information",
            titre,
            corps,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// Joue le son d'alerte `sounds/signal.ogg` via `paplay` (PulseAudio).
///
/// Appelée depuis le frontend : `invoke('jouer_son_signal')`
#[tauri::command]
fn jouer_son_signal() {
    // Chercher le fichier son dans plusieurs emplacements (dev + prod)
    let candidats = [
        // Dev : CWD = frontend/, lancé depuis src-tauri/
        std::path::PathBuf::from("src-tauri/sounds/signal.ogg"),
        // Dev alternatif : CWD = frontend/src-tauri/
        std::path::PathBuf::from("sounds/signal.ogg"),
        // Chemin absolu workspace (dev Linux)
        std::path::PathBuf::from("/mnt/IA/native-trading-ai/frontend/src-tauri/sounds/signal.ogg"),
        // Prod : à côté du binaire
        std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join("sounds/signal.ogg")))
            .unwrap_or_default(),
    ];

    let chemin = candidats.iter().find(|p| p.exists()).cloned();

    if let Some(p) = chemin {
        let _ = Command::new("paplay")
            .arg(p)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// Ouvre une URL externe dans le navigateur par défaut du système.
///
/// En Tauri, un `<a target="_blank">` ne fait RIEN (pas de shell navigateur).
/// L'app tourne dans un sandbox Flatpak : le `xdg-open` du sandbox n'y ouvre
/// pas le navigateur — on tente successivement :
///   (1) le xdg-desktop-portal via D-Bus (gdbus) — mécanisme officiel Flatpak,
///   (2) le `xdg-open` de l'hôte (monté sous /run/host dans les sandboxes
///       Flatpak courants — lui-même portal-aware),
///   (3) le `xdg-open` local du PATH en dernier recours.
///
/// Note : `dbus-send` ne sait pas encoder le `a{sv}` vide qu'exige
/// OpenURI.Open — `gdbus` est l'outil canonique (celui qu'utilise xdg-open).
///
/// Appelée depuis le frontend : `invoke('ouvrir_url', { url })`
#[tauri::command]
fn ouvrir_url(url: &str) {
    // Sécurité : n'ouvrir que des URL http(s) — jamais file:, ssh:, etc.
    if !url.starts_with("https://") && !url.starts_with("http://") {
        eprintln!("[Tauri] ouvrir_url : URL refusée ({url})");
        return;
    }

    // (1) D-Bus : org.freedesktop.portal.OpenURI.OpenURI (méthode vérifiée
    // sur ce portail — pas de binaire externe, aucune dépendance au PATH).
    let portal_ok = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest=org.freedesktop.portal.Desktop",
            "--object-path=/org/freedesktop/portal/desktop",
            "--method=org.freedesktop.portal.OpenURI.OpenURI",
            "",          // fenêtre parente (vide : aucune)
            &format!("\"{url}\""),
            "{}",        // options a{sv} vide
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if portal_ok {
        return;
    }

    // (2) xdg-open de l'hôte (monté dans les sandboxes Flatpak courants).
    let host_ok = Command::new("/run/host/usr/bin/xdg-open")
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if host_ok {
        return;
    }

    // (3) Dernier recours : xdg-open du PATH local.
    let _ = Command::new("xdg-open")
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

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
    let log_path = "/mnt/IA/native-trading-ai/data/logs/backend.log";
    let stdout_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null());
    let stderr_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null());

    match Command::new(&binaire)
        .env("DATABASE_PATH", db_path)
        .env("RUST_LOG", "api::straddle_boucle=debug,info")
        .stdin(Stdio::null())
        .stdout(stdout_file)
        .stderr(stderr_file)
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
        .invoke_handler(tauri::generate_handler![notifier, jouer_son_signal, ouvrir_url])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de l'application Tauri");
}
