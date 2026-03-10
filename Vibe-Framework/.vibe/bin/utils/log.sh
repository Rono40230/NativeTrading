#!/bin/bash
# log.sh - Fonctions de logging centralisées pour VibeOS

LOG_FILE="$VIBE_ROOT/.vibe/logs/sentinel.log"

# Fonction de logging avec niveaux
log() {
    local level=$1
    local message=$2
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message" >> "$LOG_FILE"
    # Aussi afficher en console pour visibilité
    case $level in
        ERROR)
            echo -e "${RED}❌ $message${NC}" >&2
            ;;
        WARNING)
            echo -e "${YELLOW}⚠️  $message${NC}" >&2
            ;;
        INFO)
            echo -e "${BLUE}ℹ️  $message${NC}"
            ;;
        SUCCESS)
            echo -e "${GREEN}✅ $message${NC}"
            ;;
    esac
}

# Alias pour commodité
log_error() { log "ERROR" "$1"; }
log_warning() { log "WARNING" "$1"; }
log_info() { log "INFO" "$1"; }
log_success() { log "SUCCESS" "$1"; }