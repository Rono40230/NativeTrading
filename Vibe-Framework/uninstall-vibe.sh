#!/bin/bash
# uninstall-vibe.sh - Désinstalleur VibeOS

echo "🔮 Désinstallation de VibeOS..."

# Arrêter le sentinel si en cours
if [ -f "$HOME/.vibe_sentinel.pid" ]; then
    ./.vibe/bin/stop.sh
fi

# Supprimer les fichiers
rm -rf .vibe
rm -rf .vibe.bak
rm -f vibe
rm -f vibe.bak
rm -f .clinerules.bak
rm -rf .vibe_logs  # Si logs ailleurs, mais normalement dans .vibe

echo "✅ VibeOS désinstallé avec succès !"