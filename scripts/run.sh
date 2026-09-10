#!/bin/bash
set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LOG_DIR="$ROOT_DIR/data/logs"
mkdir -p "$LOG_DIR"
mkdir -p "$ROOT_DIR/data"

# ── Sauvegarde de la base (24 mois d'historique irremplaçable) ────────────────
# Copie horodatée à chaque démarrage, rétention 30 jours. Utilise sqlite3
# .backup si disponible (copie cohérente même base ouverte), repli cp.
if [ -f "$ROOT_DIR/data/trading.db" ]; then
   BACKUP_DIR="$ROOT_DIR/data/backups"
   mkdir -p "$BACKUP_DIR"
   STAMP=$(date +%Y%m%d-%H%M%S)
   if command -v sqlite3 &>/dev/null; then
      sqlite3 "$ROOT_DIR/data/trading.db" ".backup '$BACKUP_DIR/trading-$STAMP.db'"
   else
      cp "$ROOT_DIR/data/trading.db" "$BACKUP_DIR/trading-$STAMP.db"
   fi
   # Rétention : 30 sauvegardes les plus récentes
   ls -1t "$BACKUP_DIR"/trading-*.db 2>/dev/null | tail -n +31 | xargs -r rm -f
   echo "💾 Base sauvegardée : data/backups/trading-$STAMP.db"
fi

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

# ─── Compilation backend ─────────────────────────────────────────────────────
# §11-4 (06/09) : libtorch/CUDA RETIRÉ du build (feature ml cuda off) —
# LIBTORCH/LIBCLANG/BINDGEN partent avec lui. Restent les libs runtime de
# l'hôte et le workaround GCC 15 (xgboost_lib-sys).
echo "🔨 Vérification backend..."
# xgboost_lib lie libxgboost.so DYNAMIQUEMENT — le chemin Python reste
# nécessaire au RUNTIME (pas seulement au build).
export XGBOOST_LIB_DIR=/home/rono/.local/lib/python3.14/site-packages/xgboost/lib
export LD_LIBRARY_PATH=$XGBOOST_LIB_DIR:/run/host/usr/lib64:$LD_LIBRARY_PATH
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
# sqlx::migrate! embarque la liste des migrations À LA COMPILATION, mais
# cargo ne recompile PAS la crate db quand seul un .sql est ajouté (incident
# 06/09 : binaire neuf sans la migration 0102 → endpoint en erreur « no such
# column »). On force la recompilation de db à chaque démarrage — quelques
# secondes, et les migrations embarquées sont toujours à jour.
touch "$ROOT_DIR/backend/crates/db/src/lib.rs"
# Le build doit RÉUSSIR — un échec silencieux ferait tourner un binaire
# PÉRIMÉ (incident du 15/08 : générateurs censés être suspendus toujours
# actifs, signaux Telegram non sollicités).
# ── Profil de l'api ──────────────────────────────────────────────────────────
# RELEASE depuis le 10/09 soir : la cause des crashes était double — le
# rattrapage ML intégral (corrigé : filtre snapshots manquants) ET la
# barrette DIMM1 fautive (memtest86+ échoué le 10/09, barrette retirée,
# 48 → 32 Go, integrity_check ok). Le passage DEBUG du 10/09 était une
# mitigation intérimaire. En cas de doute : API_PROFIL=debug bash scripts/run.sh
API_PROFIL="${API_PROFIL:-release}"

# --bin api --bin news_collector : SEULS binaux de production (les bins
# d'étude comme replay_v12 font segv le compilateur en release/LTO sous
# pression mémoire — incident 06/09 ; compilation manuelle si besoin).
# NB : cargo n'a PAS de flag --debug (profil défaut = debug, --release pour
# release) — incident 10/09 : `--debug` faisait échouer cargo SANS être
# détecté (grep sur « error » = succès), lançait le binaire périmé.
CARGO_DRAPEAU="--release"
[ "$API_PROFIL" = "debug" ] && CARGO_DRAPEAU=""
if ! cargo build -p api $CARGO_DRAPEAU --bin api --bin news_collector > "$LOG_DIR/build-backend.log" 2>&1; then
  echo "❌ ÉCHEC du build backend — arrêt (ne pas lancer un binaire périmé)."
  tail -5 "$LOG_DIR/build-backend.log"
  exit 1
fi
grep -E "Compiling|Finished" "$LOG_DIR/build-backend.log" | tail -3
if [ ! -f "$ROOT_DIR/backend/target/$API_PROFIL/api" ]; then
  echo "❌ Binaire $API_PROFIL introuvable après build — arrêt."
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

# ─── Arrêt des instances résiduelles Tauri (sessions précédentes) ────────────
# (Plus de dev server Vite depuis le 05/09 : la fenêtre sert le dist/ du
# disque, rebuild au démarrage — cf. section frontend plus bas.)
# FIX 08/09 : `pkill -x native-trading-ai` ne matchait JAMAIS — le noyau
# tronque les noms de process à 15 caractères ("native-trading-"), donc une
# fenêtre résiduelle survivait, la nouvelle instance mourait aussitôt
# (unicité GTK) et le watchdog emportait tout (« l'app ne reste pas ouverte »).
# On tue par : (1) nom tronqué exact, (2) chemin du binaire — motif sûr car
# le cmdline du présent script est "bash …/scripts/run.sh", il ne contient
# jamais "src-tauri/target" (la règle du 15/08 visait des motifs pouvant
# matcher le chemin DU SCRIPT lui-même).
pkill -x "native-trading-" 2>/dev/null || true
pkill -f "src-tauri/target/(debug|release)/native-trading-ai" 2>/dev/null || true
sleep 0.5

# ─── Démarrage backend ────────────────────────────────────────────────────────
echo "🔌 Backend API → port 8080"
DATABASE_PATH="$ROOT_DIR/data/trading.db" \
  "$ROOT_DIR/backend/target/$API_PROFIL/api" \
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
  "$ROOT_DIR/backend/target/$API_PROFIL/news_collector" \
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
  # ── Garde-fou : un serveur fantôme sur 1420 ferait échouer --strictPort
  # et le watchdog arrêterait toute l'app immédiatement (incident 05/09).
  SERVEUR_STALE_PID=$(ss -tlnp 2>/dev/null | grep ':1420' | grep -oP 'pid=\K[0-9]+' | head -1)
  if [ -n "$SERVEUR_STALE_PID" ]; then
    echo "🔄 Arrêt serveur front résiduel (pid $SERVEUR_STALE_PID, port 1420)..."
    kill "$SERVEUR_STALE_PID" 2>/dev/null || true
    for i in $(seq 1 10); do
      ss -tln 2>/dev/null | grep -q ':1420' || break
      sleep 0.3
    done
  fi

  # La fenêtre Tauri charge http://localhost:1420 (devUrl, résolu en IPv4
  # 127.0.0.1 par WebKit). Deux incidents le 05/09 : (1) le dev server Vite
  # ne bindait que [::1] dans ce sandbox → la fenêtre vivait sur son cache,
  # une semaine de changements front invisible ; (2) le retirer totalement
  # a donné « Connection refused ». Solution : build du dist à chaque
  # démarrage (même logique que le cargo build backend) + serveur `vite
  # preview` sur 127.0.0.1 EXPLICITE — déterministe, sert exactement le
  # dist construit.
  echo "🏗️  Build du frontend (dist/)..."
  if ! npm run build > "$LOG_DIR/frontend-build.log" 2>&1; then
    echo "❌ ÉCHEC du build frontend — arrêt (ne pas lancer un front périmé)."
    exit 1
  fi
  echo "📡 Serveur front (vite preview) → 127.0.0.1:1420"
  npx vite preview --port 1420 --host 127.0.0.1 --strictPort > "$LOG_DIR/vite.log" 2>&1 &
  VITE_PID=$!
  # Attendre que le serveur réponde (max 10s)
  for i in $(seq 1 20); do
    curl -sf http://127.0.0.1:1420/ > /dev/null 2>&1 && break
    sleep 0.5
  done
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
  echo "🛑 Arrêt de l'application (backend + UI + serveur front)..."
  kill $BACKEND_PID $TAURI_PID ${VITE_PID:-} $TAIL_PID 2>/dev/null
  # wait ciblé : le bare `wait` attendait AUSSI news_collector (vivante
  # par conception) → run.sh coincé à vie après un crash backend (10/09).
  wait $BACKEND_PID $TAURI_PID ${VITE_PID:-} $TAIL_PID 2>/dev/null
  echo "✅ Arrêt propre — tout est clos."
}
trap cleanup INT TERM

# ── Fermeture de la fenêtre (X) = arrêt COMPLET ──────────────────────────────
# Le process Tauri meurt quand on ferme la fenêtre ; le backend ou le serveur
# front peuvent aussi tomber seuls. On surveille les trois : la fin de L'UN
# QUELCONQUE déclenche l'arrêt propre des autres (compat bash sans wait -n).
while kill -0 "$BACKEND_PID" 2>/dev/null && { [ -z "${TAURI_PID:-}" ] || kill -0 "$TAURI_PID" 2>/dev/null; } && { [ -z "${VITE_PID:-}" ] || kill -0 "$VITE_PID" 2>/dev/null; }; do
  sleep 1
done
cleanup
