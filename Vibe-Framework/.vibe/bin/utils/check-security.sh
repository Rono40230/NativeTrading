#!/bin/bash
# check-security.sh - Gardien de Sécurité IA
# Bloque les failles de sécurité courantes générées par l'IA (XSS, Unsafe, Secrets)

EXIT_CODE=0
VIBE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
CONFIG_FILE="$VIBE_ROOT/.vibe/config.toml"

# Fonction pour lire config
get_config() {
    local key=$1
    grep "^$key =" "$CONFIG_FILE" | cut -d'=' -f2 | tr -d ' "[]'
}

# Fonction pour lire overrides
get_override() {
    local key=$1
    sed -n '/^\[overrides\]/,/^\[/p' "$CONFIG_FILE" | grep "^$key =" | cut -d'=' -f2 | tr -d ' "'
}

# Charger seuils
SECURITY_THRESHOLD=$(get_config "security_threshold" || echo "high")

# Charger language
LANGUAGE=$(get_config "language" || echo "auto")

# Charger forbidden_patterns
FORBIDDEN_PATTERNS=$(grep -A 10 "^forbidden_patterns =" "$CONFIG_FILE" | grep -E '^\s*"' | sed 's/.*"//' | sed 's/".*//' | tr '\n' ' ')

# Ajuster selon overrides
if [ "$ALLOW_CONSOLE_LOG" = "true" ]; then
    FORBIDDEN_PATTERNS=$(echo "$FORBIDDEN_PATTERNS" | sed 's/console\\.log//g')
fi
if [ "$ALLOW_UNWRAP" = "true" ]; then
    FORBIDDEN_PATTERNS=$(echo "$FORBIDDEN_PATTERNS" | sed 's/unwrap\\(\\)//g')
fi

echo "🛡️  Analyse de sécurité VibeOS..."

# 1. Frontend : Détection v-html (Risque XSS)
echo "   🔍 Audit Frontend (Vue.js)..."
if grep -r "v-html" src/ --include="*.vue" --exclude-dir=node_modules; then
    echo "❌ SÉCURITÉ : 'v-html' détecté ! Risque XSS critique."
    echo "   👉 Utilisez {{ mustache }} ou un sanitizer."
    EXIT_CODE=1
fi

# 2. Frontend : Détection manipulation DOM directe
if grep -r "document.getElement" src/ --include="*.vue" --include="*.ts" --exclude-dir=node_modules; then
    echo "⚠️  WARNING : Manipulation DOM directe détectée."
    echo "   👉 Utilisez des 'ref' Vue.js."
    # Warning seulement, ne bloque pas obligatoirement
fi

# 3. Vérification des patterns interdits
echo "   🔍 Vérification des patterns interdits..."
for pattern in $FORBIDDEN_PATTERNS; do
    if [ -n "$pattern" ]; then
        if grep -r "$pattern" . --include="*.rs" --include="*.vue" --include="*.ts" --include="*.js" --exclude-dir=node_modules --exclude-dir=target --exclude-dir=.git --exclude-dir=.vibe; then
            echo "❌ PATTERN INTERDIT : '$pattern' détecté !"
            EXIT_CODE=1
        fi
    fi
done

# 3. Backend : Détection blocs unsafe (Rust)
echo "   🔍 Audit Backend (Rust)..."
if grep -r "unsafe {" src-tauri/src/ --include="*.rs"; then
    echo "❌ SÉCURITÉ : Bloc 'unsafe' détecté en Rust."
    echo "   👉 Interdit sauf justification FFI explicite."
    EXIT_CODE=1
fi

# 4. Général : Détection de secrets/clés (basique)
echo "   🔍 Audit Secrets..."
if grep -rE "API_KEY|SECRET|PASSWORD" . --include="*.env" 2>/dev/null; then
    # On vérifie juste qu'on ne commite pas de secrets en dur
    echo "ℹ️  Info : Vérifiez que vos secrets sont dans .env et ignorés par git."
fi

# 5. Audit avancé (si disponible)
if [ -f "$VIBE_ROOT/.vibe/plugins/security/audit.sh" ]; then
    if ! "$VIBE_ROOT/.vibe/plugins/security/audit.sh"; then
        EXIT_CODE=1
    fi
fi

if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ SÉCURITÉ : Aucun risque critique détecté."
    exit 0
else
    echo "🔴 ÉCHEC AUDIT SÉCURITÉ"
    exit 1
fi
