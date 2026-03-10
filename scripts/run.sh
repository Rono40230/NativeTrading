#!/bin/bash
set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="$ROOT_DIR/data/logs"
mkdir -p "$LOG_DIR"
mkdir -p "$ROOT_DIR/data"

echo "🚀 Native Trading AI — démarrage..."

# ─── Compilation backend si nécessaire ───────────────────────────────────────
echo "🔨 Vérification backend..."
cd "$ROOT_DIR/backend"
cargo build -p api --release 2>&1 | grep -E "Compiling|Finished|error"

# ─── Démarrage backend ────────────────────────────────────────────────────────
echo "🔌 Backend API → port 8080"
DATABASE_PATH="$ROOT_DIR/data/trading.db" \
  "$ROOT_DIR/backend/target/release/api" \
  > "$LOG_DIR/backend.log" 2>&1 &
BACKEND_PID=$!

# Attendre que le backend soit prêt (max 10s)
echo -n "⏳ Attente backend"
for i in $(seq 1 20); do
  if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
    echo " ✅"
    break
  fi
  sleep 0.5
  echo -n "."
done

# ─── Démarrage frontend Tauri ─────────────────────────────────────────────────
echo "🖥️  Lancement fenêtre Tauri..."
cd "$ROOT_DIR/frontend"
# Forcer X11 — WebKitGTK a des problèmes avec Wayland (erreur protocole 71)
GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
  npm run tauri:start > "$LOG_DIR/frontend.log" 2>&1 &
TAURI_PID=$!

echo ""
echo "╔════════════════════════════════════════╗"
echo "║  ✅ Native Trading AI — en cours       ║"
echo "║  🖥️  Fenêtre native Tauri ouverte       ║"
echo "║  🔌 API interne : localhost:8080        ║"
echo "║  📋 Logs : data/logs/                  ║"
echo "║  🛑 Arrêter : Ctrl+C                   ║"
echo "╚════════════════════════════════════════╝"

# Suivre les logs backend en temps réel (nouvelles lignes seulement)
tail -f -n 0 "$LOG_DIR/backend.log" &
TAIL_PID=$!

cleanup() {
  echo ""
  echo "🛑 Arrêt de l'application..."
  kill $BACKEND_PID $TAURI_PID $TAIL_PID 2>/dev/null
  wait 2>/dev/null
  echo "✅ Arrêt propre."
}
trap cleanup INT TERM

wait $BACKEND_PID
