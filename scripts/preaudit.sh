#!/usr/bin/env bash
# Pré-audit bloquant — vérifie les règles absolues avant commit
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

ERREURS=0

echo "[preaudit] Vérification unwrap/expect dans les fichiers Rust modifiés..."
RUST_FILES=$(git diff --cached --name-only | grep '\.rs$' || true)
for f in $RUST_FILES; do
  [ -f "$f" ] || continue
  # Exclure src-tauri (init Tauri = expect autorisé par convention)
  [[ "$f" == *"src-tauri"* ]] && continue
  # On supprime les blocs de tests puis on cherche unwrap/expect
  HORS_TESTS=$(grep -v '^\s*//' "$f" | awk '
    /^\s*#\[.*test\]/ { in_test=1 }
    /^\s*#\[cfg\(test\)\]/ { in_test=1 }
    in_test && /^}/ { in_test=0; next }
    !in_test { print }
  ')
  if echo "$HORS_TESTS" | grep -qP '\.unwrap\(\)|\.expect\('; then
    echo "  ❌ $f contient .unwrap() ou .expect() hors tests"
    ERREURS=$((ERREURS + 1))
  fi
done

echo "[preaudit] Vérification console.log/debugger dans les fichiers TS/Vue modifiés..."
TS_FILES=$(git diff --cached --name-only | grep -E '\.(ts|vue|js)$' || true)
for f in $TS_FILES; do
  [ -f "$f" ] || continue
  if grep -nE 'console\.log|debugger|alert\(' "$f" | grep -qv '^\s*//'; then
    echo "  ❌ $f contient console.log, debugger ou alert()"
    ERREURS=$((ERREURS + 1))
  fi
done

echo "[preaudit] Vérification taille fichiers (limite 300 lignes)..."
ALL_FILES=$(git diff --cached --name-only | grep -E '\.(rs|ts|vue)$' || true)
for f in $ALL_FILES; do
  [ -f "$f" ] || continue
  NB=$(wc -l < "$f")
  if [ "$NB" -gt 300 ]; then
    echo "  ❌ $f dépasse 300 lignes ($NB lignes)"
    ERREURS=$((ERREURS + 1))
  fi
done

if [ "$ERREURS" -gt 0 ]; then
  echo ""
  echo "[preaudit] 🔴 $ERREURS erreur(s) bloquante(s) — commit annulé"
  exit 1
fi

echo "[preaudit] ✅ Pré-audit OK"
exit 0
