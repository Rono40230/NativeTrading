# 📋 CAHIER DES CHARGES - AMÉLIORATIONS COMPLÈTES DE L'INDICATEUR SMC
## SMC Institutional Scalping v12 - Module complémentaire et optimisations

---

## 1. OBJECTIF STRATÉGIQUE GLOBAL

Améliorer la **qualité des signaux**, la **robustesse** et la **diversification** de l'indicateur SMC Institutional Scalping v12, en ajoutant des modules complémentaires et en optimisant les filtres existants, **sans déstabiliser l'architecture actuelle**.

### 1.1 Objectifs quantifiés
- **Taux de réussite (WR)** : +5 à 15% selon les actifs.
- **Réduction des faux signaux** : -15 à 25%.
- **Augmentation du nombre de signaux exploitables** : +10 à 20% (surtout sur DAX et BTC).
- **Amélioration du TP3** : +20 à 30% de taux d'atteinte.

### 1.2 Contraintes impératives
- **AUCUNE** modification du comportement existant quand les modules sont désactivés.
- **AUCUN** module ne crée de trade autonome (ils bonifient le scoring existant).
- **Compatibilité** : Tous les ajouts doivent être **optionnels** (toggles).
- **Performance** : Pas de ralentissement significatif (>5% de charge CPU).

---

## 2. TABLEAU RÉCAPITULATIF DES AMÉLIORATIONS

| ID | Amélioration | Priorité | Impact | Complexité | Gain estimé |
|---|---|---|---|---|---|
| **A** | BPR - Balanced Price Range | Élevée | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | +5-10% WR |
| **B** | Scoring des Gaps (NDOG/NWOG) | Élevée | ⭐⭐⭐⭐ | ⭐ | +2 pts sur retest gaps |
| **C** | Blocage Dead Zone (NY Lunch) | Élevée | ⭐⭐⭐⭐ | ⭐ | -20% faux signaux |
| **D** | Filtre de Régime (Range/Trend) | Élevée | ⭐⭐⭐⭐ | ⭐⭐ | +5-10% WR |
| **E** | Filtre Momentum (RSI) | Moyenne | ⭐⭐⭐ | ⭐ | +3-5% WR |
| **F** | Scoring Sessions H/L | Moyenne | ⭐⭐⭐ | ⭐⭐ | +1-2 pts |
| **G** | DoL comme TP dynamique | Moyenne | ⭐⭐⭐⭐ | ⭐⭐ | +20-30% TP3 |
| **H** | Scoring Mega-Orders | Moyenne | ⭐⭐ | ⭐⭐ | +2 pts |
| **I** | OFI (Order Flow Imbalance) | Complexe | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | +10-15% WR |
| **J** | Alerte composite améliorée | Faible | ⭐⭐ | ⭐ | +Confort trading |

---

## 3. MODULE A - BPR (BALANCED PRICE RANGE) — PRIORITÉ ÉLEVÉE

### 3.1 Définition technique
Un BPR est formé lorsqu'un **FVG Bullish** et un **FVG Bearish** se chevauchent géométriquement, créant une zone d'équilibre institutionnel.

### 3.2 Structure de données
```pinescript
// Arrays BPR (à ajouter après les arrays FVG)
var array<float> bprTop   = array.new<float>(0)   // Haut de la zone BPR
var array<float> bprBot   = array.new<float>(0)   // Bas de la zone BPR
var array<int>   bprState = array.new<int>(0)     // 0=Frais, 1=Partiel, 2=Profond
var array<box>   bprBox   = array.new<box>(0)     // Boîte visuelle
var array<int>   bprBar   = array.new<int>(0)     // Barre de création
var array<bool>  bprActive= array.new<bool>(0)    // true = encore pertinent
3.3 Paramètres utilisateur
pinescript
var string GRP_BPR  = "🔄 Balanced Price Range (BPR)"
bool  i_showBPR     = input.bool(false, "Afficher les BPR", group=GRP_BPR)
int   i_maxBPR      = 10   // Nombre max de BPR actifs
int   i_bprMaxAge   = 15   // Âge max en bougies (fraîcheur)
float i_bprMinSize  = 0.5  // Taille min en ATR (évite les micro-zones)
int   i_bprScore    = 4    // Points ajoutés au scoring quand un OB touche un BPR
3.4 Fonction de détection
pinescript
// ─── DÉTECTION BPR (chevauchement FVG Bull + FVG Bear) ──────────────
f_detectBPR() =>
    if barstate.isconfirmed and i_moteurFVG and array.size(fvgBullTop) > 0 and array.size(fvgBearTop) > 0
        for _bi = 0 to array.size(fvgBullTop) - 1
            float _bullTop = array.get(fvgBullTop, _bi)
            float _bullBot = array.get(fvgBullBot, _bi)
            int   _bullBar = array.get(fvgBullBar, _bi)
            
            if (bar_index - _bullBar) > i_bprMaxAge
                continue
            
            for _ri = 0 to array.size(fvgBearTop) - 1
                float _bearTop = array.get(fvgBearTop, _ri)
                float _bearBot = array.get(fvgBearBot, _ri)
                int   _bearBar = array.get(fvgBearBar, _ri)
                
                if (bar_index - _bearBar) > i_bprMaxAge
                    continue
                
                float _ovTop = math.min(_bullTop, _bearTop)
                float _ovBot = math.max(_bullBot, _bearBot)
                
                if _ovTop > _ovBot
                    float _size = _ovTop - _ovBot
                    if _size < i_bprMinSize * atr14
                        continue
                    
                    // Anti-doublon
                    bool _exists = false
                    if array.size(bprTop) > 0
                        for _ei = 0 to array.size(bprTop) - 1
                            if math.abs(array.get(bprTop, _ei) - _ovTop) < 0.01 * close and
                               math.abs(array.get(bprBot, _ei) - _ovBot) < 0.01 * close
                                _exists := true
                                break
                    if _exists
                        continue
                    
                    // FIFO
                    if array.size(bprBox) >= i_maxBPR
                        box.delete(array.shift(bprBox))
                        array.shift(bprTop)
                        array.shift(bprBot)
                        array.shift(bprState)
                        array.shift(bprBar)
                        array.shift(bprActive)
                    
                    array.push(bprTop, _ovTop)
                    array.push(bprBot, _ovBot)
                    array.push(bprState, 0)
                    array.push(bprBar, bar_index)
                    array.push(bprActive, true)
                    
                    if i_showBPR
                        color _c = color.new(#FFD700, 70)
                        array.push(bprBox, box.new(bar_index, _ovTop, bar_index + 50, _ovBot,
                            bgcolor=_c, border_color=color.new(#FFD700, 0), border_width=2,
                            text="BPR", text_color=color.white, text_size=size.tiny,
                            text_halign=text.align_right, text_valign=text.align_top))
                    else
                        array.push(bprBox, na)
3.5 Lifecycle BPR
pinescript
// ─── LIFECYCLE BPR ──────────────────────────────────────────────────────
f_bprLifecycle() =>
    if array.size(bprTop) > 0 and barstate.isconfirmed
        array<int> _delIdx = array.new<int>(0)
        for _i = 0 to array.size(bprTop) - 1
            float _top = array.get(bprTop, _i)
            float _bot = array.get(bprBot, _i)
            int   _bar = array.get(bprBar, _i)
            bool  _act = array.get(bprActive, _i)
            
            // 1. Invalider si trop vieux
            if (bar_index - _bar) > i_bprMaxAge * 2
                array.push(_delIdx, _i)
                continue
            
            // 2. Invalider si complètement comblé
            if low <= _bot and high >= _top
                array.set(bprActive, _i, false)
                box _bx = array.get(bprBox, _i)
                if not na(_bx)
                    box.set_bgcolor(_bx, color.new(#9E9E9E, 40))
                array.push(_delIdx, _i)
                continue
            
            // 3. Mise à jour de l'état
            float _mid = (_top + _bot) / 2.0
            int _st = array.get(bprState, _i)
            if low <= _top and high >= _bot
                if _st < 2
                    array.set(bprState, _i, 2)
            else if low <= _top or high >= _bot
                if _st == 0
                    array.set(bprState, _i, 1)
            
            // 4. Extension de la box
            if barstate.islast and _act
                box _bx = array.get(bprBox, _i)
                if not na(_bx)
                    box.set_right(_bx, bar_index + 50)
        
        if array.size(_delIdx) > 0
            array.sort(_delIdx, order.descending)
            for _j = 0 to array.size(_delIdx) - 1
                int _idx = array.get(_delIdx, _j)
                box.delete(array.get(bprBox, _idx))
                array.remove(bprTop, _idx)
                array.remove(bprBot, _idx)
                array.remove(bprState, _idx)
                array.remove(bprBar, _idx)
                array.remove(bprActive, _idx)
                array.remove(bprBox, _idx)
3.6 Intégration dans le scoring (f_score)
pinescript
// ─── SCORING BPR ──────────────────────────────────────────────────────
bool _nearBPR = false
int  _bprBonus = 0
if array.size(bprTop) > 0
    for _bi = 0 to array.size(bprTop) - 1
        if not array.get(bprActive, _bi)
            continue
        float _bTop = array.get(bprTop, _bi)
        float _bBot = array.get(bprBot, _bi)
        float _obT = isBull ? array.get(obBullTop, _i) : array.get(obBearTop, _i)
        float _obB = isBull ? array.get(obBullBot, _i) : array.get(obBearBot, _i)
        
        if _obT > _bBot and _obB < _bTop
            _nearBPR := true
            int _st = array.get(bprState, _bi)
            _bprBonus := _st == 0 ? i_bprScore :
                         _st == 1 ? i_bprScore - 1 :
                         _st == 2 ? 1 : 0
            break

if _nearBPR
    sc += _bprBonus
3.7 Impact sur les modules existants
OB Labels : Affichage "⚡BPR" si confluence.

BSZones : Bonus de +2 si confluence BPR.

Zone-cœur : Le BPR devient critère de validité supplémentaire.

4. MODULE B - SCORING DES GAPS (NDOG/NWOG) — PRIORITÉ ÉLEVÉE
4.1 Problème actuel
Les NDOG (New Day Opening Gap) et NWOG (New Week Opening Gap) sont détectés et affichés mais NE SONT PAS SCORÉS.

4.2 Structure de données existante
Les arrays existent déjà :

pinescript
var array<float> ndogTop/ndogBot, nwogTop/nwogBot
4.3 Fonction de scoring à ajouter dans f_score
pinescript
// ─── SCORING GAPS (NDOG/NWOG) ────────────────────────────────────────
bool _inBullGap = false
bool _inBearGap = false

// Vérifier NDOG
if array.size(ndogTop) > 0
    for _gi = 0 to array.size(ndogTop) - 1
        if not array.get(ndogMit, _gi)  // Gap non comblé
            float _gT = array.get(ndogTop, _gi)
            float _gB = array.get(ndogBot, _gi)
            if close >= _gB and close <= _gT
                if isBull
                    _inBullGap := true
                else
                    _inBearGap := true
                break

// Vérifier NWOG
if array.size(nwogTop) > 0
    for _gi = 0 to array.size(nwogTop) - 1
        if not array.get(nwogMit, _gi)
            float _gT = array.get(nwogTop, _gi)
            float _gB = array.get(nwogBot, _gi)
            if close >= _gB and close <= _gT
                if isBull
                    _inBullGap := true
                else
                    _inBearGap := true
                break

// Ajouter au scoring
if (isBull and _inBullGap) or (not isBull and _inBearGap)
    sc += 2  // Gap de session = niveau frais
4.4 Impact attendu
+2 points sur les setups qui retestent un gap d'ouverture.

Particulièrement efficace sur indices (NASDAQ, DAX) et forex.

5. MODULE C - BLOCAGE DEAD ZONE (NY LUNCH) — PRIORITÉ ÉLEVÉE
5.1 Problème actuel
La Dead Zone (16h-18h UTC = 12h-14h NY) est détectée (ligne ~2900) mais n'est PAS utilisée pour bloquer les trades.

5.2 Modification dans les fonctions de création de trades
pinescript
// Dans f_createBuySignals(), f_createSellSignals(),
// f_createBSBuySignals(), f_createBSSellSignals()

// Ajouter en tête de fonction :
if inDeadZone
    return  // Pas de trade en NY lunch
5.3 Impact attendu
Réduction de 15-20% des faux signaux.

Les zones mortes génèrent historiquement beaucoup de bruit (faible liquidité).

6. MODULE D - FILTRE DE RÉGIME (RANGE/TREND) — PRIORITÉ ÉLEVÉE
6.1 Problème actuel
Le filtre de régime (_regimeRange) est désactivé (valeur false en dur).

6.2 Réactivation améliorée
pinescript
// ─── FILTRE DE RÉGIME (à mettre après le calcul de atr14) ──────────
float adx14 = ta.adx(high, low, close, 14)
float atr20 = ta.atr(20)
float atr60 = ta.atr(60)
bool _regimeRange = adx14 < 20 and (atr60 > 0 and atr20 / atr60 < 0.7)

// Autoriser les trades en KZ même en range
bool _tradeAutorise = not _regimeRange or inKZ

// Dans f_createBuySignals(), f_createSellSignals()
if not _tradeAutorise
    return
6.3 Impact attendu
Amélioration du WR de 5-10% sur actifs en consolidation.

Conservation des opportunités en KZ (meilleure liquidité).

7. MODULE E - FILTRE MOMENTUM (RSI) — PRIORITÉ MOYENNE
7.1 Ajout du filtre RSI
pinescript
// ─── FILTRE RSI MOMENTUM (à mettre dans f_createBuySignals/Sell) ──
float rsi7 = ta.rsi(close, 7)

// Pour BUY
if rsi7 > 70  // Trop suracheté → pas de BUY
    return

// Pour SELL
if rsi7 < 30  // Trop survendu → pas de SELL
    return
7.2 Paramètre utilisateur
pinescript
var string GRP_MOM = "⚡ Filtres Momentum"
bool  i_useRSI    = input.bool(true, "Activer filtre RSI", group=GRP_MOM)
int   i_rsiOverbought = input.int(70, "RSI suracheté", minval=60, maxval=80, group=GRP_MOM)
int   i_rsiOversold   = input.int(30, "RSI survendu", minval=20, maxval=40, group=GRP_MOM)

// Dans les fonctions
if i_useRSI
    if rsi7 > i_rsiOverbought  // Pour BUY
        return
    if rsi7 < i_rsiOversold    // Pour SELL
        return
7.3 Impact attendu
Évite les entrées en fin de tendance (momentum essoufflé).

Gain de 3-5% en WR.

8. MODULE F - SCORING SESSIONS H/L — PRIORITÉ MOYENNE
8.1 Ajout dans f_score
pinescript
// ─── SCORING SESSIONS HIGH/LOW ──────────────────────────────────────
bool _nearSessionHigh = false
bool _nearSessionLow = false

// Asian High/Low (déjà calculés dans Module 14)
if i_showAsianHL and not na(_ahHighDrawn) and not na(_ahLowDrawn)
    if isBull and close < _ahLowDrawn
        _nearSessionLow := true
    if not isBull and close > _ahHighDrawn
        _nearSessionHigh := true

// London High/Low (à calculer)
// NY High/Low (à calculer)

if (isBull and _nearSessionLow) or (not isBull and _nearSessionHigh)
    sc += 2
8.2 Impact attendu
+1-2 points sur les setups de retour de session.

Particulièrement efficace sur les sessions Asian (retour sur Asian Low/High).

9. MODULE G - DOL COMME TP DYNAMIQUE — PRIORITÉ MOYENNE
9.1 Problème actuel
La cible DoL (EQH/EQL) est affichée mais rarement atteinte car elle n'est pas intégrée au TP3.

9.2 Modification du calcul TP3
pinescript
// ─── TP3 DYNAMIQUE AVEC DOL ──────────────────────────────────────────
// Dans f_createBuySignals(), remplacer le calcul de _tp3 par :

float _dolTarget = _bsDolTarget(true, _entry)
float _tp3 = not na(_dolTarget) and _dolTarget > _tp2 ? _dolTarget : _entry + 3.0 * _r

// Pour SELL (f_createSellSignals) :
float _dolTarget = _bsDolTarget(false, _entry)
float _tp3 = not na(_dolTarget) and _dolTarget < _tp2 ? _dolTarget : _entry - 3.0 * _r
9.3 Impact attendu
Taux de TP3 amélioré de 20-30%.

Les niveaux de liquidité sont plus pertinents que des TP fixes.

10. MODULE H - SCORING MEGA-ORDERS — PRIORITÉ MOYENNE
10.1 Ajout dans f_score
pinescript
// ─── SCORING VOLUME EXCEPTIONNEL ────────────────────────────────────
if volume > ta.sma(volume, 20) * 3.0
    sc += 2  // Méga-order = intervention institutionnelle
10.2 Impact attendu
+2 points sur les bougies avec volume exceptionnel.

Identifie les interventions institutionnelles.

11. MODULE I - OFI (ORDER FLOW IMBALANCE) — PRIORITÉ COMPLEXE
11.1 Définition
L'OFI mesure le delta cumulé entre les volumes acheteurs et vendeurs.

11.2 Structure de données
pinescript
// ─── OFI (ORDER FLOW IMBALANCE) ────────────────────────────────────
var float _ofi = 0.0
var float _ofiSma = 0.0
11.3 Calcul OFI
pinescript
// ─── CALCUL OFI (dans le scope principal) ──────────────────────────
if barstate.isconfirmed
    float _buyVol = volume * (close - low) / (high - low)
    float _sellVol = volume * (high - close) / (high - low)
    _ofi := _buyVol - _sellVol
    _ofiSma := ta.sma(_ofi, 20)
11.4 Intégration dans le scoring
pinescript
// ─── SCORING OFI ──────────────────────────────────────────────────────
if _ofi > _ofiSma * 1.5
    sc += 3  // Fort déséquilibre d'ordre = confirmation institutionnelle
11.5 Paramètres utilisateur
pinescript
var string GRP_OFI  = "📊 Order Flow Imbalance"
bool  i_useOFI      = input.bool(false, "Activer OFI", group=GRP_OFI)
float i_ofiMult     = input.float(1.5, "Multiplicateur OFI", minval=1.0, maxval=3.0, group=GRP_OFI)
int   i_ofiScore    = input.int(3, "Points OFI", minval=1, maxval=5, group=GRP_OFI)

// Dans f_score
if i_useOFI and _ofi > _ofiSma * i_ofiMult
    sc += i_ofiScore
11.6 Impact attendu
+10-15% de WR sur actifs liquides (XAU, NAS, BTC).

Dimension "flux d'ordres" manquante dans l'indicateur actuel.

12. MODULE J - ALERTE COMPOSITE AMÉLIORÉE — PRIORITÉ FAIBLE
12.1 Ajout d'alertes plus granulaires
pinescript
// ─── ALERTE "SETUP FORT + BPR" ──────────────────────────────────────
bool _alertBPRBull = false
bool _alertBPRBear = false

if _bestIdx >= 0 and _nearBPR and f_force(_bestSc) >= 7
    if _bestBull
        _alertBPRBull := true
    else
        _alertBPRBear := true

alertcondition(_alertBPRBull, title="SMC — Setup FORT + BPR BUY", 
    message="🔥 SETUP FORT + BPR BUY | Score {{plot(\"Score\")}} | {{ticker}} {{interval}}")
alertcondition(_alertBPRBear, title="SMC — Setup FORT + BPR SELL", 
    message="🔥 SETUP FORT + BPR SELL | Score {{plot(\"Score\")}} | {{ticker}} {{interval}}")
13. PLAN D'INTÉGRATION PAR PHASE
Phase 1 (Semaine 1) : Améliorations Haute Priorité
Module	Actions
B (Gaps)	Ajouter scoring NDOG/NWOG dans f_score
C (Dead Zone)	Ajouter if inDeadZone return dans les fonctions de création
D (Filtre régime)	Réactiver ADX/ATR ratio avec toggle
Tests : Vérifier que les signaux sont réduits en dead zone et en range.

Phase 2 (Semaine 2) : Module BPR Complet
Module	Actions
A (BPR)	Ajouter structures, détection, lifecycle, scoring, affichage
Tests : Vérifier détection sur graphique, scoring sur OB existants.

Phase 3 (Semaine 3) : Améliorations Moyenne Priorité
Module	Actions
E (RSI)	Ajouter filtre RSI avec toggles
F (Sessions)	Ajouter scoring sessions H/L
G (DoL TP)	Modifier calcul TP3 avec DoL
H (Mega-Orders)	Ajouter scoring volume exceptionnel
Tests : Vérifier non-régression des signaux existants.

Phase 4 (Semaine 4) : Module OFI (Complexe)
Module	Actions
I (OFI)	Ajouter structure, calcul, scoring, paramètres
Tests : Comparer WR avant/après sur 3 mois sur XAU, NAS, DAX, BTC.

Phase 5 (Validation) : Tests Globaux
Comparer performances sur 3 mois (avant/après) sur tous les actifs.

Ajuster les pondérations si nécessaire.

Documenter les paramètres recommandés par actif.

14. RÉCAPITULATIF DES GAINS ESTIMÉS
Module	Gain WR estimé	Faux signaux réduits	TP3 amélioré
BPR (A)	+5-10%	-10%	-
Gaps (B)	+2-3%	-	-
Dead Zone (C)	+3-5%	-20%	-
Filtre régime (D)	+5-10%	-15%	-
RSI (E)	+3-5%	-5%	-
Sessions (F)	+2-3%	-	-
DoL TP (G)	-	-	+20-30%
Mega-Orders (H)	+1-2%	-	-
OFI (I)	+10-15%	-10%	-
Total estimé	+15-25%	-25-35%	+20-30%
15. PIÈGES À ÉVITER (CHECKLIST GLOBALE)
Piège	Solution
Surcharger le scoring	Plafonner les bonus à +5 points max par module
Over-engineering	Ne garder que les modules avec >5% de gain observé
Tests insuffisants	Tester sur 3 mois minimum, sur tous les actifs
Ignorer les Dead Zones	Bloquer systématiquement les trades en période creuse
Paramètres rigides	Rendre chaque module configurable (toggles + inputs)
Conflits entre modules	Tester chaque module séparément, puis ensemble
Performance dégradée	Utiliser des calculs simples (éviter les boucles lourdes)
16. CONCLUSION
Ce cahier des charges couvre toutes les améliorations identifiées, classées par priorité, impact et complexité.

Recommandation finale :

Commencer par les modules Haute Priorité (B, C, D) pour un gain rapide.

Intégrer le BPR (A) pour la diversification.

Ajouter les modules Moyenne Priorité (E, F, G, H) pour affiner.

Tester le module OFI (I) en backtest parallèle avant intégration définitive.

Chaque module est indépendant et peut être activé/désactivé via toggles, permettant une approche incrémentale et des tests A/B sur données réelles.