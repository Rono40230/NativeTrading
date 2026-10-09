# ÉTAT D'AVANCEMENT DU PROJET — Native Trading AI

> **Rôle de ce fichier** : point d'entrée unique sur l'état du projet (créé le 09/10/2026
> à la demande du propriétaire). Il centralise l'avancement, les tâches en cours et les
> décisions en attente. Les détails vivent dans les documents référencés en § 7.
> **Dernière mise à jour : 09/10/2026 midi — HEAD `2236035` + chantier « réglages par asset » P1-P2 validées (non commité, § 1).**

---

## 1. POINT CLAIR — où en est le projet

**L'application est complète et en production sur ses 4 stratégies.** La phase intensive
de construction est terminée ; le projet est en régime de **mesure** : les moteurs
tournent, le vécu s'accumule, et les prochaines décisions de réglage attendent des
seuils statistiques (règle des 30 trades / AUC 0,65).

| Volet | Statut |
|---|---|
| Moteurs SMC · Straddle · Rockets · KDJ | ✅ 4/4 en production (Officielle), Telegram actif |
| Qualité des données | ✅ assainie (M30/H1/D1 réparées, EA v1.35 + upsert anti-récidive) |
| Indicateur unifié OB Institutionnels | ✅ livré (7 TF à parité TradingView) |
| Exploitation | ✅ hot-reload des paramètres (60 s), advisory balayages, télémétrie santé |
| Dettes techniques | ✅ purgées (audit 15/15 du 06/10, ~1 100 lignes mortes, base 1,44 Go) |
| **En attente active** | ⏳ 4 mesures statistiques qui mûrissent (§ 3) |
| **En attente du propriétaire** | 🔑 3 décisions d'ouverture (§ 4) |
| **✅ Réglages PAR ASSET — chantier terminé (09/10)** | 3 phases validées par le propriétaire : surcharge ⊕ défaut (tables 0121) + moteurs/hot-reload par asset + modales de réglage (P1) ; labo/advisory/activation par asset, règle des 30 par asset (P2) ; conseiller IA 🤖 par asset du classement des assets, chiffres clés expliqués un à un (P3). **Poussé `b42cae7`.** Spec `docs/spec_reglages_par_asset.md` |
| **✅ Alertes d'approche des zones SMC — livrées (09/10)** | Watcher sur prix live : zones OB **fraîches** des couples ARMÉS (suivent l'armement — M5/M15/M30 aujourd'hui), seuil **0,25 × ATR du TF**, alerte unique par zone/épisode (ré-armement hystérésis 1×ATR), **Telegram + bandeau Scanner**, nuit 23h-7h Paris (ne rien faire), purement mémoire, anti-flood 20/h. Fix critique inclus : zones lues sur l'évaluation **LIVE intrabar** (le commité ne voit jamais une zone Vierge sur XAU M15). Spec `docs/spec_alertes_zones_smc.md` |

---

## 2. CHIFFRES DU JOUR (base réelle, 09/10 soir)

| Stratégie | Clôturés | WR ($>0) | Σ R encaissé | Remarque |
|---|---|---|---|---|
| SMC | 644 | 44 % | **+109,2** | M5 porteur (+62,2 sur 275) · M15 +2,6 · M30 +1,8 · production en tout-TP1 + trailing 0,1R |
| Straddle | 302 | 26 % | **+31,3** | moteur événementiel tout-armé (81 cases) depuis le 28/09 — 151 passes en octobre (+1,9) ; profil changé, à juger par source à seuil 30 |
| Rockets | 9 | 100 % | **+4,9** | scanner accumule (règle des 30) |
| KDJ | 3 émis | — | — | **premiers signaux de l'histoire** (06/10 : BNB, SOL, XPTUSD) après le fix de chauffe — 0 fermé, 3 en cours |

En cours au relevé : 7 positions SMC + 3 KDJ. Conventions : R = encaissé (distance en
étude uniquement), $ réels composés, jamais de R moyen affiché.

---

## 3. Tâches EN COURS / PASSIVES — mûrissement statistique

| # | Tâche | Jalon | Où on en est (09/10) | Prochain geste |
|---|---|---|---|---|
| A | **Re-test final conviction LLM** | ~500 clôturés SMC notés | **464/500 — IMMINENT** (AUC précédents : 0,572 puis 0,517, seuil 0,65) | lancer le re-test protocolaire (~1 semaine de vécu en plus suffit) |
| B | KDJ — mesure 30 trades H1 | 30 clôturés | 3 signaux (0 fermé) ; rythme ~1/sem/actif × 4 assets | attendre ; accélérable en ajoutant des assets (labo 7.G : XRP +1,35 R/tr en tête) |
| C | Straddle — jugement par source | ≥ 30 passes par source | 151 passes en octobre ; à re-découper par type d'événement (matrice 81 cases) | requête par source quand les seuils tombent ; DAX historiquement le traînard |
| D | Rockets — analyse par pilier | ≥ 30 clôturés | 9 clôturés (100 %) | laisser le scanner accumuler |
| E | Whale watching — labo avec/sans | règle des 30 | différé (livré 27/09, enrichit sans filtrer) | labo quand l'effectif le permet |
| F | Confluence dorée (OTE swing) — re-test | vécu accru | C1 du 09/10 : effet réel non concluant (z=+1,61), outil `etude_confluence` prêt | re-jouer périodiquement (1 commande) |
| G | Réparation résiduelle éventuelle | — | tables H1/D1/M30 réparées ; surveiller la fraîcheur `/api/sante/sources` | rien d'ouvert |

## 4. Décisions EN ATTENTE DU PROPRIÉTAIRE

| # | Décision | Contexte |
|---|---|---|
| D1 | **Exécution réelle (EA exécutant)** — 5.1 | feu vert de principe : EA recevant lots/niveaux, validation humaine par trade, kill-switch. Cahier des charges prêt (5.2) |
| D2 | **Score réévalué au retest** — 2.5 | étude A/B par levier replay (score cliquet vs confluence live) ; n'a priorité que si 2.1/2.2 had concluded — contexte inchangé |
| D3 | **Rôle futur du cockpit.html** | le cockpit (source de vérité historique) est arrêté au 27/09 : soit on le resynchronise, soit ce fichier devient la référence unique et le cockpit passe en archive |

---

## 5. État par verticale

### SMC (Officielle)
Moteur v12 figé (miroir Pine/Rust/MQL5 au centime). 26 actifs ; M1 désarmé, M5 > M15 > M30.
Production actuelle : **tout-TP1 + trailing 0,1R** (essai activé via l'advisory balayages
le 06/10 — +154 R / 959 trades en étude). Conviction LLM à l'émission (recalibrée, non
filtrante). Indicateur graphique unifié « OB Institutionnels » (08-09/10) : OTE swing par
TF + confluence dorée, zone cœur supprimée — affichage seul, aucun effet moteur (C1).
Fenêtre macro ±30 min autour des annonces High (straddle exempt).

### Straddle (Officielle)
Moteur événementiel (28/09) : taxonomie DST + matrice 81 cases TOUT ARMÉ — remplace les
créneaux statistiques, pipeline IA supprimé. Minuteur interne à l'horloge exacte (étape 15).
Périmètre = table configuration (9 assets). Hot-reload des params avec garde « passe en
fenêtre active différée » (09/10).

### Rockets (Officielle)
Scanner D1 /10 (crypto + actions US), gestion 30 s, sortie de stagnation (STAG), veto
unlocks actif. Étape 13 classée : D1 ≡ M15 (backtest, équivalence parfaite — pas de
bascule). Analyse par pilier en attente d'effectif.

### KDJ/Halftrend (Officielle)
Moteur H1 déterministe, chauffe 600 barres au montage (fix du 05/10 — avant, muet 4 j
après chaque redémarrage). Scanner tableau + params hot-reload. **Premiers signaux le
06/10**. Calibration par actif disponible (écart ~40× entre actifs).

---

## 6. Chronologie des chantiers clos (résumé)

| Période | Chantier | Clôture |
|---|---|---|
| Sept. (voir cockpit) | audit app, trailing prod, vécu immuable, refontes dashboard/cockpit, whale watching, straddle événementiel… | 0de9b0c (27/09) |
| 05-06/10 | **Audit roadmap 15/15** : KDJ chauffé, préchauffage boot, seuils fantômes, STAG rockets, télémétrie silence, ML mono-régime, purges (~1 100 lignes, 71→17 Go), advisory balayages (SMC tout-TP1 activé), rétention base (1,44 Go), minuteur straddle | 04d3b80 |
| 07/10 | **Bougies tronquées MT5** : course de lecture EA + INSERT OR IGNORE → upsert + EA v1.35 surveilleur + 1 559 M30 réparées | ad2b225 |
| 08-09/10 | **Indicateur unifié OB Institutionnels** (B1→B4) + réparations D1 (2 130) et H1 (13 350) + étude C1 (pas de filtre) | e6f2c42, a13baab |
| 09/10 soir | **Tri des résidus** + **hot-reload paramètres moteur** | 297e7d5, 2236035 |

## 7. Cartographie des documents

| Document | Rôle |
|---|---|
| **docs/ETAT-AVANCEMENT.md** (ce fichier) | point d'entrée unique — avancement, tâches, décisions |
| docs/cockpit.html | historique vivant des décisions + règles (à jour au 27/09 — voir D3) |
| docs/spec_indicateur_unifie_ob_ote.md | spec complète de l'indicateur unifié (règles, preuves, B1→B4, C1) |
| docs/audit-app-2026-09.md | audit de septembre — historique (P0-P3 traités par l'audit sept.) |
| docs/VALIDATION_MQL5.md, VALIDATION_KDJ_HALFTREND.md, protocole_screening_kdj_halftrend.md | archives de validation figées |
| docs/reference/ | étalons : Pine v12, écarts votés, définitions KDJ/rockets |

## 8. Règles du projet (condensé — la liste complète vit au cockpit)

1. Décrire avant de corriger ; checkpoint vérifié avant l'étape suivante ; mesure avant
   toute décision de réglage (≥ 30 trades par tranche, AUC 0,65 pour les filtres).
2. Vécu immuable : niveaux figés à l'émission, fractions figées à la clôture — les
   contre-factuels vivent au labo uniquement.
3. Pine = étalon (écarts votés seulement) ; l'IA n'exécute jamais ; Telegram = imminence.
4. Zéro dette en sortie de chantier ; 600 lignes/fichier ; pré-audit bloquant au commit.
5. **Commit/push uniquement à l'ordre explicite du propriétaire.**
6. R en deux voix : distance (étude) / encaissé (affichage) ; jamais de R moyen affiché.
