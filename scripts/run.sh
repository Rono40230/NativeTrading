#!/bin/bash
set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="$ROOT_DIR/data/logs"
mkdir -p "$LOG_DIR"
mkdir -p "$ROOT_DIR/data"

# ─── Initialisation nvm (npm/node non disponibles hors shell interactif) ──────
export NVM_DIR="/home/rono/.nvm/.nvm"
# shellcheck source=/dev/null
[ -s "$NVM_DIR/nvm.sh" ] && source "$NVM_DIR/nvm.sh" --no-use
# Fallback : utiliser directement le binaire nvm par défaut
if ! command -v npm &>/dev/null; then
  NVM_NODE_BIN=$(ls -d "$NVM_DIR/versions/node"/*/bin 2>/dev/null | tail -1)
  [ -n "$NVM_NODE_BIN" ] && export PATH="$NVM_NODE_BIN:$PATH"
fi

# ─── Fuseau horaire système pour Tauri/WebKit ──────────────────────────────
# WebKit sur Linux peut ignorer /etc/localtime — on force explicitement.
if [ -z "$TZ" ]; then
  SYS_TZ=$(cat /etc/timezone 2>/dev/null || timedatectl show -p Timezone --value 2>/dev/null || echo '')
  [ -n "$SYS_TZ" ] && export TZ="$SYS_TZ"
fi

echo "🚀 Native Trading AI — démarrage..."

# ─── Démarrage Ollama (si pas déjà lancé) ────────────────────────────────────
OLLAMA_BIN=$(command -v ollama 2>/dev/null || ls /usr/local/bin/ollama /usr/bin/ollama ~/.local/bin/ollama 2>/dev/null | head -1)

if ! ss -tlnp 2>/dev/null | grep -q 11434; then
  if [ -z "$OLLAMA_BIN" ]; then
    echo "   ⚠️  Ollama non installé — fonctionnalités IA désactivées"
    echo "      Pour installer : curl -fsSL https://ollama.com/install.sh | sh"
  else
    echo "🤖 Démarrage Ollama..."
    OLLAMA_MODELS="${OLLAMA_MODELS:-$ROOT_DIR/data/ollama}" \
      "$OLLAMA_BIN" serve > "$LOG_DIR/ollama.log" 2>&1 &
    OLLAMA_PID=$!
    for i in $(seq 1 30); do
      ss -tlnp 2>/dev/null | grep -q 11434 && { echo "   ✅ Ollama prêt"; break; }
      sleep 0.5
    done
  fi
else
  echo "   ✅ Ollama déjà en cours"
fi

# ─── Compilation backend si nécessaire ───────────────────────────────────────
echo "🔨 Vérification backend GPU (CUDA)..."
export LIBTORCH=/mnt/IA/libtorch
export XGBOOST_LIB_DIR=/home/rono/.local/lib/python3.14/site-packages/xgboost/lib
export LIBCLANG_PATH=/run/host/usr/lib64
export BINDGEN_EXTRA_CLANG_ARGS="-I/run/host/usr/lib/clang/21/include -I/run/host/usr/include"
export LD_LIBRARY_PATH=$LIBTORCH/lib:$XGBOOST_LIB_DIR:/run/host/usr/lib64:$LD_LIBRARY_PATH
export OMP_NUM_THREADS=1
export MKL_NUM_THREADS=1

# ─── Workaround GCC 15 : libstdc++fs fusionné dans libstdc++ (xgboost_lib-sys v3.0.4) ──
# GCC 15 a supprimé libstdc++fs séparément. On crée une archive vide pour satisfaire le linker.
FAKE_LIBS="$ROOT_DIR/.cargo-fake-libs"
mkdir -p "$FAKE_LIBS"
if [ ! -f "$FAKE_LIBS/libstdc++fs.a" ]; then
  ar rcs "$FAKE_LIBS/libstdc++fs.a"
fi
export RUSTFLAGS="-L $FAKE_LIBS ${RUSTFLAGS:-}"

export CC=clang
export CXX=clang++

cd "$ROOT_DIR/backend"
# Le build doit RÉUSSIR — un échec silencieux ferait tourner un binaire
# PÉRIMÉ (incident du 15/08 : générateurs censés être suspendus toujours
# actifs, signaux Telegram non sollicités).
if ! cargo build -p api --release 2>&1 | grep -E "Compiling|Finished|error"; then
  echo "❌ ÉCHEC du build backend — arrêt (ne pas lancer un binaire périmé)."
  exit 1
fi
if [ ! -f "$ROOT_DIR/backend/target/release/api" ]; then
  echo "❌ Binaire release introuvable après build — arrêt."
  exit 1
fi

# ─── Arrêt propre de TOUS les processus backend ──────────────────────────────
# (peut en exister plusieurs si lancements manuels accumulés)
if pgrep -f "target/(debug|release)/api" > /dev/null 2>&1; then
  echo "🔄 Arrêt instances backend existantes..."
  pkill -9 -f "target/debug/api" 2>/dev/null || true
  pkill -9 -f "target/release/api" 2>/dev/null || true
  # Attendre libération du port 8080
  for i in $(seq 1 20); do
    ss -tlnp 2>/dev/null | grep -q ':8080' || break
    sleep 0.3
  done
fi

# ─── Arrêt des instances résiduelles Vite/Tauri (sessions précédentes) ──────
# Un Vite fantôme sur le port 1420 ferait échouer le nouveau démarrage — et
# le watchdog (voir bas de script) arrêterait alors toute l'app immédiatement.
VITE_STALE_PID=$(ss -tlnp 2>/dev/null | grep ':1420' | grep -oP 'pid=\K[0-9]+' | head -1)
if [ -n "$VITE_STALE_PID" ]; then
  echo "🔄 Arrêt Vite résiduel (pid $VITE_STALE_PID, port 1420)..."
  kill "$VITE_STALE_PID" 2>/dev/null || true
  for i in $(seq 1 10); do
    ss -tln 2>/dev/null | grep -q ':1420' || break
    sleep 0.3
  done
fi
# Tauri résiduel : par nom EXACT de process uniquement. JAMAIS par motif de
# chemin (pkill -f) — un chemin matcherait le présent script si on l'invoque
# en absolu, tuant le terminal de l'appelant (bug corrigé 2026-08-15).
pkill -x native-trading-ai 2>/dev/null || true

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

# ── Collecteur de presse (process séparé, hors watchdog : sa mort n'arrête
# rien — gate 4. Rejoint la flotte des producteurs isolés.)
# pkill par nom EXACT avant lancement (règle L1) : évite les doublons d'un
# run précédent. COLLECTOR_PID volontairement NI dans cleanup NI dans le
# watchdog de fin de script.
echo "📰 Collecteur de presse"
pkill -x news_collector 2>/dev/null || true
DATABASE_PATH="$ROOT_DIR/data/trading.db" \
  "$ROOT_DIR/backend/target/release/news_collector" \
  > "$LOG_DIR/news_collector.log" 2>&1 &
COLLECTOR_PID=$!

# ─── Démarrage frontend Tauri ─────────────────────────────────────────────────
echo "🖥️  Lancement fenêtre Tauri..."
cd "$ROOT_DIR/frontend"

# Bibliothèques WebKit/GTK depuis le système hôte (non visibles du sandbox Flatpak)
HOST_LIB="/run/host/usr/lib64"
HOST_PULSE="/run/host/usr/lib64/pulseaudio"
export LD_LIBRARY_PATH="$HOST_LIB:$HOST_PULSE:${LD_LIBRARY_PATH:-}"

TAURI_BIN="$ROOT_DIR/frontend/src-tauri/target/debug/native-trading-ai"
TAURI_BIN_REL="$ROOT_DIR/frontend/src-tauri/target/release/native-trading-ai"

if [ -f "$TAURI_BIN_REL" ]; then
  TAURI_BIN="$TAURI_BIN_REL"
fi

if [ -f "$TAURI_BIN" ]; then
  # Lancer Vite (dev server) + binaire pré-compilé directement (évite recompilation avec libs manquantes)
  npx vite --port 1420 > "$LOG_DIR/vite.log" 2>&1 &
  VITE_PID=$!
  sleep 2  # attendre que Vite soit prêt
  GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    "$TAURI_BIN" > "$LOG_DIR/tauri.log" 2>&1 &
  TAURI_PID=$!
else
  # Fallback : tauri dev (nécessite les libs devel installées)
  GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    npm run tauri:start > "$LOG_DIR/frontend.log" 2>&1 &
  TAURI_PID=$!
fi

echo ""
echo "╔════════════════════════════════════════╗"
echo "║  ✅ Native Trading AI — en cours       ║"
echo "║  🖥️  Fenêtre native Tauri ouverte       ║"
echo "║  🔌 API interne : localhost:8080        ║"
echo "║  📋 Logs : data/logs/                  ║"
echo "║  🛑 Arrêter : fermer la fenêtre (X) ou Ctrl+C ║"
echo "╚════════════════════════════════════════╝"

# Suivre les logs backend en temps réel (nouvelles lignes seulement)
tail -f -n 0 "$LOG_DIR/backend.log" &
TAIL_PID=$!

NETTOYE_FAIT=0
cleanup() {
  [ "$NETTOYE_FAIT" -eq 1 ] && return
  NETTOYE_FAIT=1
  echo ""
  echo "🛑 Arrêt de l'application (backend + UI + Vite)..."
  kill $BACKEND_PID $TAURI_PID ${VITE_PID:-} $TAIL_PID 2>/dev/null
  wait 2>/dev/null
  echo "✅ Arrêt propre — tout est clos."
}
trap cleanup INT TERM

# ── Fermeture de la fenêtre (X) = arrêt COMPLET ──────────────────────────────
# Le process Tauri meurt quand on ferme la fenêtre ; le backend ou Vite peuvent
# aussi tomber seuls. On surveille les trois : la fin de L'UN QUELCONQUE
# déclenche l'arrêt propre de tous les autres (compat bash sans wait -n).
while kill -0 "$BACKEND_PID" 2>/dev/null    && { [ -z "${TAURI_PID:-}" ] || kill -0 "$TAURI_PID" 2>/dev/null; }    && { [ -z "${VITE_PID:-}" ] || kill -0 "$VITE_PID" 2>/dev/null; }; do
  sleep 1
done
cleanup
