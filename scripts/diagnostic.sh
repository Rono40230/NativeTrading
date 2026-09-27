#!/usr/bin/env bash
# ── Diagnostic santé de Native Trading AI ──────────────────────────────────
cd "$(dirname "$0")/.."
DB="data/trading.db"
API="http://localhost:8080"
OK=0; KO=0; KO_LIST=""

ok()   { OK=$((OK+1)); }
ko()   { KO=$((KO+1)); KO_LIST="$KO_LIST\n  ✗ $1: $2"; }

echo "═══ DIAGNOSTIC NATIVE TRADING AI — $(date '+%d/%m/%Y %H:%M') ═══"

# 1. Infrastructure
echo "── 1. INFRASTRUCTURE ──"
[ "$(curl -s -o /dev/null -w '%{http_code}' $API/api/strategies)" = "200" ] && ok || ko "API backend" "injoignable"
[ "$(curl -s -o /dev/null -w '%{http_code}' http://localhost:1420/)" = "200" ] && ok || ko "Frontend" "injoignable"
[ "$(ps -eo comm= | grep -c '^native-trading')" -ge 1 ] && ok || ko "Process Tauri" "absent"
[ "$(sqlite3 $DB 'SELECT 1' 2>/dev/null)" = "1" ] && ok || ko "DB" "inaccessible"

# 2. Sources de données
echo "── 2. SOURCES ──"
BOUGIES=$(sqlite3 $DB "SELECT COUNT(*) FROM bougies WHERE source='bybit_ws' AND timestamp >= strftime('%s','now','start of day')" 2>/dev/null)
[ "$BOUGIES" -ge 100 ] 2>/dev/null && ok || ko "Bybit WS" "$BOUGIES bougies/j"
SYMS=$(curl -s $API/api/mt5/statut 2>/dev/null | python3 -c "import json,sys; print(len(json.load(sys.stdin).get('symboles',[])))" 2>/dev/null)
[ "$SYMS" -ge 10 ] 2>/dev/null && ok || ko "EA MT5" "$SYMS symboles"
OLLAMA=$(curl -s $API/api/ia/status 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin).get('ollama_disponible'))" 2>/dev/null)
[ "$OLLAMA" = "True" ] && ok || ko "Ollama" "$OLLAMA"

# 3. Moteurs
echo "── 3. MOTEURS ──"
for strat in SMC straddle rockets kdj_halftrend; do
  ETAT=$(curl -s $API/api/strategies 2>/dev/null | python3 -c "import json,sys; [print(s['etat']) for s in json.load(sys.stdin) if s['id']=='$strat']" 2>/dev/null)
  if [ "$ETAT" = "Officielle" ] || [ "$ETAT" = "Observation" ]; then ok; else ko "Stratégie $strat" "$ETAT"; fi
done

# 4. Notation presse
echo "── 4. NOTATION PRESSE ──"
NOTES=$(sqlite3 $DB "SELECT COUNT(*) FROM news_sentiment WHERE analyse_le >= strftime('%s','now') - 172800" 2>/dev/null)
[ "$NOTES" -ge 5 ] 2>/dev/null && ok || ko "Notations 48h" "$NOTES (trop peu)"

# 5. Calendrier
echo "── 5. CALENDRIER ──"
CAL=$(curl -s $API/api/calendar/etat 2>/dev/null | python3 -c "import json,sys; print(json.load(sys.stdin).get('source_prete'))" 2>/dev/null)
if [ "$CAL" = "True" ]; then ok; elif [ "$CAL" = "False" ]; then ok; else ko "Calendrier" "indisponible"; fi

# 6. Positions
echo "── 6. POSITIONS ──"
for strat in SMC straddle rockets kdj_halftrend; do
  N=$(sqlite3 $DB "SELECT COUNT(*) FROM signaux WHERE strategie='$strat' AND statut='Actif'" 2>/dev/null)
  echo "  $strat : $N actif(s)"
  ok
done

# Résumé
echo
echo "═══ RÉSUMÉ : $OK OK · $KO problème(s) ═══"
if [ $KO -gt 0 ]; then
  echo -e "$KO_LIST"
fi
echo
