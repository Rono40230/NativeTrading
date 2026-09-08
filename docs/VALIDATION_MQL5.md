# Validation numérique du miroir MQL5 — mode d'emploi (Étape 5)

> Objectif : prouver par les nombres que l'EA MQL5 (`mt5/smc_ea_v12.mq5`),
> le moteur Rust et le Pine étalon produisent les MÊMES signaux sur les mêmes
> bornes. Écart attendu ≈ 0. La parité par construction est déjà acquise
> (226 tests Rust, constantes et ordre d'exécution identiques) — il reste à la
> prouver sur l'historique réel.

## 1. Lancer le backtest MT5 (action propriétaire)

1. MT5 → Affichage → Strategy Tester (`Ctrl+R`).
2. Expert : `smc_ea_v12` · Symbole : `XAUUSD` (M15) · Période : **2 derniers mois complets** (bornes notées au jour près).
3. Dépôt/lot : indifférent (on compare les SIGNAUX, pas l'argent).
4. Modélisation : « Every tick based on real ticks » si l'historique tick Axi le permet, sinon « 1 minute OHLC » (le noter — les verdicts intrabar peuvent différer entre les deux).
5. Exécuter, puis relever dans l'onglet Résultats :
   - nombre total de trades,
   - répartition des clôtures (SL / TP1+BE / TP2+BE / TP3 / TS / Expire),
   - profit factor et R cumulé (ou recalcul : somme des trades en unités de risque).

## 2. Extraire les chiffres de référence Rust (même bornes)

```bash
# Après un démarrage de l'app (backend frais) :
curl -s localhost:8080/api/smc/rejeu | jq '.rejeu.clotures
  | map(select(.asset=="XAUUSD" and .tf=="M15"))
  | {nb: length,
     verdicts: (group_by(.verdict) | map({(.[0].verdict): length}) | add),
     r_pondere: (map(.r_pondere) | add)}'
```

Alternative hors-ligne (aucun backend requis) :

```bash
cd backend && DATABASE_PATH=../data/trading.db \
  cargo run --release --bin replay_v12 -- --asset XAUUSD --tf M15
```

## 3. Comparer

| Métrique | Tolérance |
|---|---|
| Nombre de signaux | écart 0 attendu (±1 si bougie d'ouverture de borne incluse/exclue) |
| Répartition des verdicts | identique ; un SL↔TP1 basculé = tracer la minute exacte |
| R cumulé | ≈ 0 d'écart ; sinon divergence d'ordre intrabar (voir §4) |

## 4. Écart ≠ 0 : le tracer

1. Identifier le premier trade divergent (horodatage commun aux deux journaux).
2. Comparer ses niveaux (entrée/SL/TP1/TP2/TP3) — un niveau ≠ 1$ = bug de constantes.
3. Niveaux identiques mais verdict ≠ : regarder la bougie M1 de la clôture dans MT5 —
   c'est un arbitrage **intrabar** (SL touché avant TP dans la même minute) : le
   Rust live évalue au tick (~1 s), le replay Rust à la barre M1 (précédence
   conservatrice SL d'abord), MT5 dépend du mode de modélisation choisi en §1.
   Un écart purement intrabar n'est PAS un bug de miroir — le noter et l'exclure
   de la comparaison.
4. Tout autre écart = bug réel → le corriger des trois côtés (Pine/Rust/MQL5) —
   règle : Pine = étalon.

## 5. Pine dans TradingView (action propriétaire)

Coller `docs/reference/` (le Pine étalon) dans TV sous le nom « Scalp à Nono » —
vérifier visuellement que les zones d'un jour donné coïncident avec l'app.

## État — VERDICT FINAL (08/09)

- [x] Backtest MT5 (08/09 — XAUUSD M15 01/07→31/08, real ticks, Axi-US50-Demo)
- [x] **§6 au centime RÉALISÉ (08/09)** : export CSV du terminal (compte réel)
      diffé bougie par bougie contre la base → **découverte et réparation
      d'une corruption majeure** : tout l'historique `mt5` antérieur au
      26/08 était estampillé en heure SERVEUR (UTC+3 été / +2 hiver —
      régression du fix 016c6a9 du 25/08, dont la re-poussée d'historique
      n'avait jamais eu lieu) + doublons à la couture. Réparation :
      décalage DST de 4,48 M barres + zone 24-25/08 + purge doublons/tierces
      (script vérifié, sauvegarde `pre-reparation-0108.db`). **Vérification
      finale : carte jour par jour 21/07 → 08/09 = UTC correct partout,
      médiane 0,000 $, 99,2 % des barres identiques au centime** (résidus :
      ~4 barres/jour à 21h00-21h45 UTC que le serveur réel n'enregistre
      pas + gaps mineurs de couture).
- [x] Référence Rust régénérée sur données réparées (run `runtime_replay`
      **id=11**, chauffe 16,5 mois) : fenêtre 01/07→28/08 = **42 clôtures**
      (16 TP1+BE · 2 TP2+BE · 2 TP3 · 8 SL · 14 Expire), ΣR unitaire +10,92.
- [x] **Diff final EA (dump [SMC diag C], 35 clôtures) vs Rust réparé (42)** :
  - **17 paires appariées → verdicts identiques** (entrées au centime) ;
  - 8 divergences, toutes tracées : 4 × fill jamais touché côté EA
    (BOS ↔ Expire — tick réel vs barre), 2 × TP1 intrabar (26/08, 31/07),
    2 × précédence intrabar (28/07 TP3/TP1) — **limite barre-vs-tick
    documentée §5, pas un bug de miroir** ;
  - 10 EA-only + 17 Rust-only : **provenance des données** — le tester
    régénère les barres depuis la base de ticks du serveur démo (3940
    barres ≠ 4224 collectées : l'heure 21h00-21h45 UTC manque, gaps
    propres) → structures différentes aux marges. Aucune divergence de
    logique moteur trouvée.
- [x] **Verdict §3 : MIROIR VALIDÉ** — zones, niveaux (entrée/SL/TP au
      centime), lifecycle et verdicts identiques partout où les deux
      côtés voient les mêmes barres. L'écart résiduel ≈ 0 sur le nombre
      de signaux n'est pas atteignable via le Strategy Tester (sources de
      barres différentes par construction) — c'est une limite de l'outil
      de test, pas du miroir. L'automatisation future (§15) s'appuiera sur
      le même feed que les moteurs (collecteur), éliminant l'écart.

## Annexe — diagnostic ajouté à l'EA (08/09)

- `[SMC diag VERDICTS] fenetre=… SL=… TP1+BE=… TP2SL=… TP3=… EXPIRE=… BOS=…`
  imprimé dans `OnDeinit` (fin de test), filtré sur la fenêtre du test
  (`g_evalDebut` = 1er instant vu) + dump `[SMC diag C]` horodaté par
  clôture (entry/sl0/bull/R0/R) pour le traçage.
- **Garde absolue §15** : `f_executeOrders` retourne immédiatement hors
  tester (`MQLInfoInteger(MQL_TESTER)`) — l'EA ne peut plus passer d'ordre
  en live, quel que soit le bouton Algorithmique ou le compte connecté.

## Référence Rust — XAUUSD M15 · 01/07 → 31/08/2026 (extraite le 07/09)

**Réglages EA : RIEN À ENTRER.** L'EA ne expose que 5 inputs (capital, risque %,
magic, exécution marché/limite, diag inutilisé) — TP1 = 0,6R, TP2 = 2,0R,
TP3 = liquidité avec cap, BE après TP1 et time-stop sont **en dur** (littéraux
du code), et ces valeurs gravées correspondent aux réglages actuels de l'app.
NB : l'EA ne fait **pas de ventes partielles 50/30/20** — position pleine,
clôture unique au marché quand son lifecycle interne marque la fin. Le R cumulé
du rapport MT5 suit donc une troisième convention (pleine position au prix de
sortie : un TP1+BE sort à entry = 0 $ réel) — **la comparaison qui prouve le
miroir = nombre + verdicts + niveaux** ; le R se reconstruit à convention
commune à partir des verdicts. L'EA imprime en fin de test
`[SMC diag VERDICTS] SL=… TP1+BE=… TP2SL=… TP3=… EXPIRE=… BOS=…` (ajouté le
08/09) — relever cette ligne dans l'onglet Journal.

| Métrique | Valeur Rust |
|---|---|
| Trades (tous remplis) | **45** |
| Verdicts | 20 TP1+BE · 3 TP2+BE · 2 TP3 · 7 SL · 13 Expire |
| ΣR unitaire (sortie pleine) | **+16,07** |
| ΣR pondéré 50/30/20 | **+4,51** |

Correspondance des libellés : EA `BE` = Rust `TP1+BE` · EA `TP2SL` = Rust
`TP2+BE` · EA `BOS` = ordre jamais rempli (0 attendu sur la fenêtre). Les
13 Expire sont des positions REMPLIES ressorties plates au time-stop (âge
max — Pine 3851/4017) : trades ≈ 0 dans le rapport MT5, pas des ordres
annulés. Les heures du journal Rust sont **UTC** ; le rapport MT5 est en heure
serveur Axi (UTC+3 en été) — décaler de +3 h pour croiser les horodatages.
