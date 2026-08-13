#!/usr/bin/env bash
# Audit complet pré-push — vérifications qualité avant publication
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

ERREURS=0

echo "[audit] Vérification unwrap/expect dans tout le crate backend..."
while IFS= read -r -d '' f; do
  # Exclure src-tauri, binaires utilitaires et fichiers de test dédiés
  [[ "$f" == *"src-tauri"* ]] && continue
  [[ "$f" == */bin/* ]] && continue
  [[ "$f" == */tests/* ]] && continue
  HORS_TESTS=$(grep -v '^\s*//' "$f" | awk '
    /^\s*#\[.*test\]/ { in_test=1 }
    /^\s*#\[cfg\(test\)\]/ { in_test=1 }
    in_test && /^}/ { in_test=0; next }
    !in_test { print }
  ')
  # Exclure les patterns d'init critique légitimes (client HTTP, timezone, Tauri)
  VIOLATIONS=$(echo "$HORS_TESTS" | grep -P '\.unwrap\(\)|\.expect\(' | grep -vP 'client HTTP|offset valide|Création client|Tauri|runtime|Builder' || true)
  if [ -n "$VIOLATIONS" ]; then
    echo "  ❌ $f contient .unwrap() ou .expect() hors tests"
    ERREURS=$((ERREURS + 1))
  fi
done < <(find backend/crates -name '*.rs' -print0 2>/dev/null)

echo "[audit] Vérification console.log/debugger dans le frontend..."
while IFS= read -r -d '' f; do
  if grep -qP 'console\.log|debugger|alert\(' "$f" 2>/dev/null; then
    if ! grep -P 'console\.log|debugger|alert\(' "$f" | grep -qP '^\s*//'; then
      echo "  ❌ $f contient console.log, debugger ou alert()"
      ERREURS=$((ERREURS + 1))
    fi
  fi
done < <(find frontend/src -name '*.ts' -o -name '*.vue' -o -name '*.js' -print0 2>/dev/null)

echo "[audit] Vérification taille fichiers Rust (limite 600 lignes)..."
while IFS= read -r -d '' f; do
  NB=$(wc -l < "$f")
  if [ "$NB" -gt 600 ]; then
    echo "  ❌ $f dépasse 600 lignes ($NB lignes)"
    ERREURS=$((ERREURS + 1))
  fi
done < <(find backend/crates -name '*.rs' -print0 2>/dev/null)

echo "[audit] Vérification taille fichiers Vue/TS (limite 600 lignes)..."
while IFS= read -r -d '' f; do
  NB=$(wc -l < "$f")
  if [ "$NB" -gt 600 ]; then
    echo "  ⚠️  $f dépasse 600 lignes ($NB lignes) — attention"
  fi
done < <(find frontend/src -name '*.vue' -o -name '*.ts' -print0 2>/dev/null)

if [ "$ERREURS" -gt 0 ]; then
  echo ""
  echo "[audit] 🔴 $ERREURS erreur(s) bloquante(s) — push annulé"
  exit 1
fi

echo "[audit] ✅ Audit OK — push autorisé"
exit 0
