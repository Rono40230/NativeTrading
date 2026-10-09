# Spécification — Alertes d'approche des zones SMC

**Objet** : prévenir le propriétaire lorsqu'une zone d'achat/vente SMC (order block
actif) est **sur le point d'être touchée** par le prix live.
**Décisions propriétaire (09/10)** : ① zones = **OB du moteur uniquement** ;
② seuil = **0,25 × ATR** du TF du couple ; ③ zones **fraîches (jamais touchées)**
seulement ; ④ canaux = **app + Telegram, arrêt la nuit de 23h à 7h** (heure de
Paris) ; ⑤ mémoire **purement en mémoire** (aucune table, re-scan au redémarrage).
**Statut** : spec validée 09/10 (+ précision propriétaire : périmètre = assets
choisis/ARMÉS SMC uniquement — garanti par construction, seuls les couples
armés possèdent des moteurs). **CODÉE 09/10 + FIX SOIR** : le watcher lisait
les zones dans l'état COMMITÉ — or le diagnostic (`diag_approche`, replay
XAU M15) a montré qu'une OB y naît et se touche DANS la même barre (28/29
zones du jour) → jamais vue Vierge → jamais d'alerte. Correctif : les zones
sont lues sur l'évaluation LIVE (clone intrabar du `on_tick`, même discipline
que les annonces d'imminence 23/08) — cache `zones_cache` rafraîchi à chaque
tick/clôture, test `zone_approche_visible_intrabar_avant_toucher`.
**Date** : 09/10/2026.

---

## 1. Principe

Le runtime voit déjà chaque prix live (WS Bybit pour les cryptos, EA MT5 à la
seconde pour les autres) — même flux que le watcher `alertes_prix`. Les moteurs
SMC armés détiennent déjà les zones : `ObDetector.bull_zones()/bear_zones()`
(FIFO 40 par sens, états Vierge/Partiel/Profond), ATR14 par couple. Le nouveau
watcher mesure, à chaque prix live, la distance au **bord proche** de la zone
fraîche la plus proche :

- zone ACHAT (OB bull SOUS le prix) : bord proche = `top` ; approche si
  `prix − top ∈ ]0 ; 0,25 × ATR]` ;
- zone VENTE (OB bear AU-DESSUS du prix) : bord proche = `bot` ; approche si
  `bot − prix ∈ ]0 ; 0,25 × ATR]` ;
- prix déjà DANS la zone (distance ≤ 0) : pas d'alerte d'approche — trop tard,
  la clôture marquera la zone touchée et elle quittera le périmètre « fraîche ».

**Une seule zone éligible par couple × sens** : la zone fraîche la plus PROCHE
du prix (anti-bruit : un asset en congestion ne peut pas mitrailler). Une
alerte par zone et par épisode d'approche.

## 2. Machine à états (en mémoire)

Clé de zone : `(asset, tf, sens, timestamp_zone)`.

| État | Signifié | Transition |
|---|---|---|
| Armée | éligible, prix hors bande | prix entre dans la bande → **Alertée** (+ notification hors nuit) |
| Alertée | bande déjà signalée | prix s'éloigne au-delà de `seuil + 1,0 × ATR` (hystérésis) → Armée (ré-armable) |

Suppression de la clé : la zone disparaît de la liste « fraîche » du moteur
(touchée → Partiel/Profond, invalidée, ou évincée FIFO) ou le couple est désarmé.

**Nuit (23h–7h heure locale serveur = Paris)** : `verifier()` ne fait RIEN la
nuit — aucun changement d'état, aucune notification. Au réveil (7h), l'état est
celui de 23h : une zone encore en bande alerte alors ; une zone touchée ou morte
dans la nuit n'alerte jamais. Pas de rattrapage bruyant.

**Garde-fou anti-flood** : plafond 20 alertes/heure glissante (les alertes
au-delà sont journalisées en log, pas notifiées).

## 3. Architecture

1. **Trait `Engine`** : nouvelle méthode à défaut vide
   `fn zones_approche(&self) -> Vec<ZoneApproche>` ; `ZoneApproche { sens, top,
   bot, timestamp_zone, atr }` définie dans le crate engine.
   Seul `MoteurV12` (engine_v12) l'implémente : zones `Vierge` uniquement,
   ATR = `atr.value()` du moteur (source unique — jamais recalculée ailleurs).
2. **Runtime** : accès lecture `zones_approche(asset, tf)` pour le couple.
3. **Watcher `alertes_zones.rs`** (api) : appelé dans la boucle
   `demarrer_runtime_tick`, à côté de `alertes_prix::verifier`, sur CHAQUE
   `EvenementPrix` ; config rechargée dans le même tick 60 s.
4. **Notifications** : Telegram via `notifications::telegram` (même chemin que
   les alertes prix — imminence seule, conforme à la règle) ; face app par état
   mémoire + polling (même discipline que `setups_formation`).
5. **API** : `GET /api/smc/zones-approche` → zones actuellement en bande
   (asset, TF, sens, zone, distance × ATR et %, direction du prix) + alertes
   récentes (2 h). Aucune écriture DB.

Message Telegram (court, imminence) :
```
📍 Approche zone ACHAT
XAUUSD M5 · [2334,20 ; 2331,80] · à 0,12 × ATR
```

## 4. Configuration (clés `configuration`, rechargées au tick 60 s)

| Clé | Défaut | Rôle |
|---|---|---|
| `smc_alertes_zones_actif` | 1 | interrupteur global |
| `smc_alertes_zones_seuil_atr` | 0.25 | bande d'approche |
| `smc_alertes_zones_hysteresis_atr` | 1.0 | ré-armement (recul au-delà de seuil + cette valeur) |
| `smc_alertes_zones_nuit_debut` | 23 | début du silence (heure locale) |
| `smc_alertes_zones_nuit_fin` | 7 | fin du silence |
| `smc_alertes_zones_telegram` | 1 | envoi Telegram on/off (face app toujours active) |

## 5. Face app

Carte « Zones à l'approche » sur la page **Scanner SMC** (polling 30 s) :
une ligne par zone en bande — asset, TF, badge ACHAT/VENTE, zone [haut;bas],
distance live (× ATR et %). Les alertes fraîches passent aussi par le fil
d'alertes existant (notification OS + son, dédup par clé côté front).

## 6. Tests et garde-fous

- Distances : cas bull/bear, dans-la-zone (pas d'alerte), bord exact.
- Machine à états : armée → alertée → ré-armement par hystérésis ; suppression
  quand la zone quitte l'état Vierge ; nuit = aucun changement d'état, alerte
  au réveil si encore en bande.
- Plafond anti-flood respecté.
- `etat_nuit(heure)` : bornes 23h et 7h (23h00 inclus, 7h00 exclus).
- Non-régression : watch désactivé (`actif=0`) ⇒ comportement identique à
  aujourd'hui (aucune notification, aucun appel).

## 7. Hors périmètre (explicitement)

- OTE de l'indicateur unifié et liquidités (PDH/PDL, EQH/EQL) — extension
  future si demandée.
- Historique/mesure des alertes en base (décision ⑤ : purement mémoire).
- Mute pendant la fenêtre macro ±30 min (les alertes d'approche ne sont pas des
  signaux ; à rediscuter si le bruit autour des annonces High gêne).
- Granularité par TF/asset : suit l'armement, comme le scanner.

**Checkpoint (une phase)** : prix live approchant une zone fraîche → Telegram +
carte Scanner ; zone touchée → la ligne disparaît ; nuit → silence ; réveil 7h →
alerte si encore en bande ; `actif=0` → plus rien.
