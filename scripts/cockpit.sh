#!/usr/bin/env bash
# ── Lecture ÉCONOME du cockpit (protocole tokens 10/09) ────────────────────
# Imprime l'état JSON du cockpit (ou une seule section) SANS le rendu HTML.
# Règle : l'agent lit le cockpit EXCLUSIVEMENT par ce script — le fichier
# complet coûte ~8-10k tokens (CSS + gabarits + rendu) dont ~97 % inutiles ;
# l'extraction en coûte ~2-3k. Sortie compacte (séparateurs serrés).
#
# Usage :
#   cockpit.sh                    → tout l'état (compact)
#   cockpit.sh feuille_de_route   → une section
#   cockpit.sh decisions          → etc. (synchro, verticales, …)
set -euo pipefail
SECTION="${1:-}"
FICHIER="$(cd "$(dirname "$0")/.." && pwd)/docs/cockpit.html"
python3 - "$FICHIER" "$SECTION" <<'EOF'
import json, re, sys
fichier, section = sys.argv[1], sys.argv[2]
src = open(fichier, encoding='utf-8').read()
m = re.search(r'<script type="application/json" id="etat-projet">\s*(\{.*?\})\s*</script>', src, re.S)
if not m:
    sys.exit("❌ bloc JSON introuvable dans le cockpit")
etat = json.loads(m.group(1))
if section:
    if section not in etat:
        sys.exit(f"❌ section inconnue : {section} (disponibles : {', '.join(etat)})")
    print(json.dumps(etat[section], ensure_ascii=False, separators=(',', ':')))
else:
    print(json.dumps(etat, ensure_ascii=False, separators=(',', ':')))
EOF
