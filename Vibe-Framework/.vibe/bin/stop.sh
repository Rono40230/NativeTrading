#!/bin/bash
# stop.sh - Arrêter le Sentinel proprement

PID_FILE="$HOME/.vibe_sentinel.pid"

if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID"
        echo "✅ Sentinel arrêté (PID: $PID)"
    else
        echo "⚠️  Sentinel déjà arrêté"
    fi
    rm -f "$PID_FILE"
else
    echo "⚠️  Aucun PID trouvé, Sentinel peut-être déjà arrêté"
fi