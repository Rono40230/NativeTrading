#!/bin/bash
# vibe.sh - Commande principale VibeOS

VIBE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

case "$1" in
    stats)
        "$VIBE_ROOT/.vibe/bin/stats.sh"
        ;;
    --debug)
        "$VIBE_ROOT/.vibe/bin/sentinel.sh" --debug
        ;;
    *)
        "$VIBE_ROOT/.vibe/bin/sentinel.sh" "$@"
        ;;
esac