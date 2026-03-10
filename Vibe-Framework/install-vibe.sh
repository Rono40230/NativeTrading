#!/bin/bash
# install-vibe.sh - Installeur Universel VibeOS

# Vérifications pré-installation
if [ ! -w "." ]; then
    echo "❌ Pas de permissions d'écriture dans ce dossier."
    exit 1
fi

if [ -f "vibe" ] && [ ! -L "vibe" ]; then
    echo "⚠️  Fichier 'vibe' existe et n'est pas un lien. Sauvegarde en vibe.bak"
    mv vibe vibe.bak
fi

# Vérification des prérequis
echo "🔍 Vérification des prérequis..."
MISSING_DEPS=()

if ! command -v cargo >/dev/null 2>&1; then
    MISSING_DEPS+=("cargo (Rust)")
fi

if ! command -v npm >/dev/null 2>&1; then
    MISSING_DEPS+=("npm (Node.js)")
fi

if ! command -v inotifywait >/dev/null 2>&1; then
    MISSING_DEPS+=("inotify-tools")
fi

if [ ${#MISSING_DEPS[@]} -ne 0 ]; then
    echo "❌ Dépendances manquantes : ${MISSING_DEPS[*]}"
    echo "📖 Liens d'installation :"
    echo "   - Rust/Cargo : https://rustup.rs/"
    echo "   - Node.js/npm : https://nodejs.org/"
    echo "   - inotify-tools : sudo apt install inotify-tools (Ubuntu/Debian) ou brew install inotify-tools (macOS)"
    echo ""
    echo "🔧 Voulez-vous essayer une installation automatique ? (y/N)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        # Détecter le système
        if command -v apt >/dev/null 2>&1; then
            echo "📦 Installation via apt..."
            sudo apt update && sudo apt install -y inotify-tools
        elif command -v dnf >/dev/null 2>&1; then
            echo "📦 Installation via dnf..."
            sudo dnf install -y inotify-tools
        elif command -v yum >/dev/null 2>&1; then
            echo "📦 Installation via yum..."
            sudo yum install -y inotify-tools
        elif command -v pacman >/dev/null 2>&1; then
            echo "📦 Installation via pacman..."
            sudo pacman -S --noconfirm inotify-tools
        elif command -v brew >/dev/null 2>&1; then
            echo "📦 Installation via brew..."
            brew install inotify-tools
        else
            echo "❌ Gestionnaire de paquets non détecté. Installez manuellement."
            exit 1
        fi
        # Revérifier
        if ! command -v inotifywait >/dev/null 2>&1; then
            echo "❌ Échec de l'installation automatique. Installez manuellement."
            exit 1
        fi
    else
        echo "❌ Installez les dépendances manquantes et relancez l'installation."
        exit 1
    fi
fi

echo "✅ Prérequis OK."

echo "🔮 Installation de VibeOS..."

# Définir la source (le dossier où script est lancé ou passé en arg)
if [ -n "$1" ]; then
    SOURCE_DIR="$1"
else
    SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fi

# Copier .vibe
if [ -d ".vibe" ]; then
    echo "⚠️  Le dossier .vibe existe déjà. Sauvegarde en .vibe.bak"
    mv .vibe .vibe.bak
fi

cp -r "$SOURCE_DIR/.vibe" .
if [ $? -ne 0 ]; then
    echo "⚠️  .vibe non trouvé dans SOURCE_DIR, utilisation de la sauvegarde si disponible"
    if [ -d ".vibe.bak" ]; then
        mv .vibe.bak .vibe
    else
        echo "❌ Impossible de trouver .vibe"
        exit 1
    fi
fi
chmod +x .vibe/bin/*.sh
chmod +x .vibe/plugins/*/*.sh

# Créer un lien symbolique pour la commande facile
ln -sf ./.vibe/bin/vibe.sh vibe

# Générer .clinerules dynamiquement à partir de config.toml
generate_clinerules() {
    local config_file=".vibe/config.toml"
    local clinerules_file=".clinerules"
    
    if [ -f "$clinerules_file" ]; then
        echo "ℹ️  .clinerules existant conservé (règles IA avancées)."
        return
    fi

    echo "# .clinerules - Vos règles projet générées dynamiquement" > "$clinerules_file"
    echo "# Basé sur .vibe/config.toml" >> "$clinerules_file"
    echo "" >> "$clinerules_file"
    
    # Règle de nommage selon language
    local language=$(grep "^language =" "$config_file" | cut -d'=' -f2 | tr -d ' "')
    if [[ "$language" == *"fr"* ]]; then
        echo "RÈGLE 1 : Nommage en Français (language = $language)" >> "$clinerules_file"
    else
        echo "RÈGLE 1 : Nommage selon language = $language" >> "$clinerules_file"
    fi
    
    # Règles selon forbidden_patterns et overrides
    local allow_console_log=$(sed -n '/^\[overrides\]/,/^\[/p' "$config_file" | grep "^allow_console_log =" | cut -d'=' -f2 | tr -d ' "')
    if [ "$allow_console_log" != "true" ]; then
        echo "RÈGLE 2 : Pas de console.log" >> "$clinerules_file"
    fi
    
    local allow_unwrap=$(sed -n '/^\[overrides\]/,/^\[/p' "$config_file" | grep "^allow_unwrap =" | cut -d'=' -f2 | tr -d ' "')
    if [ "$allow_unwrap" != "true" ]; then
        echo "RÈGLE 3 : Pas d'unwrap() en Rust" >> "$clinerules_file"
    fi
    
    # Autres règles fixes
    echo "RÈGLE 4 : Pas de TODO sans nom (utilisez TODO(nom): )" >> "$clinerules_file"
    echo "RÈGLE 5 : Respecter max_file_lines = $(grep "^max_file_lines =" "$config_file" | cut -d'=' -f2 | tr -d ' ')" >> "$clinerules_file"
}

generate_clinerules

echo "✅ VibeOS installé avec succès !"
echo "👉 Lancez './vibe' pour démarrer la sentinelle."
echo "👉 Utilisez './.vibe/bin/stop.sh' pour arrêter."
