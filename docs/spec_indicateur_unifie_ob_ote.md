# Spécification — Indicateur unifié « OB Institutionnels » (OB + OTE swing + confluence dorée)

**Chantier** : remplacement des 3 couches d'affichage (OB, OTE-BOS, zone cœur) par une couche unique
rétro-ingénierée de « SMC Institutional Scalper v1.3 » (KASPER Trading).
**Phase** : B (affichage uniquement — les moteurs de signaux et le scoring ne sont PAS modifiés).
**Statut** : spécification pour relecture propriétaire — **aucun code écrit avant validation**.
**Date** : 08/10/2026.

---

## 1. Décisions acquises (validées par le propriétaire)

| # | Décision | Base |
|---|---|---|
| D1 | Détection OB **réutilisée telle quelle** (v12) — parité constatée visuellement | propriétaire, 07/10 |
| D2 | OTE = bande **61,8–78,6 %** de la **dernière jambe pivot-à-pivot** + trait 50 % | vérifié chiffré sur capture XAU M15 |
| D3 | OB **doré** ⟺ intersection OTE ∩ OB non vide (chevauchement ou inclusion) | règle propriétaire + confirmée sur 2 captures |
| D4 | Zone cœur **supprimée** (sa fonction croisement OB/OTE est absorbée par le marquage doré) | display-only vérifié dans le code |
| D5 | Pas de prototype Pine — l'app sert de prototype, boucle de parité visuelle | plan validé 08/10 |
| D6 | Phase C (moteurs/scoring) = décision séparée ultérieure avec backtest | plan validé 08/10 |

### Cas de référence chiffré (capture XAU M15 du 08/10)

Jambe baissière : pivot haut **4185** (point violet) → pivot bas **4067** (point vert), amplitude 118.

| Niveau | Formule | Valeur théorique | Observé sur capture |
|---|---|---|---|
| 50 % | 4067 + 0,5 × 118 | **4126,0** | trait cyan ~4126 ✔ exact |
| 61,8 % (borne basse OTE) | 4067 + 0,618 × 118 | 4139,924 | bord OTE ~4138 ✔ (lecture pixel) |
| 78,6 % (borne haute OTE) | 4067 + 0,786 × 118 | 4159,748 | bord OTE ~4160 ✔ |
| OB doré | 4138–4145 | ⊂ [4140, 4160] à 2 pts près | inclus, calé sur la borne 61,8 % ✔ |

Capture XAU H2 : même géométrie (jambe ~4220 → 4065, OTE ≈ [4162, 4187], OB doré 4162–4172
inclus dans le bas de l'OTE). Les deux OTE observées sont **bornées dans le temps** et vivantes.

---

## 2. Spécification fonctionnelle

### 2.1 Order Blocks — inchangés

Détection v12 actuelle (`order_blocks.rs`) : aucun paramètre, aucune logique modifiés.
La seule nouveauté est le **marquage doré** (§ 2.3), propriété d'affichage calculée par bar.

### 2.2 OTE swing (nouveau module) — règle consolidée après validation réelle (08/10 soir)

**Wave (zigzag)** : retracement de seuil, déclencheur **mèche**, seuil **PAR
TIMEFRAME** (rev. 08/10 soir, captures « Zone OTE par TF ») — un seuil unique
(% ou ATR) ne peut PAS reproduire les 7 TF à la fois (balayages : le ratio
jambe/ATR varie de 1,7 en D1 à 18 en M1) :
- **M1/M5 : 0,35 %** (plage fittée 0,30–0,40 %) — reproduit la jambe intraday
  4143,25 → 4109,91 (08/10 matin) des captures TV M1/M5 ;
- **M15/M30/H1/H4 : 0,9 %** (plage 0,85–1,00 %) — reproduit la jambe
  4184,52 → 4066,46 validée exactement ;
- **D1 : 1,5 %** (validé visuellement 09/10) / **W1 : 2,0 % PROVISOIRE abandonné**
  (décision propriétaire : TF non utilisé). La table D1 de la DB était corrompue
  (audit 08/10 : 2 130 bougies divergentes de l'agrégat M30 sur 9 actifs) ;
  **RÉPARÉE le 08/10** (backup `trading_backup_avant_reparation_d1_20261008.db`,
  re-audit 0 divergence / 26 actifs). **H1 également réparée le 09/10** :
  13 350 barres reconstruites depuis le M30 (XAU 4 078, NAS100 4 055, SP500
  4 050, XAG 1 081…), backup `trading_backup_avant_reparation_h1_20261008.db`,
  re-audit 0 divergence / 381 651 barres.
Table : `seuil_pct_par_tf(tf_sec)` dans `swing_ote.rs`.

**Jambe d'ancrage** = chaîne structurelle de la wave :
- **dernier HH** = dernier pivot haut confirmé **> pivot haut confirmé précédent** ;
- **dernier LL** = miroir sur les pivots bas ;
- les pivots internes (LH/HL) ne ré-ancrent pas — preuve : rebond XAU 4143,25
  (retracement 65 %, DANS la bande) sans changement de l'OTE cible ;
- la **fusion** (pivot dépassé par la jambe) déplace le pivot de la wave mais ne
  chaîne que si elle bat l'ancre de chaîne (sinon un LH mineur dégraderait la
  référence — bug rencontré et corrigé le 08/10).

**Extrême courant = running** : le côté le plus récent de la jambe utilise
l'extrême vivant (non confirmé) — preuve : la cible affichait fib(4184,52 ;
4066,46) dès le 07/10 soir alors que le creux n'était pivot confirmé que le
08/10 02:30 UTC. Le nouvel extrême élargit la bande immédiatement.

**Direction** : l'ancre de chaîne la plus récente détermine le côté courant —
LL plus récent ⇒ jambe baissière (origine = HH, OTE au-dessus du creux) ; HH
plus récent ⇒ jambe haussière (miroir).

**Persistance** : les rebonds internes ne suppriment PAS l'affichage —
« Invalidate OTE after first touch » est une règle de TRADING (FIRE), pas
d'affichage. La box est remplacée uniquement à un nouveau HH/LL chaîné.

**Niveaux** (H = prix du haut de jambe, L = prix du bas, A = H − L) :

| Sens | borne basse OTE (61,8 %) | borne haute OTE (78,6 %) | trait 50 % |
|---|---|---|---|
| Baissière (retracement vers le haut) | L + 0,618·A | L + 0,786·A | L + 0,5·A |
| Haussière (retracement vers le bas) | H − 0,786·A | H − 0,618·A | H − 0,5·A |

**Champs exposés** : `actif`, `dir`, `top`, `bot`, `mid`, `ts_naissance`
(= ancre la plus ancienne), `pivot_haut {prix, ts}`, `pivot_bas {prix, ts}`.

**Cas de référence vérifié en replay DB réelle** (08/10 soir, XAU M15) :
OTE bear [4139,42 ; 4159,26], mid 4125,49, ancrée 4184,52 @ 06/10 19:00 UTC /
4066,46 @ 07/10 12:45 UTC — identique à TradingView, avec 1 OB doré
[4136,26 ; 4142,04] chevauchant la bande.

### 2.3 Confluence dorée (marquage des OB)

Pour chaque OB **actif** du snapshot : `dore = (ob.bot ≤ ote.top) && (ob.top ≥ ote.bot)`
— intersection de prix non vide, **sans condition de sens ni de direction** (défaut, cf. point
ouvert n° 2). Propriété **vivante** : recalculée à chaque bar avec l'OTE courante, jamais figée à
la création de l'OB. Si aucune OTE vivante → aucun OB doré.

### 2.4 Rendu visuel

| Élément | Rendu |
|---|---|
| OB normal | couleurs actuelles (fond vert/rouge translucide + bordure) — inchangé |
| OB doré | fond **jaune/or translucide** + bordure **dorée**, remplace la couleur du sens |
| OTE vivante | bande translucide (gris-bleu/cyan), bordures fines aux 61,8/78,6 %, **trait 50 % pointillé cyan**, née à `ts_naissance`, s'étend à droite |
| Ancres | disque **violet** sur la barre pivot haut, disque **vert** sur la barre pivot bas + fines lignes horizontales **orange** aux deux niveaux d'ancre, tracées sur la durée de vie de l'OTE |

**Commande utilisateur** : un toggle unique « OB Institutionnels » (`v12Institutional`) qui affiche
l'ensemble (OB dorés inclus + OTE + ancres). Le toggle OB existant (`v12Ob`) continue de piloter
les OB seuls — dorure comprise. Les toggles `v12Ote` et `v12ZoneCoeur` **disparaissent** à la
bascule (§ 4).

### 2.5 Suppressions à la bascule (après validation de parité)

| Élément | Action | Justification vérifiée dans le code |
|---|---|---|
| Affichage OTE-BOS (`boxes` 61,8–78,6 % capturées au BOS, `expiry_bars`) | **retiré de l'affichage** ; calcul interne conservé | consommé par `scoring_v11` (poids `w_ote`, garde anti-bruit) et `scoring_bs_zones` (`in_ote` figé à la création) — Phase C décidera de son remplacement |
| Zone cœur (affichage + module) | **suppression complète** : `zone_coeur.rs` (code + tests), champs `SmcOutput.zone_coeur`, collecte API, types ts-rs, dessin + toggle frontend | display-only : aucun consommateur scoring/signals (vérifié par grep — seuls des commentaires) |

Règle maison respectée : **pas de code mort laissé en commentaire** — suppression propre et complète,
dans le même commit que la bascule.

---

## 3. Architecture technique

### 3.1 Backend (crate `smc`)

- **Nouveau fichier** `backend/crates/smc/src/v12/swing_ote.rs` :
  `pub struct SwingOteDetector` — machine wave autonome (zigzag seuil + fusion +
  chaînes HH/LL + extrêmes running, cf. § 2.2) avec `update(&BarInput) ->
  SwingOteEvent`. Aucune dépendance aux autres modules du moteur (il ne lit que
  les barres) → insérable n'importe où dans le pipeline.
- **`types.rs`** : `SwingOteZone`, `SwingOteEvent` (+ champs `pub swing_ote: SwingOteEvent`
  dans `SmcOutput`).
- **`mod.rs`** : instanciation + appel `update` dans `process_bar` (position indifférente ;
  proposé : juste après le `PivotDetector` moteur, pour la lisibilité).
- **`smc_v12_collect.rs`** : collecte de l'OTE swing finale + enrichissement de chaque `ObOut`
  avec `dore: bool` (intersection calculée au snapshot final).
- **`smc_v12_out.rs`** : `SwingOteOut`, champ `dore` sur `ObOut`.

### 3.2 API `/api/smc/v12/analyse`

- Nouvelle clé `swing_ote` (objet unique, § 2.2) ; `obs[]` enrichi de `dore`.
- À la bascule : retrait des clés `otes` (anciennes boxes OTE-BOS) et `zone_coeur`.
- Types TypeScript régénérés via ts-rs (chaîne existante), aucun champ supprimé avant la bascule.

### 3.3 Frontend

- **`useSmcV12Overlay.ts`** : nouvelles données `swing_ote` dans `donneesExt` ; nouveau flag
  `v12Institutional` lu comme les autres (`lireFlags`).
- **Nouveau dessin** (fichier dédié `smcV12OverlayDrawInstitutional.ts`, même style que les
  modules existants) : bande OTE + trait 50 % + ancres, appelé avant `dessinerObsEtFvgs`.
- **`dessinerObsEtFvgs`** : un OB `dore` est peint aux couleurs dorées (surcharge ponctuelle,
  pas de refonte de la fonction).
- **Phase transitoire** : le nouvel affichage est dessiné **à côté** de l'existant (rien n'est
  retiré) tant que la parité n'est pas validée. Bascule ensuite (§ 2.5) : retrait dessins
  OTE-BOS/zone cœur + flags `v12Ote`/`v12ZoneCoeur` + presets.
- Rappel exploitation : un changement front exige une **relance run.sh** (build du dist).

### 3.4 Isolation — ce qui n'est PAS touché en Phase B

`ote.rs` (calcul interne BOS-OTE), `scoring_v11.rs`, `scoring_bs_zones.rs`, `signals.rs`,
lifecycle, trade, ML, advisory, KDJ, straddle, rockets, MTF, presets de trading.
Aucun test existant n'est modifié — ils doivent tous rester verts à l'identique.

---

## 4. Points ouverts (défauts proposés, ajustables sans refonte)

| # | Point | Défaut proposé | Statut |
|---|---|---|---|
| 1 | Seuil de la wave | **0,9 %** (fit 0,85–1,00 % ; le « 0.45 » du panneau cible ne reproduit pas les ancres) | RÉSOLU (08/10) — `WAVE_SEUIL_PCT` paramétrable |
| 2 | Dorure sans condition de sens (OB bull dans OTE baissière possible ?) | **oui, intersection pure** | ouvert — bar replay TV |
| 3 | Comportement au toucher de la bande | **affichage persiste** (toucher = règle trading FIRE) — prouvé par le rebond 4143,25 dans la bande sans changement cible | RÉSOLU (08/10) |
| 4 | Labels `L…_N` / `S…_N` de la cible | **non reproduits** (cosmétique) | parked |
| 5 | Lignes d'ancrage orange : étendue | niveaux des 2 ancres, de l'ancre au bord droit | conforme captures |
| 6 | OTE et dorure sur les fenêtres multi-TF | calcul par TF affiché, comme les OB aujourd'hui | architecture existante |

---

## 5. Tests et validation

### 5.1 Tests unitaires (nouveaux, dans `swing_ote.rs`)

1. **Cas de référence M15** : barres synthétiques reproduisant 4185 → 4067 → l'OTE baissière
   doit donner `mid = 4126,0`, `bot = 4139,924`, `top = 4159,748` (tolérance 1e-9).
2. Jambe haussière : vérification miroir des bornes.
3. Changement de jambe : nouveau pivot low confirmé → nouvelle OTE, ancienne supprimée.
4. Premier toucher : wick dans la bande → OTE invalidée (plus exposée).
5. Confluence : overlap partiel / OB inclus dans OTE / disjonction / OTE absente → `dore` correct.
6. Aucun pivot sur plateau (strict `>` du détecteur existant) → pas d'OTE.

### 5.2 Non-régression

`cargo test -p smc` (et `cargo test --workspace`) **100 % verts sans aucune modification des
tests existants** — preuve que moteurs/scoring/signals sont intacts.

### 5.3 Validation visuelle (propriétaire)

1. **Jalon B3** (affichage parallèle) : XAU M15 et XAU H2 côte à côte app vs captures TradingView —
   critères : mêmes OB (déjà acquis), OTE dans la bande à ±0,05 % près, mêmes OB dorés, ancres
   violet/vert aux bons emplacements.
2. Échantillon élargi : autres actifs/TF au choix du propriétaire (replay TV si besoin).
3. **Jalon B4** (bascule) : vérifier que seules les 3 anciennes couches ont disparu, rien d'autre.

---

## 6. Séquencement Phase B (checkpoints propriétaire obligatoires)

| Étape | Contenu | Checkpoint |
|---|---|---|
| B1 | Module Rust `swing_ote.rs` + types + tests unitaires | ✅ validé 08/10 |
| B2 | Collecte API + `dore` sur `ObOut` | ✅ validé 08/10 |
| B3 | Dessin frontend **en parallèle** + toggle | ✅ validé 09/10 après 2 itérations (règle wave/HH-LL/running, puis seuils par TF M1→D1 — 7 TF à parité TV) + réparation de la table D1 (2 130 bougies, backup dédié) |
| B4 | Bascule : suppression affichages anciens + module zone cœur + flags + test diag | codé 09/10 — **validation visuelle finale propriétaire** (relance run.sh : seules les 3 anciennes couches doivent avoir disparu) |
| — | Commit/push | **uniquement sur ordre explicite**, à chaque jalon |

## 7. Phase C — étude C1 réalisée (09/10), suite en veille

Étude C1 (outil labo `api/src/bin/etude_confluence.rs` : replay + snapshots
OTE/zones par barre, 4 définitions, z-scores 2 proportions) :

| Définition | n | Win | SL | R moyen | z(win) |
|---|---|---|---|---|---|
| D1 entrée dans la bande OTE (M15, 30k barres) | 123 | 61,8 % | 10,6 % | +0,475 | **+1,61** |
| — sans | 1 650 | 54,3 % | 16,8 % | +0,307 | |
| D2/D3 zone dorée stricte | 2-3 | — | — | — | vide |
| D4 entrée dans l'OTE, sens de la jambe | 79 | 62,0 % | 10,1 % | +0,448 | +1,32 |
| Toutes définitions en M30 | | | | | +0,13 |

**Verdict : PAS de filtre ni poids** (même discipline que l'étude conviction :
z < 2). L'effet va dans le bon sens partout (win +7,5 pts, SL ÷1,6, R ×1,5)
mais décroît avec la fenêtre (84 %/25 → 69 %/52 → 62 %/123 — gonflement petit
échantillon) et disparaît en M30. La dorure stricte est inexploitable avec le
modèle Retest (2 cas sur 1 700 : les entrées au bord exact d'une zone dorée
sont rarissimes). **Re-test quand le vécu grossira — l'outil est prêt.**

Aperçu du reste de la Phase C (sans objet tant que C1 n'est pas concluant) :
recalibrage `w_ote`, confluence en poids vs filtre dur, modèle d'entrée
Retest vs OTE — backtest comparatif, pivot daté, reset advisory, retrain ML.
