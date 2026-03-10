#!/bin/bash
# sentinel.sh - Le Gardien Universel VibeOS
# Surveille les changements et orchestre les plugins de vérification.

# --- CONFIGURATION & COLORS ---
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

VIBE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CONFIG_FILE="$VIBE_ROOT/.vibe/config.toml"

# Charger les utilitaires de logging
source "$VIBE_ROOT/.vibe/bin/utils/log.sh"

# Fonction de retry avec logging
retry_command() {
    local cmd=$1
    local description=$2
    local max_attempts=3
    local attempt=1
    while [ $attempt -le $max_attempts ]; do
        log_info "Tentative $attempt/$max_attempts : $description"
        if eval "$cmd"; then
            log_success "$description réussi"
            return 0
        else
            log_warning "Échec tentative $attempt pour $description"
            attempt=$((attempt + 1))
            sleep 1
        fi
    done
    log_error "$description échoué après $max_attempts tentatives"
    return 1
}

# Simple TOML parser wrapper
get_config() {
    local key=$1
    grep "^$key =" "$CONFIG_FILE" | cut -d'=' -f2 | tr -d ' "[]'
}

# Charger la config
STACK=$(get_config "stack")
WATCH_EXT=$(get_config "watch_extensions" | sed 's/,/ -o -name *./g' | sed 's/^/-name *./')
IGNORE_PATHS=$(get_config "ignore_paths" | sed 's/,/ -not -path "*/g' | sed 's/^/-not -path "*/' | sed 's/$/\/*"/' )
TIMEOUT=$(get_config "timeout" || echo 2)
CUSTOM_PLUGINS=$(get_config "custom_plugins" | sed 's/,/ /g')

# Mode debug (désactive auto-fix)
DEBUG_MODE=false
if [ "$1" = "--debug" ]; then
    DEBUG_MODE=true
    log_info "Mode DEBUG activé : auto-fix désactivé"
fi

echo -e "${BLUE}👁️  VIBE SENTINEL ACTIVÉE${NC}"
echo -e "Stack détectée: ${YELLOW}$STACK${NC}"

# Écrire le PID pour arrêt propre
echo $$ > "$HOME/.vibe_sentinel.pid"

# Vérifier si watchexec est installé pour surveillance cross-platform
if command -v watchexec >/dev/null 2>&1; then
    echo -e "${GREEN}🔍 Utilisation de watchexec pour surveillance temps réel${NC}"
    WATCH_CMD="watchexec -r -f '**/*.{rs,vue,ts,js,py,md,toml,json}' -i 'target/**' -i 'node_modules/**' -i '.git/**' -i '.vibe/**' -- echo 'change detected'"
elif command -v inotifywait >/dev/null 2>&1; then
    echo -e "${GREEN}🔍 Utilisation d'inotify pour surveillance temps réel${NC}"
    WATCH_CMD="inotifywait -r -e modify,create,delete --format '%w%f' . --exclude 'target|node_modules|.git|.vibe'"
else
    echo -e "${YELLOW}⚠️  watchexec/inotify non disponible, utilisation du polling${NC}"
    WATCH_CMD=""
fi

LAST_CHECKSUM=""

while true; do
    CYCLE_START=$(date +%s)
    
    if [ -n "$WATCH_CMD" ]; then
        # Surveillance temps réel avec watchexec/inotify
        $WATCH_CMD >/dev/null 2>&1
        # Après événement, procéder à la vérification
    else
        # Polling avec checksum optimisé (fallback)
        # Ne scanner que les fichiers modifiés dans les dernières 5 minutes pour performance
        CURRENT_CHECKSUM=$(find . -type f \( -name "*.rs" -o -name "*.vue" -o -name "*.ts" -o -name "*.js" -o -name "*.py" \) -not -path "*/target/*" -not -path "*/node_modules/*" -not -path "*/.git/*" -not -path "*/.vibe/*" -mtime -0.00347222 -exec md5sum {} + 2>/dev/null | sort | md5sum)
        if [ "$LAST_CHECKSUM" = "$CURRENT_CHECKSUM" ]; then
            sleep $TIMEOUT
            continue
        fi
        LAST_CHECKSUM="$CURRENT_CHECKSUM"
    fi

    echo ""
    echo -e "${BLUE}🔄 Sync...${NC}"
    
    # 1. AUTO-FIX (Plugins) avec gestion erreurs
    if [ -f "debug.lock" ]; then
        echo "🚧 Mode DEBUG actif : Auto-fix désactivé (fichier debug.lock présent)"
    elif [ "$DEBUG_MODE" = false ]; then
        echo "🔧 Auto-fixing..."
        retry_command "\"$VIBE_ROOT/.vibe/plugins/rust/fix.sh\"" "Fix Rust" || log_warning "Fix Rust ignoré après échecs"
        retry_command "\"$VIBE_ROOT/.vibe/plugins/vue/fix.sh\"" "Fix Vue" || log_warning "Fix Vue ignoré après échecs"
        if [[ "$STACK" == *"python"* ]]; then
            retry_command "\"$VIBE_ROOT/.vibe/plugins/python/fix.sh\"" "Fix Python" || log_warning "Fix Python ignoré après échecs"
        fi
        # Plugins personnalisés
        for plugin in $CUSTOM_PLUGINS; do
            if [ -f "$VIBE_ROOT/.vibe/plugins/$plugin" ]; then
                retry_command "\"$VIBE_ROOT/.vibe/plugins/$plugin\"" "Plugin personnalisé $plugin" || log_warning "Plugin $plugin ignoré après échecs"
            fi
        done
    else
        log_info "Auto-fix sauté (mode debug)"
    fi

    # 2. RÈGLES UNIVERSELLES (Taille, Nommage, Sécurité)
    # Vérification taille fichiers (.clinerules règle 16)
    FAIL=0
    echo "📏 Vérification taille fichiers..."
    MAX_VUE=250
    MAX_RUST=300
    MAX_MAIN_RS=120
    SIZE_FAIL=0
    for file in src/**/*.vue src/*.vue; do
        if [ -f "$file" ]; then
            lines=$(wc -l < "$file")
            if [ "$lines" -gt $MAX_VUE ]; then
                log_error "Fichier $file : $lines lignes (> $MAX_VUE) - TROP GROS ! REFACTORISEZ"
                SIZE_FAIL=1
            fi
        fi
    done
    for file in src-tauri/src/*.rs; do
        if [ -f "$file" ] && [[ "$file" != *"main.rs" ]]; then
            lines=$(wc -l < "$file")
            if [ "$lines" -gt $MAX_RUST ]; then
                log_error "Fichier $file : $lines lignes (> $MAX_RUST) - TROP GROS ! REFACTORISEZ"
                SIZE_FAIL=1
            fi
        fi
    done
    if [ -f "src-tauri/src/main.rs" ]; then
        lines=$(wc -l < "src-tauri/src/main.rs")
        if [ "$lines" -gt $MAX_MAIN_RS ]; then
            log_error "Fichier src-tauri/src/main.rs : $lines lignes (> $MAX_MAIN_RS) - TROP GROS ! REFACTORISEZ"
            SIZE_FAIL=1
        fi
    fi
    if [ $SIZE_FAIL -eq 0 ]; then
        log_success "Tailles fichiers OK"
    else
        FAIL=1
        notify-send -u critical -t 0 "✂️ VibeOS - TAILLE CRITIQUE" "Un fichier dépasse la limite ! Refactorisez IMMÉDIATEMENT." 2>/dev/null
    fi
    
    # Vérification de sécurité critique avec gestion erreur
    if ! retry_command "\"$VIBE_ROOT/.vibe/bin/utils/check-security.sh\"" "Vérification sécurité"; then
        log_error "FAILLES DE SÉCURITÉ DÉTECTÉES !"
        FAIL=1
    fi

    # 3. TESTS (Plugins) avec gestion erreurs
    
    # Détection des changements récents pour optimisation des tests
    # On cherche les fichiers modifiés dans les dernières secondes
    RS_CHANGED=$(find src-tauri -name "*.rs" -mmin -0.05 2>/dev/null | grep -v "target" | wc -l)
    VUE_CHANGED=$(find src -name "*.vue" -o -name "*.ts" -o -name "*.js" -mmin -0.05 2>/dev/null | wc -l)
    
    # Si aucun changement détecté (cas polling ou premier run), on teste tout par sécurité
    if [ "$RS_CHANGED" -eq 0 ] && [ "$VUE_CHANGED" -eq 0 ]; then
        RS_CHANGED=1
        VUE_CHANGED=1
    fi

    # NOTE: On ne reset pas FAIL ici, pour garder l'échec de la taille ou sécurité
    if [[ "$STACK" == *"rust"* ]] && [ "$RS_CHANGED" -gt 0 ]; then
        if ! retry_command "\"$VIBE_ROOT/.vibe/plugins/rust/test.sh\"" "Tests Rust"; then 
            log_warning "Erreur dans tests Rust"
            FAIL=1
        fi
    fi
    if [[ "$STACK" == *"vue"* ]] && [ "$VUE_CHANGED" -gt 0 ]; then
        if ! retry_command "\"$VIBE_ROOT/.vibe/plugins/vue/test.sh\"" "Tests Vue"; then 
            log_warning "Erreur dans tests Vue"
            FAIL=1
        fi
    fi
    if [[ "$STACK" == *"python"* ]]; then
        if ! retry_command "\"$VIBE_ROOT/.vibe/plugins/python/test.sh\"" "Tests Python"; then 
            log_warning "Erreur dans tests Python"
            FAIL=1
        fi
    fi
    # Tests personnalisés
    for plugin in $CUSTOM_PLUGINS; do
        test_plugin="${plugin/fix/test}"
        if [ -f "$VIBE_ROOT/.vibe/plugins/$test_plugin" ]; then
            if ! retry_command "\"$VIBE_ROOT/.vibe/plugins/$test_plugin\"" "Tests plugin $test_plugin"; then 
                log_warning "Erreur dans tests $test_plugin"
                FAIL=1
            fi
        fi
    done
    
    if [ $FAIL -eq 0 ]; then
        echo -e "${GREEN}✅ VIBE CHECK: OK${NC}"
    else
        echo -e "${RED}❌ VIBE CHECK: ERROR${NC}"
    fi

    # Calculer et enregistrer les métriques
    CYCLE_END=$(date +%s)
    CYCLE_TIME=$((CYCLE_END - CYCLE_START))
    
    # Lire métriques actuelles
    if [ -f "$VIBE_ROOT/.vibe/metrics.json" ]; then
        TOTAL_CYCLES=$(jq -r '.total_cycles // 0' "$VIBE_ROOT/.vibe/metrics.json")
        AVG_TIME=$(jq -r '.average_cycle_time // 0' "$VIBE_ROOT/.vibe/metrics.json")
        ERRORS=$(jq -r '.errors_count // 0' "$VIBE_ROOT/.vibe/metrics.json")
        SUCCESSES=$(jq -r '.success_count // 0' "$VIBE_ROOT/.vibe/metrics.json")
    else
        TOTAL_CYCLES=0
        AVG_TIME=0
        ERRORS=0
        SUCCESSES=0
    fi
    
    TOTAL_CYCLES=$((TOTAL_CYCLES + 1))
    if [ $FAIL -eq 0 ]; then
        SUCCESSES=$((SUCCESSES + 1))
    else
        ERRORS=$((ERRORS + 1))
    fi
    NEW_AVG=$(( (AVG_TIME * (TOTAL_CYCLES - 1) + CYCLE_TIME) / TOTAL_CYCLES ))
    
    # Écrire métriques
    jq -n \
        --arg total "$TOTAL_CYCLES" \
        --arg last "$CYCLE_TIME" \
        --arg avg "$NEW_AVG" \
        --arg errors "$ERRORS" \
        --arg successes "$SUCCESSES" \
        --argjson plugins "$(echo "$STACK" | jq -R 'split(" ")')" \
        '{total_cycles: ($total | tonumber), last_cycle_time: ($last | tonumber), average_cycle_time: ($avg | tonumber), errors_count: ($errors | tonumber), success_count: ($successes | tonumber), plugins_used: $plugins}' > "$VIBE_ROOT/.vibe/metrics.json.tmp" && mv "$VIBE_ROOT/.vibe/metrics.json.tmp" "$VIBE_ROOT/.vibe/metrics.json"

    # --- AMÉLIORATIONS SOLO (Feedback & Sauvegarde) ---
    
    # 1. Micro-Commits (WIP) sur succès (si et seulement si FAIL est 0)
    if [ $FAIL -eq 0 ]; then
        if [[ -n $(git status -s) ]]; then
            echo -e "${YELLOW}💾 Sauvegarde automatique (WIP)...${NC}"
            git add . >/dev/null 2>&1
            git commit -m "VibeOS Auto-save: $(date '+%Y-%m-%d %H:%M:%S')" --no-verify >/dev/null 2>&1
        fi
    fi

    # 2. Notifications Desktop
    PREV_STATE_FILE="/tmp/vibe_prev_state_$(echo "$PWD" | md5sum | cut -d' ' -f1)" 
    PREV_STATE="0"
    if [ -f "$PREV_STATE_FILE" ]; then PREV_STATE=$(cat "$PREV_STATE_FILE"); fi
    
    if [ $FAIL -eq 0 ] && [ "$PREV_STATE" -ne 0 ]; then
        notify-send -u normal -t 3000 "✅ VibeOS" "Le système est de nouveau stable." 2>/dev/null
    elif [ $FAIL -ne 0 ] && [ "$PREV_STATE" -eq 0 ]; then
        notify-send -u critical -t 5000 "❌ VibeOS" "Erreur détectée ! Vérifiez le terminal." 2>/dev/null
    fi
    echo "$FAIL" > "$PREV_STATE_FILE"

    # Pour inotify, pas de sleep, sinon sleep configurable
    if [ -z "$WATCH_CMD" ]; then
        sleep $TIMEOUT
    fi
done
