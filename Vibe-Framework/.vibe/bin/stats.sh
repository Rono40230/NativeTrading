#!/bin/bash
# stats.sh - Afficher les métriques VibeOS

VIBE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
METRICS_FILE="$VIBE_ROOT/.vibe/metrics.json"

if [ ! -f "$METRICS_FILE" ]; then
    echo "❌ Aucune métrique trouvée. Lancez d'abord la sentinelle."
    exit 1
fi

echo "📊 Métriques Vibe-Framework"
echo "=========================="

TOTAL=$(jq -r '.total_cycles' "$METRICS_FILE")
LAST=$(jq -r '.last_cycle_time' "$METRICS_FILE")
AVG=$(jq -r '.average_cycle_time' "$METRICS_FILE")
ERRORS=$(jq -r '.errors_count' "$METRICS_FILE")
SUCCESSES=$(jq -r '.success_count' "$METRICS_FILE")
PLUGINS=$(jq -r '.plugins_used | join(", ")' "$METRICS_FILE")

echo "Cycles totaux : $TOTAL"
echo "Dernier cycle : ${LAST}s"
echo "Moyenne cycle : ${AVG}s"
echo "Succès : $SUCCESSES"
echo "Erreurs : $ERRORS"
echo "Plugins utilisés : $PLUGINS"

if [ "$TOTAL" -gt 0 ]; then
    SUCCESS_RATE=$((SUCCESSES * 100 / TOTAL))
    echo "Taux de succès : ${SUCCESS_RATE}%"
fi