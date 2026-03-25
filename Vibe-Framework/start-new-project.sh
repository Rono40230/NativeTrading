#!/bin/bash
# start-new-project.sh - Script de démarrage pour créer un nouveau projet avec Vibe-Framework

echo "🚀 Création d'un Nouveau Projet avec Vibe-Framework"
echo "=================================================="

# Demander le nom du projet
read -p "Nom du projet : " PROJECT_NAME
if [ -z "$PROJECT_NAME" ]; then
    echo "❌ Nom requis."
    exit 1
fi

# Définir le répertoire parent (au même niveau que Vibe-Framework)
PARENT_DIR="$(cd .. && pwd)"

# Demander la stack
echo "Choisissez la stack :"
echo "1) Rust + Vue.js (Tauri app)"
echo "2) Python seul"
echo "3) Autre (manuel)"
read -p "Choix (1-3) : " STACK_CHOICE

case $STACK_CHOICE in
    1)
        STACK="rust-vue"
        echo "📦 Création d'une app Tauri (Rust + Vue)..."
        if ! command -v npx >/dev/null 2>&1; then
            echo "❌ npx requis. Installe Node.js."
            exit 1
        fi
        cd "$PARENT_DIR" && npx create-tauri-app "$PROJECT_NAME" --template vue --yes
        ;;
    2)
        STACK="python"
        echo "🐍 Création d'un projet Python..."
        cd "$PARENT_DIR" && mkdir "$PROJECT_NAME"
        cd "$PROJECT_NAME"
        echo "# $PROJECT_NAME" > README.md
        echo "python>=3.8" > requirements.txt
        ;;
    3)
        STACK="manual"
        echo "📁 Création d'un dossier vide..."
        cd "$PARENT_DIR" && mkdir "$PROJECT_NAME"
        cd "$PROJECT_NAME"
        ;;
    *)
        echo "❌ Choix invalide."
        exit 1
        ;;
esac

cd "$PARENT_DIR/$PROJECT_NAME" || exit 1

# Copier Vibe-Framework
VIBE_SOURCE="$(cd .. && pwd)/Vibe-Framework"
echo "🔮 Installation de Vibe-Framework..."
cp -r "$VIBE_SOURCE"/* .
cp -r "$VIBE_SOURCE"/.* . 2>/dev/null || true  # Copier les fichiers cachés comme .vibe
rm -rf screenshots videos  # Nettoyer les médias du template

# Injecter le template global de preferences si present
if [ -f "$VIBE_SOURCE/global-vibe-rules.toml" ]; then
    mkdir -p .vibe
    cp "$VIBE_SOURCE/global-vibe-rules.toml" .vibe/global-vibe-rules.toml
    echo "✅ Template global vibe copié vers .vibe/global-vibe-rules.toml"
fi

# Installer Vibe
./install-vibe.sh "$VIBE_SOURCE"

# Vérifier si l'installation a réussi
if [ $? -ne 0 ]; then
    echo "❌ Échec de l'installation Vibe-Framework. Vérifiez les dépendances et relancez."
    exit 1
fi

# Configurer la stack dans config.toml
case $STACK in
    rust-vue)
        sed -i 's/stack = .*/stack = ["rust", "vue"]/' .vibe/config.toml
        
        echo "🛡️ Configuration Shift-Left (ESLint & Rust Clippy)..."
        
        # 1. Installer ESLint pour Vue
        if [ -f "package.json" ]; then
            npm install --save-dev eslint eslint-plugin-vue globals >/dev/null 2>&1
            cat > eslint.config.js << 'EOF'
import globals from "globals";
import pluginVue from "eslint-plugin-vue";

/** @type {import('eslint').Linter.Config[]} */
export default [
  {files: ["**/*.{js,mjs,cjs,vue}"]},
  {languageOptions: { globals: globals.browser }},
  ...pluginVue.configs["flat/essential"],
  {
    rules: {
      "no-console": "warn",
      "vue/max-len": ["warn", { 
        "code": 150,
        "template": 150,
        "ignoreStrings": true,
        "ignoreUrls": true
      }]
    }
  }
];
EOF
            echo "   ✅ ESLint configuré."
        fi

        # 2. Injecter les warnings Rust (Anti-Unwrap)
        if [ -d "src-tauri/src" ]; then
            for file in src-tauri/src/main.rs src-tauri/src/lib.rs; do
                if [ -f "$file" ]; then
                    # Ajouter les warnings en haut du fichier
                    temp_file=$(mktemp)
                    echo "#![warn(clippy::unwrap_used)]" > "$temp_file"
                    echo "#![warn(clippy::expect_used)]" >> "$temp_file"
                    echo "#![warn(clippy::panic)]" >> "$temp_file"
                    cat "$file" >> "$temp_file"
                    mv "$temp_file" "$file"
                    echo "   ✅ Rust Clippy warnings ajoutés dans $file"
                fi
            done
        fi
        ;;
    python)
        sed -i 's/stack = .*/stack = ["python"]/' .vibe/config.toml
        ;;
    manual)
        echo "⚙️ Configurez .vibe/config.toml manuellement."
        ;;
esac

# Appliquer les preferences globales valideses par defaut
if [ -f ".vibe/config.toml" ]; then
    sed -i 's/^language = .*/language = "agnostic"/' .vibe/config.toml
    sed -i 's/^require_docs = .*/require_docs = false/' .vibe/config.toml
    sed -i 's/^require_function_docs = .*/require_function_docs = false/' .vibe/config.toml
    sed -i 's/^require_module_docs = .*/require_module_docs = false/' .vibe/config.toml

    if grep -q '^max_file_lines = ' .vibe/config.toml; then
        if grep -q '^file_lines_warning = ' .vibe/config.toml; then
            sed -i 's/^file_lines_warning = .*/file_lines_warning = 250/' .vibe/config.toml
        else
            sed -i '/^max_file_lines = /a file_lines_warning = 250' .vibe/config.toml
        fi
    fi

    # Remplacer ou ajouter la section frontend_ui pour standardiser les apps
    temp_cfg=$(mktemp)
    awk '
    BEGIN { skip=0 }
    /^\[frontend_ui\]/ { skip=1; next }
    /^\[/ && skip==1 { skip=0 }
    skip==0 { print }
    ' .vibe/config.toml > "$temp_cfg"
    mv "$temp_cfg" .vibe/config.toml
    cat >> .vibe/config.toml << 'EOF'

[frontend_ui]
dark_mode_required = true
tauri_vue_required = true
collapsible_sidebar_required = true
horizontal_menu_forbidden = true
dropdown_background = "white"
dropdown_text_color = "black"
EOF

    echo "✅ Préférences globales appliquées dans .vibe/config.toml"
fi

# Installer un hook Git pre-commit pour audits automatiques (si repo Git existe)
if [ -d ".git" ]; then
    mkdir -p .git/hooks
    cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
echo "🔍 Exécution de l'audit Vibe avant commit..."
if ./.vibe/bin/audit.sh; then
    echo "✅ Audit passé : commit autorisé."
    exit 0
else
    echo "❌ Audit échoué : corrigez les erreurs avant de committer."
    exit 1
fi
EOF
    chmod +x .git/hooks/pre-commit
    echo "🔒 Hook pre-commit installé : audits automatiques sur chaque commit."
fi

# Créer le fichier de tâche contextuel (Shift Left pour l'IA)
cat > task.md << 'EOF'
# Tâche en Cours

## 🧠 Contexte & Règles (Lisez-moi !)
> **⚠️ RÈGLES CRITIQUES VIBE OS**
> 1. **Anti-Unwrap** : Interdiction totale de `unwrap()` en Rust. Utilisez `match` ou `?`.
> 2. **Taille Fichiers** : Max 300 lignes. Si > 250, refactorisez IMMÉDIATEMENT.
> 3. **Console** : Pas de `console.log` en prod (utilisez le logger Vibe).

---

## 📋 Todo List
- [ ] Initialiser la tâche...
EOF

echo "✅ Projet '$PROJECT_NAME' créé avec Vibe-Framework !"
echo ""
echo "📂 Structure :"
ls -la
echo ""
echo "🚀 Démarrage automatique de la surveillance Vibe..."
./vibe &
echo "✅ Surveillance démarrée en arrière-plan."
echo "📖 Consultez README.md pour plus d'infos."