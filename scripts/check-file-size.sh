#!/bin/bash
# check-file-size.sh — Vérifie la taille de tous les fichiers source
# Règle .clinerules #16 : alerte à 250 lignes, limite dure à 300 lignes

set -e

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BOLD='\033[1m'
NC='\033[0m'

WARN_LIMIT=250
HARD_LIMIT=300

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

warn_files=()
over_files=()

check_file() {
    local f="$1"
    local lines
    lines=$(wc -l < "$f")
    local rel="${f#$ROOT/}"

    if [ "$lines" -gt "$HARD_LIMIT" ]; then
        over_files+=("$lines $rel")
    elif [ "$lines" -gt "$WARN_LIMIT" ]; then
        warn_files+=("$lines $rel")
    fi
}

# Extensions à surveiller
while IFS= read -r -d '' file; do
    check_file "$file"
done < <(find "$ROOT" \
    -type f \( -name "*.rs" -o -name "*.vue" -o -name "*.ts" -o -name "*.js" \) \
    -not -name "*.vue.js" \
    -not -path "*/target/*" \
    -not -path "*/node_modules/*" \
    -not -path "*/dist/*" \
    -not -path "*/.git/*" \
    -not -path "*/.vibe/*" \
    -not -path "*/gen/*" \
    -print0)

echo ""
echo -e "${BOLD}📏 VIBE — Contrôle taille fichiers${NC}"
echo -e "   Alerte : >${WARN_LIMIT} lignes | Limite : >${HARD_LIMIT} lignes"
echo ""

if [ ${#over_files[@]} -gt 0 ]; then
    echo -e "${RED}${BOLD}❌ LIMITE DÉPASSÉE — À refactoriser immédiatement :${NC}"
    for entry in "${over_files[@]}"; do
        lines="${entry%% *}"
        path="${entry#* }"
        echo -e "   ${RED}● ${path} — ${BOLD}${lines} lignes${NC}"
    done
    echo ""
fi

if [ ${#warn_files[@]} -gt 0 ]; then
    echo -e "${YELLOW}${BOLD}⚠️  ALERTE — Approche de la limite :${NC}"
    for entry in "${warn_files[@]}"; do
        lines="${entry%% *}"
        path="${entry#* }"
        echo -e "   ${YELLOW}● ${path} — ${lines} lignes${NC}"
    done
    echo ""
fi

if [ ${#over_files[@]} -eq 0 ] && [ ${#warn_files[@]} -eq 0 ]; then
    echo -e "${GREEN}✅ Tous les fichiers respectent la limite (≤${WARN_LIMIT} lignes)${NC}"
fi

echo ""

# Exit code 1 si des fichiers dépassent la limite dure
if [ ${#over_files[@]} -gt 0 ]; then
    exit 1
fi
exit 0
