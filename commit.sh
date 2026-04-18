git add .
git commit -m "fix(data): resolution scale XAGUSD, boucle IG et refactor architecture" -m "- fix(ui): correction compilation Vue.js SFC (balises et styles vides pour SmcSignauxBloc).
- fix(ig): désactivation forcée Lightstreamer (Error 71) suite régression git/restore sur state.rs et ig_lightstreamer/mod.rs.
- refactor(data): suppression IG mapping métaux (XAGUSD, XAUUSD...) au profit du gestionnaire cache MT5 SQLite.
- fix(api): fallback DB intégré aux endpoints (REST prix_handlers et WS ws_handlers/ig) pour garantir un flux live continu aux assets MT5.
- refactor(api): extraction ig_handlers depuis handlers.rs pour strict respect de la limite de 300 lignes métier.
- refactor(clean): suppression fichiers de sauvegarde inutiles dans /backend/data/."
git push origin HEAD
