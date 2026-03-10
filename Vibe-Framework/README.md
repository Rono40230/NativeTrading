# Vibe-Framework 🧠

Un environnement de développement assisté par IA pour des projets Rust + Vue.js (avec Tauri). VibeOS surveille votre code en temps réel, applique des corrections automatiques, et garantit la qualité via des tests et audits de sécurité.

## 🚀 Installation Rapide

### Option 1 : Script Automatique (Recommandé)
Pour créer un nouveau projet prêt à l'emploi :
```bash
./start-new-project.sh
```
Le script demande le nom et la stack, crée le projet, installe Vibe, et configure automatiquement.

### Option 2 : Installation Manuelle
1. **Téléchargez ou clonez** ce repo dans votre projet.
2. **Installez les dépendances** : `cargo`, `npm`, et optionnellement `inotify-tools` ou `watchexec`.
3. **Lancez l'installation** :
   ```bash
   ./install-vibe.sh
   ```
4. **Démarrez la surveillance** :
   ```bash
   ./vibe
   ```

## 📋 Fonctionnalités

- **Surveillance temps réel** : Détecte les changements et corrige automatiquement (formatage, linting).
- **Tests intégrés** : Lance tests unitaires, E2E, et audits de sécurité.
- **Multi-stack** : Support Rust, Vue.js, Python.
- **Configuration flexible** : Personnalisez via `.vibe/config.toml`.
- **Logging et métriques** : Suivez les performances dans `.vibe/logs/` et `.vibe/metrics.json`.

## 🛠️ Utilisation

### Démarrage
```bash
./vibe  # Lance la sentinelle en arrière-plan
```

### Audit Final (avant commit)
```bash
./.vibe/bin/audit.sh
```

### Statistiques
```bash
./vibe stats  # Affiche métriques (cycles, temps, erreurs)
```

### Mode Debug (sans auto-fix)
```bash
./vibe --debug
```

## ⚙️ Configuration

Modifiez `.vibe/config.toml` pour adapter :
- Stack : `stack = ["rust", "vue"]`
- Langage : `language = "fr"`
- Seuils : `security_threshold = "high"`
- Overrides : `allow_console_log = false`

## 📖 Tutoriel

1. **Créez un nouveau projet** : `mkdir my-app && cd my-app`
2. **Installez Vibe** : Copiez les fichiers et lancez `./install-vibe.sh`
3. **Codez** : Écrivez en Rust/Vue, la sentinelle corrige automatiquement.
4. **Testez** : Les tests se lancent à chaque sauvegarde.
5. **Validez** : `./.vibe/bin/audit.sh` avant commit.

### Exemple de Projet
- Frontend Vue.js dans `src/`
- Backend Rust dans `src-tauri/`
- Tests dans `tests/` ou `__tests__/`

## 🔧 Dépannage

- **Erreur de permissions** : `chmod +x .vibe/bin/*.sh`
- **Outils manquants** : Installez via `./install-vibe.sh` (auto-détection)
- **Logs** : Consultez `.vibe/logs/sentinel.log`
- **Métriques** : `.vibe/metrics.json` pour performances

## 📸 Captures d'Écran

- ![Installation](screenshots/install.png) - Processus d'installation
- ![Sentinel](screenshots/sentinel.png) - Sentinelle en action
- ![Audit](screenshots/audit.png) - Résultat d'audit réussi

## 🎥 Vidéos

- [Installation Rapide](videos/install.mp4) - 2 min tutoriel
- [Usage Quotidien](videos/usage.mp4) - Démo complète

## 🤝 Contribution

Ce framework est open-source. Pour contribuer :
1. Forkez le repo
2. Créez une branche feature
3. Soumettez une PR avec tests

## 📄 Licence

MIT License - Libre d'usage.

---

**Vibe-Framework** : Codez fluide, livrez solide. 🚀</content>
<parameter name="filePath">/home/rono/Démarrage/Vibe-Framework/README.md