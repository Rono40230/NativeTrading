// ════════════════════════════════════════════════════════════════════
//  SMC INSTITUTIONAL SCALPING v12 — Portage MQL5 miroir (Pine étalon + Rust)
//  Portage de smc_indicateur_v12.pine (Pine Script v6 → MQL5) — miroir strict
//  Mono-symbole, anti-repaint (shift >= 1), moteur signaux uniquement
// ════════════════════════════════════════════════════════════════════
#property copyright "Rono"
#property version   "12.00"
#property strict
#property indicator_chart_window
#property indicator_plots 0

// === SECTION 0 : Header, inputs, enums, structs ===

// --- Input technique (debug uniquement) ---
input bool InpDiagScore     = false;   // (inutilisé — diag supprimé, gardé pour compatibilité paramètres)

// Variables d'affichage modifiables via panneau de contrôle (Solution A : panneau seul maître).
// Initialisées depuis GlobalVariableGet (persistance disque) dans LoadToggles(), fallback sur défaut.
bool g_showStructure, g_showBOS, g_showMSS, g_showCHOCH, g_showLiq, g_showSweep;
bool g_showFVG, g_showOB, g_showSignals, g_showLevels, g_showAsianHL, g_showZones;
bool g_showBgTrend, g_showBgVol, g_showBgAtr, g_showBgPD;
bool g_showBB, g_showPropulsion, g_showNDOG, g_showNWOG;
bool g_showAsian, g_showLondon, g_showNY;
// Phase 1.3/1.4 : IB, Equilibrium, OTE, OB HTF (7 toggles, vids 24-30).
bool g_showIB, g_showEqLn, g_showFIB, g_showH1, g_showH4, g_showW1, g_showMN;

// --- Constantes techniques (hardcodées, Pine AGENTS.md §5) ---
#define SMC_PREFIX       "SMC_"      // Préfixe objets graphiques (FIFO + nettoyage)
#define MAX_OB_PER_SIDE  40          // OB actifs max par sens (Pine i_maxOB)
#define MAX_FVG_PER_SIDE 10          // FVG actifs max par sens (Pine i_maxFVG)
#define MAX_VIS_LABELS   12          // Labels structure max (Pine _maxStructLbls)
#define MAX_VIS_LINES    6           // Lignes BOS/MSS/CHOCH max (Pine _bosMaxVis)
#define ATR_PERIOD       14          // Période ATR (Pine ta.atr(14))
#define BGCOLOR_BARS     500         // Fonds bgcolor : ne dessiner QUE les N dernières bougies visibles
#define FORCE_MIN        4           // Pine i_forceMin (codé en dur, AGENTS.md §3 calibrage v11 figé)
                                      // (anti-explosion : 1 rectangle/bougie/type → 500 objets max au lieu de 16000+)
// Modules décoratifs étape 5 — limites FIFO (Pine i_maxBB=5/sens, i_maxProp=3/sens, i_maxGap=3/type).
// Fidèle Pine MODULE 8b : pools SÉPARÉS par sens (i_maxBB=5 par sens, FIFO avec rotation).
// Avant : pool commun 20 sans rotation → divergence (trop de Breakers OU blocage une fois 20 atteints).
#define MAX_BB_PER_SIDE  5           // Breaker Blocks actifs max PAR SENS (Pine i_maxBB=5, en dur)
#define MAX_PROP_PER_SIDE 3          // Propulsion Blocks max par sens (Pine i_maxProp=3, en dur)
#define MAX_GAP_PER_TYPE 1           // Pine i_maxGap=1 par type (1 NDOG + 1 NWOG max, décision 2026-07-27)
#define MAX_GAPS         (MAX_GAP_PER_TYPE * 2)  // = 2 (1 NDOG + 1 NWOG max simultanés)
#define GAP_MIN_MULT     0.3         // Pine i_gapMinMult : taille min gap = 0.3 × ATR14

// Kill Zones (UTC, en minutes depuis minuit) — Pine lignes 125-132.
// Les 4 plages horaires de forte volatilité institutionnelle (Asie, Londres,
// NY AM, NY PM). Utilisées par InKzAt() puis la composante KZ du scoring.
#define KZ_ASIAN_START   0
#define KZ_ASIAN_END     180
#define KZ_LONDON_START  420
#define KZ_LONDON_END    570
#define KZ_NYAM_START    720
#define KZ_NYAM_END      840
#define KZ_NYPM_START    1020
#define KZ_NYPM_END      1140

// Sessions complètes pour l'AFFICHAGE (heure broker Axi = Paris + 1h).
// Décision Rono 2026-07-26 : affichage sessions en heure de Paris. Le broker Axi suit
// la DST US (UTC+2 hiver / UTC+3 été), Paris suit la DST EU (UTC+1 hiver / UTC+2 été).
// Comme les DST US/EU sont quasi-synchronisées, l'offset broker-Paris est CONSTANT à +1h.
// On travaille donc directement en heure broker (time[] est déjà en heure broker).
//   Asie Paris 0h00-6h30   → broker 1h00-7h30   → 60-450 min
//   Londres Paris 8h-16h30 → broker 9h-17h30    → 540-1050 min
//   NY Paris 14h30-21h     → broker 15h30-22h   → 930-1320 min
// LIMITATION : ~4 semaines/an (transitions US/EU décalées), l'offset peut temporairement
// être différent. Impact mineur sur l'affichage (1h de décalage pendant ces fenêtres).
#define SES_BROKER_PARIS_OFFSET  60    // offset broker - Paris (minutes, Axi = Paris+1h)
#define SES_BROKER_ASIE_START    (0   + SES_BROKER_PARIS_OFFSET)   // 60 = 1h00 broker
#define SES_BROKER_ASIE_END      (390 + SES_BROKER_PARIS_OFFSET)   // 450 = 7h30 broker
#define SES_BROKER_LONDON_START  (480 + SES_BROKER_PARIS_OFFSET)   // 540 = 9h00 broker
#define SES_BROKER_LONDON_END    (990 + SES_BROKER_PARIS_OFFSET)   // 1050 = 17h30 broker
#define SES_BROKER_NY_START      (870 + SES_BROKER_PARIS_OFFSET)   // 930 = 15h30 broker
#define SES_BROKER_NY_END        (1260+ SES_BROKER_PARIS_OFFSET)   // 1320 = 22h00 broker

// --- Structs du moteur SMC (à extraire en .mqh Phase 2) ---
// === MOTEUR SMC — structures de données ===

enum ENUM_SWING_KIND { SWING_HH = 1, SWING_LH = 2, SWING_HL = 3, SWING_LL = 4 };

struct SwingT {        // pivot structure (HH/HL/LH/LL)
    datetime  t;       // timestamp du pivot
    double    price;    // niveau du pivot
    int       barIdx;   // index bar Pine-équivalent (bar_index absolu)
    int       kind;     // ENUM_SWING_KIND
};

struct FVGT {          // Fair Value Gap
    datetime  t0;       // bar d'origine (1re des 3)
    double    top;
    double    bot;
    int       state;    // 0=actif, 1=mitigé
    int       barIdx;   // bar_index d'origine (Pine bar_index[2])
};

struct OBT {           // Order Block
    datetime  t0;       // bar d'origine (bougie avant impulsion)
    double    top;      // _rt (bord haut = high[1] bougie impulsion)
    double    bot;      // _rb (bord bas = low[1] bougie impulsion)
    double    mid;      // _mid (centre)
    int       state;    // _rst : 0=vierge, 1=touché/mitigé, 2=rempli
    int       force;    // score force 0-10 (via f_force)
    int       score;    // score brut (accumulé)
    bool      bull;     // true=bull OB, false=bear OB
    bool      isIB;     // Inner Bar (Pine ibBull[1]/ibBear[1])
    bool      signaled; // OB déjà signalé (Pine obBullSignaled)
    bool      zoneActive; // LATCH zone (Pine obBullZoneBox != na) : true si zone créée, reste jusqu'à suppression OB
    int       barIdx;   // bar_index bougie impulsion (anti-suppression immédiate)
    // DIAGNOSTIC (temporaire) : mémorise les composantes actives au moment où le score a
    // atteint son max (figé). Permet d'identifier la composante divergente vs TradingView.
    int   diagMaxSc;        // base f_score au moment du max
    datetime diagMaxT;      // tBar au moment du max
    string diagFlags;       // flags actifs au moment du max
};

struct SignalT {       // signal BUY/SELL généré
    datetime  t;        // bar de détection (création)
    double    entry;    // bord OB (TR forcé : _rt pour BUY, _rb pour SELL)
    double    sl;       // SL (initial, peut devenir BE=entry après TP1 hit)
    double    tp1;      // TP1 (entry ± R)
    double    tp2;      // TP2 (entry ± 2R)
    double    tp3;      // TP3 (entry ± 3R, ou liquidité la plus proche)
    int       force;    // force 0-10 (f_force)
    int       score;    // score brut
    bool      bull;     // true=BUY, false=SELL
    int       obIdx;    // index de l'OB sous-jacent (pour _scoreDeg : dégradation force OB)
    datetime  openTs;   // timestamp de création (Pine stBullOpenTs, pour expiration)
    // --- Lifecycle (machine à états, clone Pine L3356-3449) ---
    bool      filled;   // true si le prix a touché l'entry (fill retest, Pine stBullFilled)
    datetime  fillT;    // bar de fill (Pine _fillL) — boxes ré-ancrées ici si filled
    bool      t1Hit;    // TP1 touché (Pine stBullTP1Hit) → SL passe à BE (entry)
    datetime  t2Ts;     // timestamp TP2 touché (Pine stBullTP2HitTs, 0 = non touché)
    bool      closed;   // trade fermé (SL/BE/TP2SL/TP3/expire/BOS contre) → arrêt affichage
    datetime  closeT;   // bar de clôture (bord droit des boxes)
    string    closeRsn; // raison de clôture lisible ("SL", "BE", "TP2SL", "TP3", "EXPIRE", "BOS")
    double    closeR;   // R final à la clôture (close vs entry, /R) — pour EXPIRE principalement
    double    R0;       // R initial figé à la création (entry - sl initial), pour calcul closeR stable
};

// --- Modules décoratifs étape 5 — structs (Breaker / Propulsion / NDOG-NWOG) ---
// DÉCORATIF en v11 (AGENTS §3) : retirés du scoring, juste affichés. La zone d'un Breaker =
// la zone exacte de l'OB invalidé directionnellement (Pine lignes 1017-1036 / 1095-1114) :
//   - OB BULL invalidé par close < botB → Bearish Breaker (résistance, bull=false).
//   - OB BEAR invalidé par close > topBr → Bullish Breaker (support, bull=true).
struct BreakerT {
    datetime t0;        // bar de création du Breaker (= bar d'invalidation de l'OB source)
    double   top;       // zone = zone exacte de l'OB invalidé (Pine push topB/topBr)
    double   bot;
    bool     bull;      // true=bullish breaker (support), false=bearish breaker (résistance)
};

// Propulsion Block = chevauchement FVG+OB MÊME SENS (Pine MODULE 8c, f_fvgBullOB/BearOB).
// Le Pine conserve la zone de chevauchement (ovBot/ovTop), pas l'OB entier. On stocke donc
// la zone d'overlap + le sens + le bar d'origine pour l'affichage. FIFO 3 par sens (i_maxProp).
struct PropT {
    datetime t0;        // bar de création (= bar du FVG, i)
    double   top;       // bord haut de la zone de chevauchement FVG∩OB
    double   bot;       // bord bas de la zone de chevauchement FVG∩OB
    bool     bull;      // true=Propulsion bull (FVG bull ∩ OB bull), false=bear
};

// NDOG/NWOG = gap entre clôture veille et ouverture nouveau jour/semaine (Pine MODULE 10b).
// Zone = [min(open, close[1]), max(open, close[1])] si gap ≥ 0.3×ATR14. Gating TF Pine :
// NDOG pertinent M1-M15, NWOG pertinent H1-H4 (sinon ni créé ni affiché).
struct GapT {
    datetime t0;        // bar d'ouverture du nouveau jour/semaine (bar_index dans le Pine)
    double   top;       // max(open, close[1]) — bord haut du gap
    double   bot;       // min(open, close[1]) — bord bas du gap
    bool     isDay;     // true=NDOG (new day), false=NWOG (new week)
    bool     mitigated; // true = gap traversé (low<=bot && high>=top) → recoloration atténuée
};

// --- Calibration automatique par asset (portage Pine lignes 19-82) ---
// Reproduit exactement le bloc de calibration du Pine.
bool   g_isXAU, g_isXAG, g_isNAS, g_isBTC, g_isDAX, g_isSPX, g_assetReconnu;
double g_pipValue, g_pvLot;
int    g_tf;          // timeframe en minutes (Pine _tf)
bool   g_tfM15, g_tfH1, g_tfH4;
int    g_autoSwing;   // longueur pivot par asset (Pine _autoSwing)
int    g_autoRocSeuil; // seuil ROC bps (Pine _autoRocSeuil = 5)
double g_autoSeuilIB;
double g_autoAtrScore, g_autoAtrSeuil;
int    g_autoTpWidth, g_autoTp3Mins;
int    g_autoTradeMaxMins;   // Pine _autoTradeMaxMins (L2305) : durée max trade avant expiration
string g_autoSlMode;

void InitAssetCalibration() {
    string tk = _Symbol;
    g_isXAU = (StringFind(tk, "XAU") >= 0);
    g_isXAG = (StringFind(tk, "XAG") >= 0);
    g_isNAS = (StringFind(tk, "NAS") >= 0) || (StringFind(tk, "NDX") >= 0) || (StringFind(tk, "US100") >= 0);
    g_isBTC = (StringFind(tk, "BTC") >= 0);
    g_isDAX = (StringFind(tk, "DAX") >= 0) || (StringFind(tk, "GER40") >= 0) || (StringFind(tk, "DE30") >= 0);
    // Phase 5 29/08 : SP500 — profil miroir NAS100 (indices US CFD jumeaux).
    g_isSPX = (StringFind(tk, "SP500") >= 0) || (StringFind(tk, "SPX") >= 0) || (StringFind(tk, "US500") >= 0);
    g_assetReconnu = g_isXAU || g_isXAG || g_isNAS || g_isBTC || g_isDAX || g_isSPX;

    // Pine lignes 33-36
    g_pipValue = g_isXAU ? 0.1 : g_isXAG ? 0.01 : 1.0;
    g_pvLot    = g_isXAU ? 10.0 : g_isXAG ? 50.0 : (g_isNAS || g_isSPX) ? 20.0 :
                 g_isBTC ? 1.0  : g_isDAX ? 25.0  : 10.0;

    g_tf   = PeriodSeconds(_Period) / 60;
    g_tfM15 = (g_tf <= 15);
    g_tfH1  = (g_tf <= 60);
    g_tfH4  = (g_tf <= 240);

    // Pine lignes 42-47
    g_autoSwing =
        g_isXAU && g_tfM15 ? 3 : g_isXAU ? 5 :
        g_isXAG && g_tfM15 ? 3 : g_isXAG ? 4 :
        g_isNAS && g_tfM15 ? 3 : g_isNAS ? 5 :
        g_isBTC && g_tfM15 ? 3 : g_isBTC ? 5 :
        g_isDAX && g_tfM15 ? 3 : g_isDAX ? 5 : 5;

    g_autoRocSeuil = 5;   // Pine ligne 51

    g_autoSeuilIB = g_isXAU ? 1.5 : g_isXAG ? 1.2 : g_isNAS ? 1.5 :
                    g_isBTC ? 2.0 : g_isDAX ? 1.5 : 1.5;

    g_autoAtrScore = g_isXAU ? 3.0 : g_isXAG ? 2.5 : g_isNAS ? 3.0 :
                     g_isBTC ? 3.5 : g_isDAX ? 3.0 : 3.0;

    g_autoAtrSeuil = g_isXAU ? 2.0 : g_isXAG ? 1.8 : g_isNAS ? 2.0 :
                     g_isBTC ? 2.5 : g_isDAX ? 2.0 : 2.0;

    g_autoTpWidth = 40;
    if(g_tfH1)       g_autoTpWidth = 30;
    else if(g_tfH4)  g_autoTpWidth = 20;

    // Pine L71-76 : _autoTp3Mins (délai max TP2→TP3, en minutes).
    g_autoTp3Mins = g_isXAU && g_tfM15 ? 60  : g_isXAU && g_tfH1 ? 240 :
                    g_isXAG && g_tfM15 ? 45  : g_isXAG && g_tfH1 ? 180 :
                    (g_isNAS || g_isSPX) && g_tfM15 ? 30  : (g_isNAS || g_isSPX) && g_tfH1 ? 120 :
                    g_isBTC && g_tfM15 ? 90  : g_isBTC && g_tfH1 ? 360 :
                    g_isDAX && g_tfM15 ? 30  : g_isDAX && g_tfH1 ? 120 : 60;

    // Pine L2305 : _autoTradeMaxMins (durée max d'un trade avant expiration, en minutes).
    g_autoTradeMaxMins = g_tfH1 ? 480 : g_tfH4 ? 1920 : (PeriodSeconds(_Period) == 86400) ? 5760 : 240;

    g_autoSlMode =
        g_isBTC ? "2× ATR sous OB" :
        g_isXAG ? "1.5× ATR sous OB" :
        (g_isXAU || g_isNAS || g_isSPX || g_isDAX) ? "1× ATR sous OB" : "Bas OB";
}

// --- Anti-repaint : détection nouveau bar ---
datetime g_lastBarTime = 0;
bool IsNewBar() {
    datetime t = iTime(_Symbol, _Period, 0);
    if(t != g_lastBarTime) { g_lastBarTime = t; return true; }
    return false;
}

// === SECTION 1 : Structure de marché (HH/HL/LH/LL, pivots) ===
// Portage Pine lignes 235-332.
// === MOTEUR SMC — structure ===

// Pivots : sh1=plus haut récent, sh2=précédent, sl1/sl2 idem bas.
// Pine lignes 268-286 : on garde les 2 derniers pivots de chaque côté.
double g_sh1, g_sh2, g_sl1, g_sl2;
int    g_bsh1, g_bsh2, g_bsl1, g_bsl2;   // bar_index des pivots (Pine absolu)
bool   g_sh1_valid, g_sl1_valid;          // true dès qu'un premier pivot a été vu
// Flags "pivot confirmé AU BAR COURANT" (Pine not na(ph)/not na(pl)). Requis pour le CHOCH
// (Pine chochHaussier exige isHL = not na(pl) ET pl > sl2). Sans ça, isHL_pivot reste vrai
// indéfiniment tant que sl1 > sl2 → CHOCH sur-déclenché → sur-scoring +4 (bug audit J4).
bool   g_hasPH_thisBar = false, g_hasPL_thisBar = false;

int    g_bullCount, g_bearCount;          // Pine lignes 293-300
bool   g_tendanceHaussiere, g_tendanceBaissiere;  // Pine lignes 301-302

// Flags signaux structure (Pine lignes 355-358, 389-397)
int    g_dernierSH1_sig, g_dernierSL1_sig;
bool   g_mssHPending, g_mssBPending;

// --- Événements structure pour l'affichage décoratif (labels HH/HL/LH/LL + lignes MSS/CHOCH) ---
// MSS/CHOCH/HH/HL/LH/LL/Sweep sont des événements ponctuels (1 bar précis). On les capture pendant
// la boucle bar-par-bar dans cet array FIFO, puis on les dessine APRÈS la boucle (Option B,
// comme f_drawOB/f_drawFVG/f_drawLevels). FIFO max 100 événements (g_nStructEvents < 100).
// kind : 1=HH, 2=LH, 3=HL, 4=LL, 5=MSS_h, 6=MSS_b, 7=CHOCH_h, 8=CHOCH_b, 9=SwpBull, 10=SwpBear.
// Un événement structurel ponctuel (HH/HL/LH/LL, BOS, MSS, CHOCH, Sweep).
// Clone Pine : 5 pools séparés avec FIFO propre (cf. Pine _structLbls=12, _bosMaxVis=6, etc.).
struct StructEventT {
    datetime t;       // timestamp de l'événement (départ : pivot pour HH, niveau cassé pour BOS/MSS/CHOCH)
    datetime t2;      // bord droit optionnel (BOS uniquement : bougie du BOS = fin de ligne, Pine bar_index)
    double   price;   // niveau (pivot pour structure, niveau cassé pour BOS/MSS/CHOCH, low/high sweep)
    int      kind;    // 1=HH, 2=LH, 3=HL, 4=LL, 5=MSS_h, 6=MSS_b, 7=CHOCH_h, 8=CHOCH_b,
                      // 9=Sweep bull (▲), 10=Sweep bear (▼), 11=BOS_h, 12=BOS_b
};
// 5 pools dédiés (clone Pine _maxStructLbls=12, _bosMaxVis=6, _sweepMaxVis=6 — L276/462/652).
// Chaque pool a son propre FIFO : les HH n'évincent pas les BOS, etc. PAS de filtre 24h
// (le Pine garde les N derniers events depuis le début du graphique, peu importe l'âge).
#define MAX_POOL_HH    12   // Pine _maxStructLbls (HH/HL/LH/LL)
#define MAX_POOL_BOS    6   // Pine _bosMaxVis
#define MAX_POOL_MSS    6   // Pine _bosMaxVis (MSS réutilise _bosMaxVis)
#define MAX_POOL_CHOCH  6   // Pine _bosMaxVis (CHOCH réutilise _bosMaxVis)
#define MAX_POOL_SWP    6   // Pine _sweepMaxVis
StructEventT g_poolHH[MAX_POOL_HH];       int g_nHH = 0;
StructEventT g_poolBOS[MAX_POOL_BOS];     int g_nBOS = 0;
StructEventT g_poolMSS[MAX_POOL_MSS];     int g_nMSS = 0;
StructEventT g_poolCHOCH[MAX_POOL_CHOCH]; int g_nCHOCH = 0;
StructEventT g_poolSWP[MAX_POOL_SWP];     int g_nSWP = 0;

// PushPool : ajoute un événement dans un pool dédié avec FIFO circulaire (shift si plein).
// Helper générique : remplace PushStructEvent (pool commun) par 5 pools séparés fidèles Pine.
// t2 = bord droit optionnel (utilisé par BOS : bougie du BOS, Pine line.new(bsh1,...,bar_index,...)).
void PushPool(StructEventT &pool[], int &n, int maxN, datetime t, double price, int kind, datetime t2 = 0) {
    if(n >= maxN) {
        for(int k = 0; k < maxN - 1; k++) pool[k] = pool[k + 1];
        n = maxN - 1;
    }
    pool[n].t     = t;
    pool[n].t2    = t2;
    pool[n].price = price;
    pool[n].kind  = kind;
    n++;
}

// --- Stockage des états bgcolor pour les N dernières bougies (phase décorative étape 3) ---
// Pine bgcolor() colore TOUTES les bougies (tendance/volume/ATR/PD). En MQL5 on ne peut pas
// créer 16000+ rectangles (saturation MT5). On stocke donc l'état des BGCOLOR_BARS dernières
// bougies pendant la boucle (via f_storeBg), puis f_drawBg dessine un rectangle par bar active
// APRÈS la boucle (Option B). Index dans l'array = i - (rates_total - BGCOLOR_BARS).
// NB : ces flags sont instantanés au bar i (1 booléen/bougie), pas de lifecycle ni d'expiration.
bool g_bgTrendBull[BGCOLOR_BARS];   // tendanceHaussiere (Pine ligne 301) au bar idx
bool g_bgTrendBear[BGCOLOR_BARS];   // tendanceBaissiere (Pine ligne 302)
bool g_bgVol[BGCOLOR_BARS];         // volume fort (Pine MODULE 9 volOk)
bool g_bgAtrBull[BGCOLOR_BARS];     // impulsion ATR haussière (Pine MODULE 10 atrBull)
bool g_bgAtrBear[BGCOLOR_BARS];     // impulsion ATR baissière (Pine MODULE 10 atrBear)
bool g_bgPrem[BGCOLOR_BARS];        // inPremium (Pine MODULE PD, ligne 1510)
bool g_bgDisc[BGCOLOR_BARS];        // inDiscount (Pine MODULE PD, ligne 1511)
int  g_bgCount = 0;                 // combien d'entrées valides dans les arrays (≤ BGCOLOR_BARS)

// === MOTEUR SMC — structure : calcul des pivots + HH/HL/LH/LL ===
// Reproduit Pine lignes 264-302.
// Pivot Pine : ta.pivothigh(high, len, len) retourne la valeur à shift=len si
// high[len] est le max local sur [0..2*len], sinon na. Confirmation = len bars après.
//
// Pine exécute le script séquentiellement bar-par-bar : à chaque bar_index courant,
// ta.pivothigh regarde la fenêtre [bar_index-2*len .. bar_index] et renvoie
// high[bar_index-len] si c'est le max local. Sous barstate.isconfirmed, le "bar
// courant" Pine = le bar clôturé.
//
// Le portage MQL5 rejoue donc le moteur bar-par-bar : à chaque bar courant `i`
// (= bar clôturé traité, comme dans la boucle OnCalculate qui s'arrête à
// rates_total-2), on évalue le pivot candidat à `i - len` sur la fenêtre
// `[i - 2*len .. i]`. Le pivot est confirmé seulement quand `i >= 2*len`.

// Retourne true si un pivot haut est confirmé au bar `i - len` (vu depuis le bar
// courant clôturé `i`). Met ph au niveau. high[] est la série chronologique MQL5
// (index 0 = bar le plus ancien). Anti-repaint respecté : `i` vient toujours de la
// boucle OnCalculate qui s'arrête à rates_total-2 (jamais le bar en cours).
bool PivotHighAt(const double &high[], int len, int i, double &ph) {
    if(i < 2 * len) return false;            // pivot non encore confirmé (fenêtre incomplète)
    int cand = i - len;                      // bar pivot candidat
    int windowLeft = i - 2 * len;            // borne ancienne de la fenêtre centrée
    double candidate = high[cand];
    for(int k = 0; k <= 2 * len; k++) {
        int j = windowLeft + k;
        // Fix plateaux 2026-07-27 : test >= (et non > strict) pour aligner avec ta.pivothigh Pine.
        // Une bougie de hauteur ÉGALE casse le pivot (sinon MQL5 confirmait des pivots en plateau
        // que Pine ignorait → EQH/EQL fantômes en MQL5).
        if(j != cand && high[j] >= candidate) return false;
    }
    ph = candidate;
    return true;
}

bool PivotLowAt(const double &low[], int len, int i, double &pl) {
    if(i < 2 * len) return false;
    int cand = i - len;
    int windowLeft = i - 2 * len;
    double candidate = low[cand];
    for(int k = 0; k <= 2 * len; k++) {
        int j = windowLeft + k;
        if(j != cand && low[j] <= candidate) return false;   // <= (plateaux) aligné ta.pivotlow
    }
    pl = candidate;
    return true;
}

void f_structureCompute(int i, const int &bar_index[],
                        const datetime &time[],
                        const double &high[], const double &low[]) {
    int len = g_autoSwing;
    double ph = 0, pl = 0;
    bool hasPH = PivotHighAt(high, len, i, ph);
    bool hasPL = PivotLowAt(low, len, i, pl);
    // Persister les flags "pivot confirmé au bar courant" pour f_chochCompute (audit J4) :
    // le CHOCH Pine exige isHL = not na(pl) (pivot bas confirmé CE bar). Sans ça, isHL_pivot
    // reste vrai indéfiniment → CHOCH sur-déclenché → sur-scoring.
    g_hasPH_thisBar = hasPH;
    g_hasPL_thisBar = hasPL;
    // Pine ligne 266 : barPivot = bar_index[i_swingLength]  (bar_index absolu du pivot).
    // En rejeu bar-par-bar, le bar pivot confirmé est à l'index absolu `i - len`.
    int pivotPos = i - len;
    int barPivot = (pivotPos >= 0) ? bar_index[pivotPos] : 0;

    // Pine lignes 277-286 : décalage sh1<-sh2, sl1<-sl2
    if(hasPH) {
        g_sh2 = g_sh1;  g_bsh2 = g_bsh1;
        g_sh1 = ph;     g_bsh1 = barPivot;
        g_sh1_valid = true;
    }
    if(hasPL) {
        g_sl2 = g_sl1;  g_bsl2 = g_bsl1;
        g_sl1 = pl;     g_bsl1 = barPivot;
        g_sl1_valid = true;
    }

    // Pine lignes 288-291 : HH/LH/HL/LL (sh2/sl2 valide dès le 2e pivot = g_bsh2/g_bsl2 != 0)
    bool isHH = hasPH && g_bsh2 != 0 && ph > g_sh2;
    bool isLH = hasPH && g_bsh2 != 0 && ph < g_sh2;
    bool isHL = hasPL && g_bsl2 != 0 && pl > g_sl2;
    bool isLL = hasPL && g_bsl2 != 0 && pl < g_sl2;

    // Pine lignes 293-302 : compteurs tendance
    if(isHH || isHL) {
        g_bullCount += 1;
        g_bearCount = (int)MathMax(0, g_bearCount - 1);
    }
    if(isLH || isLL) {
        g_bearCount += 1;
        g_bullCount = (int)MathMax(0, g_bullCount - 1);
    }
    g_tendanceHaussiere = (g_bullCount >= 2);
    g_tendanceBaissiere = (g_bearCount >= 2);

    // ── Capture des événements structure (HH/HL/LH/LL) pour l'affichage décoratif ──
    // (phase décorative étape 2). Les variables isHH/isLH/isHL/isLL sont locales à cette
    // fonction : on les pousse dans g_structEvents[] pour que f_drawStructEvents (appelée
    // APRÈS la boucle) puisse les dessiner. Le timestamp = time[barPivot] = time[i - len]
    // (bar où le pivot a été confirmé, fidèle au Pine barPivot).
    // CONDITIONNEL au toggle (règle #1 clone Pine) : Pine `if i_moteurStructure and i_showLabels` (L319).
    // Si g_showStructure est OFF pendant la boucle, aucun label n'est capturé → activer le toggle
    // après coup affiche un pool vide (comportement identique au Pine).
    if(g_showStructure && (isHH || isLH || isHL || isLL)) {
        int kind = isHH ? 1 : (isLH ? 2 : (isHL ? 3 : 4));
        PushPool(g_poolHH, g_nHH, MAX_POOL_HH, time[pivotPos], (isHH || isLH) ? ph : pl, kind);
    }
}

// === SECTION 2 : BOS (Break of Structure) ===
// Portage Pine lignes 355-370.
// === MOTEUR SMC — BOS ===
// Validation BOS par clôture uniquement, mèches ignorées (AGENTS.md §5).
// Pine : close > sh1 and close[1] <= sh1. En rejeu bar-par-bar, "close" = close[i]
// (bar clôturé courant), "close[1]" = close[i-1]. Anti-repaint : le bar `i` vient
// toujours de la boucle OnCalculate qui s'arrête à rates_total-2 (jamais le bar en cours).

bool g_bosHaussier, g_bosBaissier;

void f_bosCompute(int i, const double &close[], const double &open[]) {
    if(i < 2) { g_bosHaussier = false; g_bosBaissier = false; return; }
    if(!g_sh1_valid || !g_sl1_valid) { g_bosHaussier = false; g_bosBaissier = false; return; }

    double c0 = close[i];     // "close" Pine (bar clôturé courant)
    double c1 = close[i - 1]; // "close[1]" Pine

    // Pine ligne 357-358 : close > sh1 and close[1] <= sh1 and (na(dernierSH1_sig) or bsh1 != ...)
    g_bosHaussier = (c0 > g_sh1) && (c1 <= g_sh1) &&
                    (g_dernierSH1_sig == 0 || g_bsh1 != g_dernierSH1_sig);
    g_bosBaissier = (c0 < g_sl1) && (c1 >= g_sl1) &&
                    (g_dernierSL1_sig == 0 || g_bsl1 != g_dernierSL1_sig);
}

// === SECTION 3 : MSS / CHOCH ===
// Portage Pine lignes 372-439.
// === MOTEUR SMC — MSS / CHOCH ===
// MSS  = premier BOS contre la tendance (alerte précoce, non confirmé)
// CHOCH = MSS + swing confirmé dans le nouveau sens (signal fiable)

bool   g_mssHaussier, g_mssBaissier, g_chochHaussier, g_chochBaissier;
double g_dernierMSS_level, g_dernierCHOCH_level;
int    g_dernierMSS_bar, g_dernierCHOCH_bar;

void f_chochCompute(int i, const int &bar_index[], const datetime &time[]) {
    // Pine lignes 385-386 : MSS = premier BOS contre la tendance dominante
    g_mssHaussier  = g_bosHaussier  && g_tendanceBaissiere;
    g_mssBaissier  = g_bosBaissier  && g_tendanceHaussiere;

    // Pine lignes 392-397 : flags persistants
    if(g_mssHaussier)  { g_mssHPending = true;  g_mssBPending = false; }
    if(g_mssBaissier)  { g_mssBPending = true;  g_mssHPending = false; }

    // Pine lignes 401-402 : CHOCH = MSS pending + nouveau swing HL/LH confirmé AU BAR COURANT.
    // BUG AUDIT J4 corrigé : le Pine exige isHL = not na(pl) ET pl > sl2, où `pl` est le pivot
    // bas confirmé CE BAR (sortie de ta.pivotlow). Avant, on comparait juste sl1 > sl2 (pivots
    // archivés connus depuis plusieurs bars) → isHL_pivot restait vrai indéfiniment → CHOCH
    // sur-déclenché → sur-scoring +4. Maintenant on exige g_hasPL_thisBar (pivot bas confirmé
    // ce bar, persisté par f_structureCompute), fidèle au not na(pl) du Pine.
    bool isHL_pivot = g_hasPL_thisBar && (g_sl1_valid && g_bsl2 != 0 && g_sl1 > g_sl2);
    bool isLH_pivot = g_hasPH_thisBar && (g_sh1_valid && g_bsh2 != 0 && g_sh1 < g_sh2);
    g_chochHaussier = g_mssHPending && isHL_pivot && !g_mssHaussier;
    g_chochBaissier = g_mssBPending && isLH_pivot && !g_mssBaissier;

    if(g_chochHaussier) g_mssHPending = false;
    if(g_chochBaissier) g_mssBPending = false;

    // Pine lignes 416-439 : reset tendance au MSS + archivage niveaux + dernier*_sig
    int curBar = bar_index[i];
    if(g_mssHaussier) {
        g_bullCount = 0; g_bearCount = 0;
        g_dernierSH1_sig   = g_bsh1;
        g_dernierMSS_level = g_sh1; g_dernierMSS_bar = curBar;
    }
    if(g_mssBaissier) {
        g_bullCount = 0; g_bearCount = 0;
        g_dernierSL1_sig   = g_bsl1;
        g_dernierMSS_level = g_sl1; g_dernierMSS_bar = curBar;
    }
    if(g_chochHaussier) {
        g_dernierCHOCH_level = g_sh1; g_dernierCHOCH_bar = curBar;
    }
    if(g_chochBaissier) {
        g_dernierCHOCH_level = g_sl1; g_dernierCHOCH_bar = curBar;
    }
    // Pine lignes 436-439 : BOS hors MSS met à jour le flag anti-doublon
    if(g_bosHaussier && !g_mssHaussier) g_dernierSH1_sig = g_bsh1;
    if(g_bosBaissier && !g_mssBaissier) g_dernierSL1_sig = g_bsl1;

    // ── Capture des événements MSS/CHOCH pour l'affichage décoratif ──
    // (phase décorative étape 2). MSS/CHOCH sont des événements ponctuels (1 bar) ; on les
    // pousse dans g_structEvents[] pour f_drawStructEvents (appelée APRÈS la boucle).
    // Niveau cassé : MSS/CHOCH haussier = g_sh1 (cassure du swing haut), baissier = g_sl1.
    // Timestamp = time[i] (bar courant où la cassure est confirmée).
    // Capture INCONDITIONNELLE (filtrage du toggle g_showMSS/CHOCH reporté à l'affichage,
    // fidèle au pattern f_liqCompute — sinon activer le toggle après coup affiche un tableau vide).
    // Capture dans 3 pools séparés (clone Pine _mssLines/_chochLines/_bosLines, FIFO 6 chacun).
    // Les 3 (BOS/MSS/CHOCH) utilisent le même pattern Pine : line.new(bsh1, sh1, bar_index, sh1)
    // → t = timestamp du pivot cassé (départ), t2 = timestamp de l'événement (bougie, fin de ligne).
    // CONDITIONNEL au toggle (règle #1 clone Pine) : Pine `if i_moteurCHOCH and i_showMSS` etc.
    datetime evT = time[i];
    int rates_total_loc = ArraySize(time);
    bool pivotHOk = (g_bsh1 != 0 && g_bsh1 < rates_total_loc);
    bool pivotLOk = (g_bsl1 != 0 && g_bsl1 < rates_total_loc);
    if(g_showMSS) {
        if(g_mssHaussier   && pivotHOk) PushPool(g_poolMSS,   g_nMSS,   MAX_POOL_MSS,   time[g_bsh1], g_sh1, 5, evT);
        if(g_mssBaissier   && pivotLOk) PushPool(g_poolMSS,   g_nMSS,   MAX_POOL_MSS,   time[g_bsl1], g_sl1, 6, evT);
    }
    if(g_showCHOCH) {
        if(g_chochHaussier && pivotHOk) PushPool(g_poolCHOCH, g_nCHOCH, MAX_POOL_CHOCH, time[g_bsh1], g_sh1, 7, evT);
        if(g_chochBaissier && pivotLOk) PushPool(g_poolCHOCH, g_nCHOCH, MAX_POOL_CHOCH, time[g_bsl1], g_sl1, 8, evT);
    }
    if(g_showBOS) {
        // BOS : exclut les MSS pour éviter doublon (Pine bosHaussier and not mssHaussier).
        if(g_bosHaussier  && !g_mssHaussier  && pivotHOk) PushPool(g_poolBOS, g_nBOS, MAX_POOL_BOS, time[g_bsh1], g_sh1, 11, evT);
        if(g_bosBaissier  && !g_mssBaissier  && pivotLOk) PushPool(g_poolBOS, g_nBOS, MAX_POOL_BOS, time[g_bsl1], g_sl1, 12, evT);
    }
}

// === SECTION 3bis : Premium/Discount + OTE (scoring) ===
// Portage Pine lignes 1485-1495 (PD) et 1846-1897 (OTE).
// === MOTEUR SMC — Premium/Discount & OTE ===
// Composantes 100% locales (calculs Fib sur les swings sh1/sl1). Contrairement au
// Pine qui utilise `na`, en MQL5 on simule l'absence d'initialisation via :
//   - g_pdOk        (bool séparé) pour la plage Premium/Discount,
//   - g_fibBullBar/g_fibBearBar == -1 (garde anti-na) pour les plages OTE.
//
// IMPORTANT : la capture se fait au BOS (Pine 1487-1489 et 1860-1867), PAS au MSS/CHOCH.
// La plage PD est figée au dernier BOS (qu'il soit haussier ou baissier) : c'est le
// "dealing range" SMC. Les plages OTE sont capturées séparément pour le sens bull et
// le sens bear (Pine utilise 2 paires de variables indépendantes), avec une expiration
// temporelle de OTE_EXPIRY_BARS (Pine 1870-1875) = max(1, round(10800/PeriodSeconds)).

// Premium/Discount (Pine 1485-1495).
double g_pdRangeH = 0, g_pdRangeL = 0;
bool   g_pdOk = false;            // false = plage PD pas encore capturée (équiv. Pine `na`)
double g_pdEquilibrium = 0;
bool   g_inPremium = false, g_inDiscount = false;
#define EQ_TOL 0.5                // Pine i_eqTol (en %, ligne 1480).

// OTE - Optimal Trade Entry (Pine 1846-1897).
double g_fibSHL = 0, g_fibSHH = 0;   // plage OTE bull (sl1/sh1 au dernier BOS haussier)
int    g_fibBullBar = -1;            // -1 = plage OTE bull pas/plus active (garde anti-na)
double g_fibSBH = 0, g_fibSBL = 0;   // plage OTE bear (sh1/sl1 au dernier BOS baissier)
int    g_fibBearBar = -1;            // -1 = plage OTE bear pas/plus active (garde anti-na)
bool   g_inOTE_bull = false, g_inOTE_bear = false;
#define FIB_OTE_HIGH 0.618          // Pine ligne 1883.
#define FIB_OTE_LOW  0.786          // Pine ligne 1884.

// Calcule, au bar courant `i`, les composantes Premium/Discount + OTE du scoring.
// Doit être appelée APRÈS f_chochCompute (qui produit g_bosHaussier/g_bosBaissier à
// partir des flags BOS figés par f_bosCompute) et AVANT f_accumScores (qui consomme
// g_inPremium/g_inDiscount/g_inOTE_* via f_score). Anti-repaint : tout sur `i`.
//
// Pine équivalence : le script exécute ces blocs séquentiellement à chaque bar.
// Ici on rejoue bar-par-bar : les `var` Pine deviennent des globales persistantes
// entre 2 appels de la boucle (seul ResetSmcState les remet à zéro).
void f_pdOteCompute(int i, const int &bar_index[], const datetime &time[], const double &close[]) {
    // --- Premium/Discount : capture de la plage au dernier BOS (Pine 1487-1489) ---
    // Les 2 sens (bosHaussier OR bosBaissier). Gardes na → g_sh1_valid && g_sl1_valid.
    // Condition Pine sh1 > sl1 reproduite (sinon plage invalide / inversée).
    if((g_bosHaussier || g_bosBaissier) && g_sh1_valid && g_sl1_valid && g_sh1 > g_sl1) {
        g_pdRangeH = g_sh1;
        g_pdRangeL = g_sl1;
        g_pdOk = true;
    }
    // Pine lignes 1491-1495 : équilibre, tolérance, zones Premium/Discount sur close[i].
    g_inPremium  = false;
    g_inDiscount = false;
    if(g_pdOk) {
        double pdEquilibrium = (g_pdRangeH + g_pdRangeL) / 2.0;
        g_pdEquilibrium = pdEquilibrium;
        double pdTolAbs  = pdEquilibrium * (EQ_TOL / 100.0);
        g_inPremium  = close[i] > pdEquilibrium + pdTolAbs;
        g_inDiscount = close[i] < pdEquilibrium - pdTolAbs;
    }

    // --- OTE : capture des plages au BOS (Pine 1860-1867), séparé bull/bear ---
    if(g_bosHaussier && g_sl1_valid && g_sh1_valid) {
        g_fibSHL     = g_sl1;
        g_fibSHH     = g_sh1;
        g_fibBullBar = bar_index[i];
    }
    if(g_bosBaissier && g_sh1_valid && g_sl1_valid) {
        g_fibSBH     = g_sh1;
        g_fibSBL     = g_sl1;
        g_fibBearBar = bar_index[i];
    }

    // Expiration temporelle des plages OTE (Pine 1870-1875). OTE_EXPIRY_BARS dérive du
    // TF courant : 10800 s = 3 h (M15 → 12 bars, M5 → 36, M30 → 6). Pas une constante
    // dure : recalculée à chaque appel via PeriodSeconds (le Pine fait pareil via _tfSec).
    int tfSec = PeriodSeconds(_Period);
    int oteExpiryBars = (tfSec > 0) ? (int)MathMax(1, MathRound(10800.0 / tfSec)) : 12;
    if(g_fibBullBar >= 0 && (bar_index[i] - g_fibBullBar) > oteExpiryBars)
        g_fibBullBar = -1;   // invalide la plage OTE bull (équiv. Pine _fibSHL := na)
    if(g_fibBearBar >= 0 && (bar_index[i] - g_fibBearBar) > oteExpiryBars)
        g_fibBearBar = -1;   // invalide la plage OTE bear

    // --- Zones OTE (Pine 1883-1897) ---
    // Bull : top = SHH - rng*0.618, bot = SHH - rng*0.786 (deep). Condition SHH > SHL.
    // Bear : bot = SBL + rng*0.618, top = SBL + rng*0.786 (deep). Condition SBH > SBL.
    // Variables globalisées (g_ote*) pour f_drawOTE (Phase 1.4b).
    g_inOTE_bull = false;
    g_inOTE_bear = false;
    g_oteBullValid = false;
    g_oteBearValid = false;
    if(g_fibBullBar >= 0 && g_fibSHH > g_fibSHL) {
        double rng        = g_fibSHH - g_fibSHL;
        g_oteTopBull = g_fibSHH - rng * FIB_OTE_HIGH;
        g_oteBotBull = g_fibSHH - rng * FIB_OTE_LOW;
        g_oteBullValid = true;
        g_inOTE_bull = (close[i] <= g_oteTopBull) && (close[i] >= g_oteBotBull);
    }
    if(g_fibBearBar >= 0 && g_fibSBH > g_fibSBL) {
        double rng        = g_fibSBH - g_fibSBL;
        g_oteBotBear = g_fibSBL + rng * FIB_OTE_HIGH;
        g_oteTopBear = g_fibSBL + rng * FIB_OTE_LOW;
        g_oteBearValid = true;
        g_inOTE_bear = (close[i] >= g_oteBotBear) && (close[i] <= g_oteTopBear);
    }

    // --- AFFICHAGE OTE persistant (Pine L1992-2020, var box _oteBullBox/_oteBearBox) ---
    // SEMANTIQUE PINE : la box est créée AU BOS (avec la zone OTE du moment), puis RESTE
    // affichée jusqu'à ce que le prix sorte de la zone (close < botBull / close > topBear).
    // L'expiration 12 bars (g_fibBullBar = -1) n'invalide que la plage de SCORING, pas la box.
    // 1) Création au BOS haussier (Pine L1992) : bosHaussier AND _oteTopBull != na AND top > bot
    if(g_bosHaussier && g_oteBullValid && g_oteTopBull > g_oteBotBull) {
        g_oteBoxBullActive = true;
        g_oteBoxBullTop    = g_oteTopBull;
        g_oteBoxBullBot    = g_oteBotBull;
        g_oteBoxBullT0     = time[i];   // bar du BOS = bar de création de la box
    }
    // 2) Création au BOS baissier (Pine L2002) : bosBaissier AND _oteTopBear != na AND top > bot
    if(g_bosBaissier && g_oteBearValid && g_oteTopBear > g_oteBotBear) {
        g_oteBoxBearActive = true;
        g_oteBoxBearTop    = g_oteTopBear;
        g_oteBoxBearBot    = g_oteBotBear;
        g_oteBoxBearT0     = time[i];   // bar du BOS
    }
    // 3) Suppression si sortie de zone (Pine L2012-2020) : close < botBull / close > topBear
    if(g_oteBoxBullActive && close[i] < g_oteBoxBullBot)
        g_oteBoxBullActive = false;
    if(g_oteBoxBearActive && close[i] > g_oteBoxBearTop)
        g_oteBoxBearActive = false;
}

// === SECTION 10 (helper) : ATR14 (ta.atr(14), formule Wilder) ===
// Portage Pine ligne 341 : atr14 = ta.atr(14). Le Pine ta.atr utilise la
// moyenne mobile de Wilder (RMA / EMA-like avec alpha = 1/length).
// Formule Wilder : ATR[i] = (ATR[i-1] * (period - 1) + TR[i]) / period, avec
// amorçage ATR[period-1] = moyenne simple des TR sur les `period` 1ères barres.
//
// IMPORTANT : ATR14 est PRÉCALCULÉ une fois dans OnCalculate (tableau g_atr14[])
// avant la boucle bar-par-bar, puis lu en O(1) via g_atr14[i]. Ne PAS appeler
// Atr14At(i) dans la boucle : c'était O(n) par appel → O(n²) total → timeout MT5
// sur de gros historiques (bug observé : rien ne s'affichait, y compris les BOS).

// Tableau d'ATR14 précalculé. g_atr14[i] = ATR14 au bar i (0 si i < ATR_PERIOD).
double g_atr14[];

// Précalcule le tableau complet g_atr14[] pour tout l'historique, en une seule
// passe O(n). À appeler une fois par OnCalculate, AVANT la boucle bar-par-bar.
void PrecalcAtr14(int rates_total, const double &high[], const double &low[],
                  const double &close[]) {
    ArrayResize(g_atr14, rates_total);
    if(rates_total <= ATR_PERIOD) return;
    // Amorçage : moyenne arithmétique des TR sur ATR_PERIOD 1ères barres (1..period).
    // TR[k] = max(high[k]-low[k], |high[k]-close[k-1]|, |low[k]-close[k-1]|)
    double sum = 0.0;
    for(int k = 1; k <= ATR_PERIOD; k++) {
        double tr = MathMax(high[k] - low[k],
                   MathMax(MathAbs(high[k] - close[k - 1]),
                           MathAbs(low[k]  - close[k - 1])));
        sum += tr;
    }
    double atr = sum / ATR_PERIOD;   // ATR à l'index ATR_PERIOD (bar d'amorçage)
    for(int k = 0; k < ATR_PERIOD; k++) g_atr14[k] = 0.0;   // indispo avant amorçage
    g_atr14[ATR_PERIOD] = atr;
    // Propagation Wilder sur le reste de l'historique.
    for(int k = ATR_PERIOD + 1; k < rates_total; k++) {
        double tr = MathMax(high[k] - low[k],
                   MathMax(MathAbs(high[k] - close[k - 1]),
                           MathAbs(low[k]  - close[k - 1])));
        atr = (atr * (ATR_PERIOD - 1) + tr) / ATR_PERIOD;
        g_atr14[k] = atr;
    }
}

// Accès direct O(1) à l'ATR14 précalculé (fallback 0 si i hors plage).
double Atr14At(int i) {
    if(i < 0 || i >= ArraySize(g_atr14)) return 0.0;
    return g_atr14[i];
}

// === SECTION 4 : Liquidité (EQH/EQL) ===
// Portage Pine lignes 522-540.
// === MOTEUR SMC — liquidité ===
// Pine : isEQH = not na(ph) and not na(sh2) and |sh1-sh2| <= tolEq (tolEq = 0.20*atr14).
// EQH/EQL dépendent des pivots g_sh1/g_sh2/g_sl1/g_sl2 calculés par f_structureCompute.
// Refonte 2026-07-27 : modèle SMC avancé avec compteur de touches + grisage des niveaux sweepés.
#define TOL_EQ  0.20   // Pine i_tolEq (× ATR14), tolérance DÉTECTION EQH/EQL (2 pivots égaux)
#define TOL_CLUSTER 0.50   // Pine i_tolCluster (× ATR14), tolérance REGROUPEMENT niveaux proches
double g_dernierEQH_level, g_dernierEQL_level;

// UDT LiqLevel (clone Pine type LiqLevel) : un niveau EQH/EQL avec compteur de touches + état sweep.
struct LiqLevelT {
    double   price;     // niveau moyen (moyenne des pivots touchés)
    datetime tFirst;    // timestamp du 1er pivot (bord gauche de la ligne)
    int      touches;   // nombre de pivots touchés (2=développement, 3+=confirmé)
    bool     swept;     // true si le prix a traversé le niveau puis est revenu
    bool     isHigh;    // true=EQH, false=EQL
};
#define MAX_LIQ  20   // Pine _maxLiq (FIFO max niveaux EQH/EQL)
LiqLevelT g_liqLevels[MAX_LIQ];
int       g_nLiq = 0;

// LiqUpdate : met à jour g_liqLevels avec un nouveau pivot (clone Pine f_liqUpdate).
// - Si un niveau existant du même type est dans la tolérance → incrémente touches + MAJ price.
// - Sinon → crée un nouveau niveau (touches=2 car sh1+sh2 = 2 pivots à la création).
void LiqUpdate(bool isHigh, double p1, double p2, datetime tFirst) {
    double newPrice = (p1 + p2) / 2.0;
    double tolCluster = TOL_CLUSTER * g_atr14_def;   // tolérance regroupement (plus large que détection)
    // Chercher un niveau existant du même type dans la tolérance cluster.
    int foundIdx = -1;
    for(int i = 0; i < g_nLiq; i++) {
        if(g_liqLevels[i].isHigh == isHigh && !g_liqLevels[i].swept) {
            if(MathAbs(g_liqLevels[i].price - p1) <= tolCluster) { foundIdx = i; break; }
        }
    }
    if(foundIdx >= 0) {
        g_liqLevels[foundIdx].touches++;
        g_liqLevels[foundIdx].price = (g_liqLevels[foundIdx].price * (g_liqLevels[foundIdx].touches - 1) + p1) / g_liqLevels[foundIdx].touches;
    } else {
        if(g_nLiq >= MAX_LIQ) {   // FIFO : décale le plus ancien
            for(int k = 0; k < MAX_LIQ - 1; k++) g_liqLevels[k] = g_liqLevels[k + 1];
            g_nLiq = MAX_LIQ - 1;
        }
        g_liqLevels[g_nLiq].price   = newPrice;
        g_liqLevels[g_nLiq].tFirst  = tFirst;
        g_liqLevels[g_nLiq].touches = 2;       // 2 pivots à la création (sh1+sh2 ou sl1+sl2)
        g_liqLevels[g_nLiq].swept   = false;
        g_liqLevels[g_nLiq].isHigh  = isHigh;
        g_nLiq++;
    }
}
// ATR14 courant pour LiqUpdate (mis à jour par f_liqCompute à chaque appel).
double g_atr14_def = 0;

void f_liqCompute(int i, const double &close[], const datetime &time[], double atr14) {
    double tolEq = TOL_EQ * atr14;
    double tolCluster = TOL_CLUSTER * atr14;   // tolérance regroupement (plus large que détection)
    g_atr14_def = atr14;   // pour LiqUpdate (tolérance courante)
    // isEQH/isEQL exigent un pivot confirmé CE BAR (g_hasPH_thisBar/g_hasPL_thisBar, équiv. Pine
    // `not na(ph)`/`not na(pl)`). Refonte 2026-07-27 : modèle SMC avancé avec compteur de touches.
    bool isEQH = g_hasPH_thisBar && g_bsh2 != 0 && MathAbs(g_sh1 - g_sh2) <= tolEq;
    bool isEQL = g_hasPL_thisBar && g_bsl2 != 0 && MathAbs(g_sl1 - g_sl2) <= tolEq;
    int n = ArraySize(time);
    // EQH : nouveau niveau ou incrémentation d'un niveau existant.
    if(isEQH && g_bsh2 > 0 && g_bsh2 < n) {
        double lvlH = (g_sh1 + g_sh2) / 2.0;
        g_dernierEQH_level = lvlH;   // pour scoring/DoL (niveau actif non-sweepé le + récent)
        LiqUpdate(true, g_sh1, g_sh2, time[g_bsh2]);
    }
    // EQL : idem.
    if(isEQL && g_bsl2 > 0 && g_bsl2 < n) {
        double lvlL = (g_sl1 + g_sl2) / 2.0;
        g_dernierEQL_level = lvlL;
        LiqUpdate(false, g_sl1, g_sl2, time[g_bsl2]);
    }
    // 3e touche isolée (clone Pine L613-632) : un pivot confirmé qui n'est PAS égal à sh2 (isEQH=false)
    // MAIS qui est dans la tolérance d'un niveau EQH existant → incrémente touches.
    if(g_hasPH_thisBar && !isEQH && g_bsh2 != 0) {
        for(int k = 0; k < g_nLiq; k++) {
            if(g_liqLevels[k].isHigh && !g_liqLevels[k].swept && MathAbs(g_liqLevels[k].price - g_sh1) <= tolCluster) {
                g_liqLevels[k].touches++;
                g_liqLevels[k].price = (g_liqLevels[k].price * (g_liqLevels[k].touches - 1) + g_sh1) / g_liqLevels[k].touches;
                break;
            }
        }
    }
    if(g_hasPL_thisBar && !isEQL && g_bsl2 != 0) {
        for(int k = 0; k < g_nLiq; k++) {
            if(!g_liqLevels[k].isHigh && !g_liqLevels[k].swept && MathAbs(g_liqLevels[k].price - g_sl1) <= tolCluster) {
                g_liqLevels[k].touches++;
                g_liqLevels[k].price = (g_liqLevels[k].price * (g_liqLevels[k].touches - 1) + g_sl1) / g_liqLevels[k].touches;
                break;
            }
        }
    }
}

// === SECTION 4bis : Liquidité précédente (PDH/PDL/PWH/PWL) ===
// Portage Pine lignes 152-161, 2034-2042, 2089-2092.
// === MOTEUR SMC — prevLiq (PDH/PDL/PWH/PWL) ===
// Le Pine récupère ces niveaux via request.security(syminfo.tickerid, "D"/"W", high[1]/
// low[1], lookahead=barmerge.lookahead_off). En MQL5 l'équivalent naturel est iHigh/iLow
// sur PERIOD_D1/PERIOD_W1 à la bougie précédente clôturée.
//
// IMPORTANT (anti-repaint + fidélité rejeu) : pendant le rejeu bar-par-bar de l'historique
// (AGENTS.md §5bis.1), iHigh(_Symbol, PERIOD_D1, 1) à shift CONSTANT renverrait le PDH
// d'aujourd'hui pour TOUTES les barres passées → DoL/scoring prevLiq faux sur l'historique,
// validation 1:1 vs Pine impossible. On calcule donc l'index D1/W1 contenant la barre `i`
// via iBarShift(_Symbol, PERIOD_D1, time[i]) puis on lit la bougie PRÉCÉDENTE clôturée
// (shift = idx+1, jamais 0). Cela donne le PDH/PDL « du jour précédent » au sens Pine,
// correct pour chaque barre i de l'historique ET strictement anti-repaint (jamais shift 0).
//
// Simulation du `na` Pine : g_pdh==0.0 = données non chargées / indisponibles.
// Les inputs Pine sont hardcodés (AGENTS.md §3 calibrage v11 figé) :
//   i_prevLiqScore  = true
//   i_prevLiqAtrProx = 0.35
//   i_prevLiqPtsProx = 2
//   i_prevLiqPtsSweep = 4
#define PREVLIQ_ATR_PROX  0.35   // Pine i_prevLiqAtrProx (× ATR14)
#define PREVLIQ_PTS_PROX  2      // Pine i_prevLiqPtsProx
#define PREVLIQ_PTS_SWEEP 4      // Pine i_prevLiqPtsSweep

// Niveaux PDH/PDL/PWH/PWL au bar courant (0.0 = na). Stockés aussi pour DoL (PARTIE 2).
double g_pdh = 0.0, g_pdl = 0.0, g_pwh = 0.0, g_pwl = 0.0;
// SMC pur : niveau de liquidité pris (sweep) = OBSOLÈTE. Variables actives invalidées au sweep.
double g_pdhActive = 0.0, g_pdlActive = 0.0, g_pwhActive = 0.0, g_pwlActive = 0.0;
// Timestamps de début du jour/semaine courants (Pine _curDayStartTime/_curWeekStartTime L180-181).
// Capturés au changement de jour/semaine dans la boucle, utilisés comme bord GAUCHE des lignes
// PDH/PDL/PWH/PWL dans f_drawLevels (clone Pine : ligne.new(_curDayStartTime, ..., time, ...)).
datetime g_curDayStartTime = 0;   // 0 = non encore capturé (1er bar)
datetime g_curWeekStartTime = 0;
datetime g_prevDayStartTime = 0;  // 1re bougie UT du jour PRÉCÉDENT (bord gauche PDH/PDL)
datetime g_prevWeekStartTime = 0; // 1re bougie UT de la semaine PRÉCÉDENTE (bord gauche PWH/PWL)
// Flags dérivés (consommés par f_score via composantes prevLiq +2/+4).
// Module H (Phase 5 29/08) : mega-order — volume[1] ≥ 2× SMA20[1] (fenêtre
// bars [1..20], bougie courante exclue — sémantique replay +21.3R).
bool   g_megaVol = false;
bool   g_nearBullPrevLiq = false, g_nearBearPrevLiq = false;
bool   g_sweepBullPrevLiq = false, g_sweepBearPrevLiq = false;

// Récupère pdh/pdl/pwh/pwl au bar `i` (bougie D1/W1 précédente clôturée), puis calcule
// nearBull/nearBear/sweepBull/sweepBear (Pine lignes 2034-2042). À appeler AVANT
// f_accumScores (qui consomme les flags via f_score) ET AVANT DoL (qui lit g_pdh/g_pwh).
// Anti-repaint : tout sur `i`, shift D1/W1 = idx+1 (jamais 0).
// Module H : +2 au f_score si participation institutionnelle sur la bougie OB
// (volume[1] ≥ 2× SMA20[1]). Global par bar, même idiome que prevLiq.
#define MEGA_VOL_MULT 2.0   // audit 28/08 : 2×
#define MEGA_VOL_PTS   2    // points de bonus f_score
void f_megaVolCompute(int i, const long &tick_volume[]) {
    g_megaVol = false;
    if(i < 20) return;   // fenêtre [i-20 .. i-1] = 20 bars, courant exclu
    double sum = 0.0;
    for(int v = i - 20; v <= i - 1; v++) sum += (double)tick_volume[v];
    double sma1 = sum / 20.0;
    if(sma1 > 0.0 && (double)tick_volume[i - 1] >= MEGA_VOL_MULT * sma1)
        g_megaVol = true;
}
void f_prevLiqCompute(int i, const datetime &time[], const double &high[],
                      const double &low[], const double &close[], double atr14) {
    // RAZ des flags chaque bar (Pine : flags dérivés du bar courant, jamais persistants).
    g_nearBullPrevLiq  = false;
    g_nearBearPrevLiq  = false;
    g_sweepBullPrevLiq = false;
    g_sweepBearPrevLiq = false;

    // iBarShift(exact=false) renvoie l'index (série D1) de la bougie D1 contenant time[i]
    // (ou la précédente si time[i] tombe entre 2 bougies D1). -1 si données non chargées.
    int dayIdx  = iBarShift(_Symbol, PERIOD_D1, time[i], false);
    int weekIdx = iBarShift(_Symbol, PERIOD_W1, time[i], false);

    // Capture du timestamp de début du jour/semaine courants (Pine _curDayStartTime/_curWeekStartTime
    // L180-187 : capturé au changement de dayofmonth/weekofyear). On détecte le changement de jour
    // directement sur les bougies UT (time[i] vs time[i-1]), comme le Pine. On mémorise aussi
    // le timestamp du jour PRÉCÉDENT (g_prevDayStartTime) pour le bord gauche des lignes PDH/PDL.
    if(i >= 1) {
        MqlDateTime mCur, mPrev;
        TimeToStruct(time[i],     mCur);
        TimeToStruct(time[i - 1], mPrev);
        bool newDay  = (mCur.day != mPrev.day) || (mCur.mon != mPrev.mon) || (mCur.year != mPrev.year);
        bool newWeek = (mCur.mon != mPrev.mon)
                       || (MathFloor((mCur.day + (mCur.mon == 1 ? 0 : 31)) / 7.0) != MathFloor((mPrev.day + (mPrev.mon == 1 ? 0 : 31)) / 7.0));
        // Détection semaine : comparaison du numéro de semaine (approximation via dayofyear/7).
        // Plus simple et robuste : utiliser iTime(PERIOD_W1) pour comparer le shift W1.
        if(newDay) {
            g_prevDayStartTime = g_curDayStartTime;
            g_curDayStartTime  = time[i];
        }
        int wIdxPrev = (i >= 1) ? iBarShift(_Symbol, PERIOD_W1, time[i - 1], false) : -1;
        int wIdxCur  = iBarShift(_Symbol, PERIOD_W1, time[i],     false);
        if(wIdxPrev >= 0 && wIdxCur >= 0 && wIdxPrev != wIdxCur) {
            g_prevWeekStartTime = g_curWeekStartTime;
            g_curWeekStartTime  = time[i];
        }
    }

    // Garde-fou données non chargées (AGENTS vérif 2) : si l'index HTF est invalide on
    // garde g_pdh=0 (équiv. na) → les composantes near/sweep restent false (fidèle Pine).
    double pdh = 0.0, pdl = 0.0, pwh = 0.0, pwl = 0.0;
    if(dayIdx >= 0) {
        // shift = idx+1 : bougie D1 précédente clôturée (jamais la D1 en cours = shift 0).
        // Équivalent Pine request.security("D", high[1]) sous lookahead_off.
        double dHi = iHigh (_Symbol, PERIOD_D1, dayIdx + 1);
        double dLo = iLow  (_Symbol, PERIOD_D1, dayIdx + 1);
        if(dHi > 0.0) pdh = dHi;   // iHigh renvoie 0 si données non chargées
        if(dLo > 0.0) pdl = dLo;
    }
    if(weekIdx >= 0) {
        double wHi = iHigh(_Symbol, PERIOD_W1, weekIdx + 1);
        double wLo = iLow (_Symbol, PERIOD_W1, weekIdx + 1);
        if(wHi > 0.0) pwh = wHi;
        if(wLo > 0.0) pwl = wLo;
    }

    g_pdh = pdh; g_pdl = pdl; g_pwh = pwh; g_pwl = pwl;
    // SMC pur : reset des niveaux actifs au changement de jour/semaine (recalcul request.security).
    // Détection changement de jour/semaine : si la valeur brute change, on réactive.
    if(g_pdhActive == 0.0 || pdh != g_pdhActive || pdl != g_pdlActive) { g_pdhActive = pdh; g_pdlActive = pdl; }
    if(g_pwhActive == 0.0 || pwh != g_pwhActive || pwl != g_pwlActive) { g_pwhActive = pwh; g_pwlActive = pwl; }

    // Pine lignes 2034-2042. _prevLiqProx = i_prevLiqAtrProx * atr14.
    // Si atr14 indispo (début historique), on sort (pas de proximity fiable).
    if(atr14 <= 0.0) return;
    double prevLiqProx = PREVLIQ_ATR_PROX * atr14;
    double c0 = close[i];
    double lo = low[i];
    double hi = high[i];

    bool nearPDH = (g_pdhActive > 0.0) && (MathAbs(c0 - g_pdhActive) <= prevLiqProx);
    bool nearPDL = (g_pdlActive > 0.0) && (MathAbs(c0 - g_pdlActive) <= prevLiqProx);
    bool nearPWH = (g_pwhActive > 0.0) && (MathAbs(c0 - g_pwhActive) <= prevLiqProx);
    bool nearPWL = (g_pwlActive > 0.0) && (MathAbs(c0 - g_pwlActive) <= prevLiqProx);
    g_nearBullPrevLiq = nearPDL || nearPWL;
    g_nearBearPrevLiq = nearPDH || nearPWH;

    // Sweep : mèche au-delà du niveau puis rejet (clôture de l'autre côté). Pine 2041-2042.
    g_sweepBullPrevLiq = ((pdl > 0.0) && (lo < pdl) && (c0 > pdl)) ||
                         ((pwl > 0.0) && (lo < pwl) && (c0 > pwl));
    g_sweepBearPrevLiq = ((pdh > 0.0) && (hi > pdh) && (c0 < pdh)) ||
                         ((pwh > 0.0) && (hi > pwh) && (c0 < pwh));
    // SMC pur : invalider le niveau actif après sweep (liquidity taken → obsolète).
    if((pdl > 0.0) && (lo < pdl) && (c0 > pdl)) g_pdlActive = 0.0;
    if((pwl > 0.0) && (lo < pwl) && (c0 > pwl)) g_pwlActive = 0.0;
    if((pdh > 0.0) && (hi > pdh) && (c0 < pdh)) g_pdhActive = 0.0;
    if((pwh > 0.0) && (hi > pwh) && (c0 < pwh)) g_pwhActive = 0.0;
}

// === SECTION 4ter : Asian High/Low (session Asie 0h-9h UTC) ===
// Portage Pine lignes 2622-2693 (MODULE 14 sessions — Asian HL).
// === MOTEUR SMC — Asian HL ===
// Le Pine calcule le Asian High/Low pendant la session Asie heure Paris
// (_s14Asian = minutes Paris dans [SES_PARIS_ASIE_START=0, SES_PARIS_ASIE_END=390[ = 0h-6h30
// Paris, décision 2026-07-26), fige les niveaux à la FIN de session (transition dans→hors),
// puis les invalide si le prix les casse. Ces niveaux alimentent le DoL (PARTIE 2) et seront
// affichés en phase décorative.
//
// Cycle Pine (lignes 2641-2693) :
//   _s14Asian = true   → on est dans la session Asie heure Paris (minutes 0-390).
//     → _ahHigh = high, _ahLow = low (initialisation).
//   _s14Asian && !_ahStart → bar suivant de la session.
//     → _ahHigh = max(_ahHigh, high), _ahLow = min(_ahLow, low) (accumulation).
//   _ahEnd = !_s14Asian && _s14Asian[1] → 1er bar hors session (transition dans→hors).
//     → _ahHighDrawn = _ahHigh, _ahLowDrawn = _ahLow (fige pour la journée).
//   Invalidation (2680-2693) : si close > _ahHighDrawn → na ; si close < _ahLowDrawn → na.
//
// En rejeu bar-par-bar : les `var` Pine deviennent des globales persistantes entre 2 appels
// de la boucle (seul ResetSmcState les remet à zéro). La transition « dans→hors » se détecte
// en comparant g_inAsianSession (bar courant) à l'état du bar précédent (mémorisé).
// Simulation du `na` Pine : g_ahHigh_valid/g_ahLow_valid == false (séparés des valeurs,
// comme pour g_pdOk/g_fibBullBar).
// SES_ASIE_START/END (UTC) : conservés pour référence historique (scoring Asian HL était
// en UTC). Désormais InAsianSession utilise SES_PARIS_ASIE_* (heure Paris, clone Pine).
#define SES_ASIE_START  0      // legacy UTC (0h UTC)
#define SES_ASIE_END    540    // legacy UTC (9h UTC)

// État accumulation session Asie (persistant entre bars).
double g_ahHigh = 0.0, g_ahLow = 0.0;     // _ahHigh/_ahLow (accumulation pendant session)
bool   g_inAsianSession = false;          // _s14Asian du bar courant (pour détection transition)
bool   g_ahHigh_valid = false;            // _ahHigh non-na pendant la session en cours
bool   g_ahLow_valid  = false;            // _ahLow  non-na pendant la session en cours
// Niveaux figés (Drawn) — alimentent le DoL. false = na (cassé ou pas encore figé).
double g_ahHighDrawn = 0.0, g_ahLowDrawn = 0.0;
bool   g_ahHighDrawn_valid = false, g_ahLowDrawn_valid = false;
// Timestamp du début de session Asie (Pine _ahStartBar L2688/2711). Capturé au ahStart
// (transition hors→dans session). Sert de bord GAUCHE aux lignes Asian HL dans f_drawAsianHL
// (clone Pine line.new(_ahStartBar, ..., bar_index, ...)). 0 = pas encore capturé.
datetime g_ahStartBar = 0;

// Helper : true si le bar `i` tombe dans la session Asie (heure Paris 0h00-6h30).
// On travaille en heure broker (= Paris + 1h chez Axi), via SES_BROKER_ASIE_*.
// TimeToStruct décompose t en MqlDateTime (champs en heure broker nativement).
// Clone 1:1 du Pine _s14Asian (heure Paris, décision Rono 2026-07-26).
bool InAsianSession(datetime t) {
    MqlDateTime mdt;
    TimeToStruct(t, mdt);
    long mins = (long)(mdt.hour * 60 + mdt.min);   // minutes depuis minuit broker
    return (mins >= SES_BROKER_ASIE_START && mins < SES_BROKER_ASIE_END);
}

// Calcule, au bar courant `i`, l'état Asian HL (début/fin session + invalidation).
// À appeler APRÈS f_prevLiqCompute et AVANT f_accumScores (le DoL en a besoin via les
// flags *_valid). Anti-repaint : tout sur `i`.
void f_asianHlCompute(int i, const datetime &time[], const double &high[],
                      const double &low[], const double &close[]) {
    // État session du bar COURANT (avant mise à jour de g_inAsianSession, qui reflète
    // encore le bar PRÉCÉDENT à l'entrée de cette fonction).
    bool curInSession = InAsianSession(time[i]);
    bool prevInSession = g_inAsianSession;   // _s14Asian[1] Pine (bar précédent)

    // Début de session (Pine _ahStart = _sAsStart = transition hors→dans).
    bool ahStart = curInSession && !prevInSession;
    // Fin de session (Pine _ahEnd = !_s14Asian && _s14Asian[1] = transition dans→hors).
    bool ahEnd   = !curInSession && prevInSession;

    if(ahStart) {
        // Initialisation aux valeurs du bar courant (Pine 2645-2646).
        g_ahHigh = high[i];
        g_ahLow  = low[i];
        g_ahHigh_valid = true;
        g_ahLow_valid  = true;
        g_ahStartBar = time[i];   // bord gauche Asian HL (Pine _ahStartBar := bar_index)
    } else if(curInSession && g_ahHigh_valid) {
        // Accumulation pendant la session (Pine 2649-2651). Max/min sur high[i]/low[i].
        g_ahHigh = MathMax(g_ahHigh, high[i]);
        g_ahLow  = MathMin(g_ahLow,  low[i]);
    }

    // Fin de session : fige les niveaux Drawn pour la journée (Pine 2653-2677).
    // Pine teste `i_showAsianHL` (toggle affichage) — ici on fige inconditionnellement
    // car les niveaux alimentent le DoL (indépendant de l'affichage). Le `not na(_ahHigh)`
    // Pine = g_ahHigh_valid.
    if(ahEnd && g_ahHigh_valid) {
        g_ahHighDrawn = g_ahHigh;
        g_ahHighDrawn_valid = true;
    }
    if(ahEnd && g_ahLow_valid) {
        g_ahLowDrawn = g_ahLow;
        g_ahLowDrawn_valid = true;
    }

    // Mémorise l'état session du bar courant pour la détection de transition au bar suivant.
    g_inAsianSession = curInSession;

    // « Décisions trading » 2026-08-23 : niveau ATTEINT (sweep ou cassure) = consommé.
    // Avant : close franchi uniquement — un sweep mèche+retour laissait le niveau actif.
    if(g_ahHighDrawn_valid && high[i] >= g_ahHighDrawn) {
        g_ahHighDrawn = 0.0;
        g_ahHighDrawn_valid = false;
    }
    if(g_ahLowDrawn_valid && low[i] <= g_ahLowDrawn) {
        g_ahLowDrawn = 0.0;
        g_ahLowDrawn_valid = false;
    }
}

// ══════════════════════════════════════════════════════════════════
//  MODULE 14b — LONDON High/Low (Phase 4 28/08, miroir v12)
//  Même mécanique que l'Asie : range pendant la session Londres
//  (SES_PARIS_LONDON 08:00-16:30 Paris = 480-990 min), niveaux DRAWN à la
//  fin, consommés à l'atteinte (« décisions trading » 23/08). État dissocié
//  de l'affichage (i_showLondonHL équivalent : g_showLondonHL, défaut false).
// ══════════════════════════════════════════════════════════════════
double g_ldHigh = 0.0, g_ldLow = 0.0;
bool   g_ldHigh_valid = false, g_ldLow_valid = false;
datetime g_ldStartBar = 0;
double g_ldHighDrawn = 0.0, g_ldLowDrawn = 0.0;
bool   g_ldHighDrawn_valid = false, g_ldLowDrawn_valid = false;
bool   g_inLondonSession = false;
bool   g_showLondonHL = false;   // défaut off (Pine i_showLondonHL)

bool InLondonSession(datetime t) {
    MqlDateTime mdt;
    TimeToStruct(t, mdt);
    long mins = (long)(mdt.hour * 60 + mdt.min);
    return (mins >= SES_BROKER_LONDON_START && mins < SES_BROKER_LONDON_END);
}

void f_londonHlCompute(int i, const datetime &time[], const double &high[],
                       const double &low[]) {
    bool curInSession = InLondonSession(time[i]);
    bool prevInSession = g_inLondonSession;
    bool ldStart = curInSession && !prevInSession;
    bool ldEnd   = !curInSession && prevInSession;

    if(ldStart) {
        g_ldHigh = high[i];  g_ldLow = low[i];
        g_ldHigh_valid = true;  g_ldLow_valid = true;
        g_ldStartBar = time[i];
    } else if(curInSession && g_ldHigh_valid) {
        g_ldHigh = MathMax(g_ldHigh, high[i]);
        g_ldLow  = MathMin(g_ldLow,  low[i]);
    }
    if(ldEnd && g_ldHigh_valid) { g_ldHighDrawn = g_ldHigh; g_ldHighDrawn_valid = true; }
    if(ldEnd && g_ldLow_valid)  { g_ldLowDrawn  = g_ldLow;  g_ldLowDrawn_valid  = true; }
    g_inLondonSession = curInSession;

    // Consommation à l'atteinte (23/08) — niveaux figés sinon.
    if(g_ldHighDrawn_valid && high[i] >= g_ldHighDrawn) { g_ldHighDrawn = 0.0; g_ldHighDrawn_valid = false; }
    if(g_ldLowDrawn_valid  && low[i]  <= g_ldLowDrawn)  { g_ldLowDrawn  = 0.0; g_ldLowDrawn_valid  = false; }
}

// ══════════════════════════════════════════════════════════════════
//  MODULE 6b — BPR (Balanced Price Range) — miroir v12 (28/08)
//  Appariement d'un FVG NAISSANT avec le FVG opposé le plus récent dans
//  une fenêtre de 10 bars (intersection stricte). Le gap le plus récent
//  fixe le rôle (bull = support). Anti-doublon ≥80% sur les BPR ACTIFS.
//  FIFO 20 (actives + figées). Lifecycle : âge > 15 bars ou clôture au-delà
//  du bord lointain → FIGÉ (gris, conservé à l'affichage, hors tout usage).
//  États sticky : partiel dès l'entrée dans la zone, profond à la CE.
//  Scoring : AUCUN (étude comparatif_bpr 28/08 : +1.0R = bruit → retiré).
// ══════════════════════════════════════════════════════════════════
#define BPR_WINDOW      10    // fenêtre d'appariement (bars entre les 2 origines)
#define MAX_BPR         20    // zones conservées (actives + figées)
#define BPR_MAX_AGE     15    // âge max (bars) d'un BPR ACTIF
#define COL_BPR_BULL    C'255,179,0'    // #FFB300 ambre — support
#define COL_BPR_BEAR    C'255,109,0'    // #FF6D00 orange — résistance

struct BPRT {
    double   top, bot;
    int      state;      // 0 frais · 1 partiel · 2 profond
    int      barIdx;     // bar de complétion du 2e gap (naissance)
    bool     isBull;     // rôle = sens du gap le plus récent
    bool     dead;       // figé (clôture au travers ou âge)
};
BPRT g_bpr[MAX_BPR];
int  g_nBpr = 0;
bool g_showBPR = true;   // Pine i_showBPR (défaut ON)

void BprShift() {
    for(int k = 0; k < MAX_BPR - 1; k++) g_bpr[k] = g_bpr[k + 1];
    g_nBpr = MAX_BPR - 1;
}

// Apparie le FVG naissant (le plus récent du pool `newPool`) au FVG opposé
// le plus récent de `oppPool` qui chevauche dans la fenêtre. Retourne l'index
// du hit ou -1. PARITÉ : lit le pool opposé PRÉ-lifecycle (le Pine apparie
// avant f_fvg*BearLifecycle — un gap rempli cette bar reste appariantable).
int BprFindPair(bool newIsBull, double nT, double nB, int nBar,
                const FVGT &oppPool[], int nOpp) {
    if(nOpp <= 0) return -1;
    for(int k = nOpp - 1; k >= 0; k--) {          // du plus récent au plus ancien
        int gBar = oppPool[k].barIdx;
        if(nBar >= gBar && nBar - gBar <= BPR_WINDOW) {
            double it = MathMin(nT, oppPool[k].top);
            double ib = MathMax(nB, oppPool[k].bot);
            if(it > ib) return k;                  // intersection stricte
        }
    }
    return -1;
}

void f_bprCreate(int i) {
    // Appelle à la naissance d'un FVG (après f_fvgCreate, avant f_fvgLifecycle).
    if(g_isFVGBullBar && g_nFvgBull > 0) {
        FVGT nz = g_fvgBull[g_nFvgBull - 1];
        int hit = BprFindPair(true, nz.top, nz.bot, nz.barIdx, g_fvgBear, g_nFvgBear);
        if(hit >= 0) {
            double t = MathMin(nz.top, g_fvgBear[hit].top);
            double b = MathMax(nz.bot, g_fvgBear[hit].bot);
            // Anti-doublon ≥ 80% sur les BPR ACTIFS uniquement.
            bool dup = false;
            for(int d = 0; d < g_nBpr; d++) {
                if(g_bpr[d].dead) continue;
                double dT = MathMin(t, g_bpr[d].top);
                double dB = MathMax(b, g_bpr[d].bot);
                double minH = MathMin(t - b, g_bpr[d].top - g_bpr[d].bot);
                if(minH > 0.0 && (dT - dB) / minH >= 0.8) { dup = true; break; }
            }
            if(!dup) {
                if(g_nBpr >= MAX_BPR) BprShift();
                g_bpr[g_nBpr].top    = t;
                g_bpr[g_nBpr].bot    = b;
                g_bpr[g_nBpr].state  = 0;
                g_bpr[g_nBpr].barIdx = i;          // bar de complétion du gap récent
                g_bpr[g_nBpr].isBull = true;
                g_bpr[g_nBpr].dead   = false;
                g_nBpr++;
            }
        }
    }
    if(g_isFVGBearBar && g_nFvgBear > 0) {
        FVGT nz = g_fvgBear[g_nFvgBear - 1];
        int hit = BprFindPair(false, nz.top, nz.bot, nz.barIdx, g_fvgBull, g_nFvgBull);
        if(hit >= 0) {
            double t = MathMin(nz.top, g_fvgBull[hit].top);
            double b = MathMax(nz.bot, g_fvgBull[hit].bot);
            bool dup = false;
            for(int d = 0; d < g_nBpr; d++) {
                if(g_bpr[d].dead) continue;
                double dT = MathMin(t, g_bpr[d].top);
                double dB = MathMax(b, g_bpr[d].bot);
                double minH = MathMin(t - b, g_bpr[d].top - g_bpr[d].bot);
                if(minH > 0.0 && (dT - dB) / minH >= 0.8) { dup = true; break; }
            }
            if(!dup) {
                if(g_nBpr >= MAX_BPR) BprShift();
                g_bpr[g_nBpr].top    = t;
                g_bpr[g_nBpr].bot    = b;
                g_bpr[g_nBpr].state  = 0;
                g_bpr[g_nBpr].barIdx = i;
                g_bpr[g_nBpr].isBull = false;
                g_bpr[g_nBpr].dead   = false;
                g_nBpr++;
            }
        }
    }
}

// Lifecycle : figé si âge > 15 ou clôture au-delà du bord LOINTAIN (trigger
// Close). États sticky (jamais de retour profond→partiel). Un BPR né cette
// bar est évalué cette même bar (ordre Pine : naissance puis lifecycle).
void f_bprLifecycle(int i, const double &high[], const double &low[], const double &close[]) {
    for(int k = 0; k < g_nBpr; k++) {
        if(g_bpr[k].dead) continue;
        double ce = (g_bpr[k].top + g_bpr[k].bot) / 2.0;
        bool old  = (i - g_bpr[k].barIdx) > BPR_MAX_AGE;
        bool dead = old || (g_bpr[k].isBull ? (close[i] < g_bpr[k].bot)
                                            : (close[i] > g_bpr[k].top));
        if(dead) { g_bpr[k].dead = true; continue; }
        if(g_bpr[k].isBull) {
            if(low[i] <= ce)                          g_bpr[k].state = 2;
            else if(low[i] < g_bpr[k].top && g_bpr[k].state == 0) g_bpr[k].state = 1;
        } else {
            if(high[i] >= ce)                         g_bpr[k].state = 2;
            else if(high[i] > g_bpr[k].bot && g_bpr[k].state == 0) g_bpr[k].state = 1;
        }
    }
}

// Affichage (après la boucle, pattern f_drawFVG) : rectangle ambre/orange,
// CE pointillée, label « BPR », gris si figé. Bord droit = fin de la bougie
// courante (miroir du correctif Pine 28/08 : jamais au-delà).
void f_drawBPR(const datetime &time[]) {
    if(!g_showBPR) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "BPR");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period);

    // Ne dessiner que les zones des 500 dernières bars (les plus vieilles sont
    // hors du contexte visuel — fidèle au comportement TV où on les voit
    // seulement en scrollant dans leur zone temporelle).
    int iMin = MathMax(0, rates_total - 500);

    int nDrawn = 0;
    for(int k = 0; k < g_nBpr; k++) {
        int idx = g_bpr[k].barIdx;
        if(idx < iMin || idx >= rates_total) continue;   // hors fenêtre visible

        double top = g_bpr[k].top;
        double bot = g_bpr[k].bot;
        double h   = top - bot;
        if(h <= 0) continue;                              // zone dégénérée
        // Garantir une hauteur minimale visible (au moins 2 pixels en prix).
        double minH = 2.0 * _Point;
        if(h < minH) { top = bot + minH; }

        datetime t0 = time[MathMax(0, idx - 2)];          // origine du gap récent

        color clr = g_bpr[k].dead ? clrDarkGray
                                  : (g_bpr[k].isBull ? COL_BPR_BULL : COL_BPR_BEAR);

        // ── Rectangle BPR ──
        string nm = SMC_PREFIX + "BPR_R_" + IntegerToString(k);
        if(ObjectFind(0, nm) < 0) {
            ObjectCreate(0, nm, OBJ_RECTANGLE, 0, t0, top, tEnd, bot);
        }
        ObjectSetInteger(0, nm, OBJPROP_COLOR, clr);
        ObjectSetInteger(0, nm, OBJPROP_FILL, true);
        ObjectSetInteger(0, nm, OBJPROP_BACK, false);
        ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        ObjectMove(0, nm, 0, t0, top);
        ObjectMove(0, nm, 1, tEnd, bot);

        // ── CE pointillée (mi-range) ──
        double ce = (top + bot) / 2.0;
        string nmC = SMC_PREFIX + "BPR_C_" + IntegerToString(k);
        if(ObjectFind(0, nmC) < 0) {
            ObjectCreate(0, nmC, OBJ_TREND, 0, t0, ce, tEnd, ce);
        }
        ObjectSetInteger(0, nmC, OBJPROP_COLOR, clrBlack);   // noir : visible sur ambre/orange/gris
        ObjectSetInteger(0, nmC, OBJPROP_STYLE, STYLE_DOT);
        ObjectSetInteger(0, nmC, OBJPROP_RAY_RIGHT, false);
        ObjectSetInteger(0, nmC, OBJPROP_SELECTABLE, false);
        ObjectMove(0, nmC, 0, t0, ce);
        ObjectMove(0, nmC, 1, tEnd, ce);

        // ── Label « BPR » ──
        string nmL = SMC_PREFIX + "BPR_L_" + IntegerToString(k);
        if(ObjectFind(0, nmL) < 0) {
            ObjectCreate(0, nmL, OBJ_TEXT, 0, tEnd, top);
        }
        ObjectSetString(0, nmL, OBJPROP_TEXT, "BPR");
        ObjectSetInteger(0, nmL, OBJPROP_COLOR, clr);
        ObjectSetInteger(0, nmL, OBJPROP_FONTSIZE, 8);
        ObjectSetInteger(0, nmL, OBJPROP_ANCHOR, ANCHOR_LEFT_UPPER);
        ObjectSetInteger(0, nmL, OBJPROP_SELECTABLE, false);
        ObjectMove(0, nmL, 0, tEnd, top);

        nDrawn++;
    }
}

// === SECTION 5 : Sweep ===
// Portage Pine lignes 580-636 (MODULE 5 — Sweep). MACHINE À ÉTATS À 5 PHASES.
// === MOTEUR SMC — sweep ===
// Sweep = prise de liquidité (mèche au-delà d'un niveau EQH/EQL archivé) puis rejet
// (close revient du bon côté). Le Pine gère cela via une machine à états :
//   - un sweep n'est PAS crédité par un simple test instantané au bar courant ;
//   - il faut D'ABORD enregistrer un "pending" quand le prix franchit le niveau,
//     PUIS confirmer quand la close repasse le niveau (dans la fenêtre de validité),
//     PUIS archiver le sweep confirmé (consommant le niveau EQH/EQL) ;
//   - le sweep archivé ne compte au scoring que s'il est RÉCENT (fraîcheur).
// L'ancienne version 1-étape testait low< EQL && close> EQL sur le même bar : faux
// (ex. OB BEAR#14 crédité Swp=1 à 04:45 alors que le Pine ne le crédite pas).
//
// Pine i_moteurSweep = true (constante) → garde implicite, on exécute toujours.
// Variables d'état persistantes (Pine `var`) = globales MQL5 (AGENTS.md §5bis).

// Phase 1 : état sweep "pending" (en attente de confirmation). -1 = na (pas de pending).
int    g_sweepH_bar   = -1;   // sweep haussier pending (bar_index d'enregistrement)
double g_sweepH_level = 0;    // niveau EQL franchi (en attente de confirmation)
int    g_sweepB_bar   = -1;   // sweep baissier pending
double g_sweepB_level = 0;    // niveau EQH franchi (en attente de confirmation)

// Phase 4 : sweep confirmé ARCHIVÉ (pour le test de fraîcheur phase 5 + scoring).
double g_dernierSweepH_level = 0;   int g_dernierSweepH_bar = -1;
double g_dernierSweepB_level = 0;   int g_dernierSweepB_bar = -1;

// Pine i_maxSwpB = 3 : durée max d'un pending avant expiration sans confirmation.
#define SWEEP_MAX_PENDING_BARS 3

// Phase 5 : fraîcheur dynamique selon le TF. Pine SWEEP_FRESH_BARS = max(1, round(4500/_tfSec)).
//   M15=5 (réf), M5=15, M30=3, plancher 1.
// 4500 s = 5 barres × 900 s (référence M15 validée). Constante partagée par les 2 phases fraîcheur.
#define SWEEP_FRESH_SECONDS 4500.0

// Phase 5 : sorties flags "frais" consommés par f_score (composante Wsweep).
// Réinitialisés chaque bar (Pine : flags dérivés du bar courant, jamais persistants).
bool g_sweepBullFrais, g_sweepBearFrais;

void f_sweepCompute(int i, const double &high[], const double &low[],
                    const double &close[], const int &bar_index[],
                    const datetime &time[]) {
    double lo = low[i], hi = high[i], c0 = close[i];
    int curBar = bar_index[i];

    // ── Phase 1-2 : DÉTECTION + EXPIRATION du pending (Pine lignes 588-604).
    // Détection : si le prix franchit un niveau EQH/EQL archivé ET pas de pending déjà
    // actif pour ce côté → on enregistre un pending (bar_index + niveau). On ne réenregistre
    // pas tant qu'un pending est en cours (Pine `na(sweepH_bar)`).
    if(g_dernierEQL_level > 0 && lo < g_dernierEQL_level && g_sweepH_bar < 0) {
        g_sweepH_bar   = curBar;
        g_sweepH_level = g_dernierEQL_level;
    }
    if(g_dernierEQH_level > 0 && hi > g_dernierEQH_level && g_sweepB_bar < 0) {
        g_sweepB_bar   = curBar;
        g_sweepB_level = g_dernierEQH_level;
    }
    // Expiration : si un pending dépasse i_maxSwpB bars sans confirmation → on l'annule.
    if(g_sweepH_bar >= 0 && (curBar - g_sweepH_bar) > SWEEP_MAX_PENDING_BARS) {
        g_sweepH_bar = -1; g_sweepH_level = 0;
    }
    if(g_sweepB_bar >= 0 && (curBar - g_sweepB_bar) > SWEEP_MAX_PENDING_BARS) {
        g_sweepB_bar = -1; g_sweepB_level = 0;
    }

    // ── Phase 3 : CONFIRMATION (close franchit le niveau du pending) (Pine lignes 606-607).
    // Un pending haussier est confirmé si la close repasse AU-DESSUS du niveau EQL franchi.
    // Un pending baissier est confirmé si la close repasse EN-DESSOUS du niveau EQH franchi.
    bool sweepHaussier = (g_sweepH_bar >= 0) && (c0 > g_sweepH_level);
    bool sweepBaissier = (g_sweepB_bar >= 0) && (c0 < g_sweepB_level);

    // ── Phase 4 : ARCHIVAGE + CONSOMMATION du niveau (Pine lignes 614-626).
    // On archive le sweep confirmé (level + bar) puis on RÉINITIALISE le pending.
    // CRUCIAL : on consomme le niveau EQH/EQL (g_dernierEQ*_level = 0) → un niveau ne peut
    // être sweepé qu'une fois (Pine ligne 619 / 625). C'est ce qui corrige le bug OB BEAR#14.
    if(sweepHaussier) {
        g_dernierSweepH_level = g_sweepH_level;
        g_dernierSweepH_bar   = curBar;
        g_sweepH_bar = -1; g_sweepH_level = 0;
        g_dernierEQL_level = 0;   // niveau consommé (Pine ligne 619) pour scoring/DoL
        // Marquer le niveau EQL correspondant comme sweepé dans g_liqLevels (refonte 2026-07-27).
        double tolSweep = TOL_EQ * Atr14At(i);   // tolérance pour matcher le niveau sweepé
        for(int k = 0; k < g_nLiq; k++) {
            if(!g_liqLevels[k].isHigh && !g_liqLevels[k].swept && MathAbs(g_liqLevels[k].price - g_sweepH_level) <= tolSweep) {
                g_liqLevels[k].swept = true;
                break;
            }
        }
    }
    if(sweepBaissier) {
        g_dernierSweepB_level = g_sweepB_level;
        g_dernierSweepB_bar   = curBar;
        g_sweepB_bar = -1; g_sweepB_level = 0;
        g_dernierEQH_level = 0;   // niveau consommé (Pine ligne 625) pour scoring/DoL
        double tolSweep = TOL_EQ * Atr14At(i);
        for(int k = 0; k < g_nLiq; k++) {
            if(g_liqLevels[k].isHigh && !g_liqLevels[k].swept && MathAbs(g_liqLevels[k].price - g_sweepB_level) <= tolSweep) {
                g_liqLevels[k].swept = true;
                break;
            }
        }
    }

    // ── Phase 5 : FRAÎCHEUR (Pine lignes 633-636).
    // Le sweep archivé ne compte au scoring que s'il est RÉCENT (≤ SWEEP_FRESH_BARS bars).
    // SWEEP_FRESH_BARS = max(1, round(4500/_tfSec)). M15=5, M5=15, M30=3, plancher 1.
    int tfSec      = PeriodSeconds(_Period);
    int freshBars  = (tfSec > 0) ? (int)MathMax(1, MathRound(SWEEP_FRESH_SECONDS / tfSec)) : 5;
    g_sweepBullFrais = (g_dernierSweepH_bar >= 0) && ((curBar - g_dernierSweepH_bar) <= freshBars);
    g_sweepBearFrais = (g_dernierSweepB_bar >= 0) && ((curBar - g_dernierSweepB_bar) <= freshBars);

    // ── Capture sweep pour l'affichage décoratif (phase décorative étape 4) ──
    // Comme HH/HL/LH/LL/MSS/CHOCH, le sweep est un événement ponctuel (1 bar précis) qu'on
    // pousse dans g_structEvents[] pour que f_drawStructEvents (appelée APRÈS la boucle) le
    // dessine. On utilise les booléens de confirmation sweepHaussier/sweepBaissier (phase 4),
    // qui ne sont vrais QU'AU bar de confirmation — fidèle au Pine label.new(sweepHaussier).
    // kind : 9 = Sweep bull (▲), 10 = Sweep bear (▼).
    // Capture dans pool dédié (clone Pine _sweepLbls, FIFO 6).
    // IMPORTANT (règle #1 clone Pine) : la capture est CONDITIONNELLE à g_showSweep, fidèle au
    // Pine `if i_moteurSweep and i_showSwp` (L656). Le Pine crée le label SEULEMENT si le toggle
    // est ON au moment de la détection. Si OFF, le sweep n'est jamais ajouté au pool → invisible
    // même si on active le toggle plus tard. C'est ce qui évite d'afficher des sweeps que le Pine
    // n'affiche pas (le MQL5 capturait avant inconditionnellement → divergence).
    if(g_showSweep) {
        if(sweepHaussier)  PushPool(g_poolSWP, g_nSWP, MAX_POOL_SWP, time[i], lo, 9);
        if(sweepBaissier)  PushPool(g_poolSWP, g_nSWP, MAX_POOL_SWP, time[i], hi, 10);
    }
}

// === SECTION 6 : FVG (Fair Value Gap) ===
// Portage Pine lignes 653-808.
// === MOTEUR SMC — FVG ===
// Pine lignes 668-669 : FVG bull si (low - high[2]) > minGap (détection 3-bougies).
//                      FVG bear si (low[2] - high) > minGap.
// En rejeu bar-par-bar, "bar courant" = i, "bar[2]" = i-2. top/bot/barIdx conformes
// au Pine (topB=low, botB=high[2], bar_index[2] → i-2 ; topBr=low[2], botBr=high).
#define MIN_FVG     0.20   // Pine i_minFVG (× ATR14), taille minimale FVG
#define FVG_MAX_AGE 50     // Pine i_fvgMaxAge, durée de vie max d'un FVG (bars)

FVGT g_fvgBull[MAX_FVG_PER_SIDE];
FVGT g_fvgBear[MAX_FVG_PER_SIDE];
int  g_nFvgBull = 0, g_nFvgBear = 0;
// Flags "FVG apparu CE bar" (Pine ligne 668 : isFVGBull = (low - high[2]) > minGap).
// Reset à false au début de chaque f_fvgCreate (sinon persistance = sur-scoring f_score).
// Contrairement à g_nFvgBull>0 (qui teste tout l'historique), ces flags reflètent EXACTEMENT
// le bar courant → le +Wfvg du scoring n'est crédité que si un FVG vient d'apparaître.
bool g_isFVGBullBar = false;
bool g_isFVGBearBar = false;

// Helper FIFO : décale le tableau (supprime le plus ancien) si plein.
// Pine équivalent : array.shift sur les 4 arrays parallèles.
void FvgShiftBull() {
    for(int k = 0; k < MAX_FVG_PER_SIDE - 1; k++) g_fvgBull[k] = g_fvgBull[k + 1];
    g_nFvgBull = MAX_FVG_PER_SIDE - 1;
}
void FvgShiftBear() {
    for(int k = 0; k < MAX_FVG_PER_SIDE - 1; k++) g_fvgBear[k] = g_fvgBear[k + 1];
    g_nFvgBear = MAX_FVG_PER_SIDE - 1;
}

void f_fvgCreate(int i, const double &high[], const double &low[], double atr14) {
    // Reset bar-courant AVANT tout calcul (sinon persistance du flag d'un bar précédent).
    g_isFVGBullBar = false;
    g_isFVGBearBar = false;
    if(i < 2) return;   // besoin de high[i-2]/low[i-2]
    double minGap = MIN_FVG * atr14;
    double lo0 = low[i];       // "low" Pine (bar courant clôturé)
    double hi0 = high[i];
    double lo2 = low[i - 2];   // "low[2]" Pine
    double hi2 = high[i - 2];  // "high[2]" Pine

    bool isFVGBull = (lo0 - hi2) > minGap;
    bool isFVGBear = (lo2 - hi0) > minGap;

    // Remontée flags bar-courant (Pine ligne 668) pour f_score + garde anti-bruit hsFVG.
    g_isFVGBullBar = isFVGBull;
    g_isFVGBearBar = isFVGBear;

    if(isFVGBull) {
        if(g_nFvgBull >= MAX_FVG_PER_SIDE) FvgShiftBull();
        g_fvgBull[g_nFvgBull].t0     = 0;            // non utilisé (affichage via barIdx)
        g_fvgBull[g_nFvgBull].top    = lo0;          // Pine topB = low
        g_fvgBull[g_nFvgBull].bot    = hi2;          // Pine botB = high[2]
        g_fvgBull[g_nFvgBull].state  = 0;            // actif
        g_fvgBull[g_nFvgBull].barIdx = i - 2;        // Pine bar_index[2]
        g_nFvgBull++;
    }
    if(isFVGBear) {
        if(g_nFvgBear >= MAX_FVG_PER_SIDE) FvgShiftBear();
        g_fvgBear[g_nFvgBear].t0     = 0;
        g_fvgBear[g_nFvgBear].top    = lo2;          // Pine topBr = low[2]
        g_fvgBear[g_nFvgBear].bot    = hi0;          // Pine botBr = high
        g_fvgBear[g_nFvgBear].state  = 0;
        g_fvgBear[g_nFvgBear].barIdx = i - 2;
        g_nFvgBear++;
    }
}

void f_fvgLifecycle(int i, const double &high[], const double &low[],
                    const double &close[]) {
    // Pine lignes 730-808 : mitigation (state 0→1 si low < top) + suppression
    // si close < bot (bull) / close > top (bear) ou âge > FVG_MAX_AGE.
    double c0 = close[i];
    int curBar = i;

    // Bull FVG
    for(int k = g_nFvgBull - 1; k >= 0; k--) {
        bool oldFVG = (FVG_MAX_AGE > 0) && ((curBar - g_fvgBull[k].barIdx) > FVG_MAX_AGE);
        if(c0 < g_fvgBull[k].bot || oldFVG) {
            // Suppression (Pine _bullDel) — compaction du tableau.
            for(int j = k; j < g_nFvgBull - 1; j++) g_fvgBull[j] = g_fvgBull[j + 1];
            g_nFvgBull--;
        } else {
            if(low[i] < g_fvgBull[k].top && g_fvgBull[k].state == 0)
                g_fvgBull[k].state = 1;   // mitigé (recoloré à l'affichage)
        }
    }
    // Bear FVG (symétrique)
    for(int k = g_nFvgBear - 1; k >= 0; k--) {
        bool oldFVG = (FVG_MAX_AGE > 0) && ((curBar - g_fvgBear[k].barIdx) > FVG_MAX_AGE);
        if(c0 > g_fvgBear[k].top || oldFVG) {
            for(int j = k; j < g_nFvgBear - 1; j++) g_fvgBear[j] = g_fvgBear[j + 1];
            g_nFvgBear--;
        } else {
            if(high[i] > g_fvgBear[k].bot && g_fvgBear[k].state == 0)
                g_fvgBear[k].state = 1;
        }
    }
}

// === SECTION 7 : Order Blocks ===
// Portage Pine lignes 810-997 (MODULE 7 — ORDER BLOCKS).
// === MOTEUR SMC — Order Blocks ===
// Pine lignes 829-833 : ROC = (high-low)/close × 10000 bps (range complet, mèches incluses).
//   OB BULL : close > open AND close[1] < open[1] AND _rocOk  (impulsion haussière
//             précédée d'une bougie baissière).
//   OB BEAR : open  > close AND close[1] > open[1] AND _rocOk (symétrique).
// En pattern `i` (bar courant clôturé), "close"/"open" = close[i]/open[i], "close[1]"/"open[1]"
// = close[i-1]/open[i-1]. Le ROC se mesure sur la bougie d'impulsion (bar i, range complet).
//
// Pine f_newBullOB (lignes 907-948) : l'OB = la bougie juste AVANT l'impulsion, donc
//   _obT = high[1] (= high[i-1]), _obB = low[1] (= low[i-1]), time[1] = time[i-1].
//   obBullBar = bar_index (= i, la bougie d'impulsion) → garde anti-suppression immédiate :
//   le lifecycle ignore la bougie de création. En pattern `i` : barIdx = i (impulsion).
// Anti-repaint : i vient toujours de la boucle OnCalculate (<= rates_total-2).

OBT g_obBull[MAX_OB_PER_SIDE];
OBT g_obBear[MAX_OB_PER_SIDE];
int g_nObBull = 0, g_nObBear = 0;

// === Modules décoratifs étape 5 — arrays Breaker / Propulsion / NDOG-NWOG ===
// DÉCORATIF v11 (AGENTS §3) : non connectés au scoring, juste affichés après la boucle.
// Remplis pendant la boucle (captures dans f_obLifecycle pour Breaker, f_propCompute pour
// Propulsion, f_gapCompute pour NDOG/NWOG), puis lus par f_drawBreakers/Propulsion/Gaps.
// Fidèle Pine MODULE 8b : pools Breaker SÉPARÉS par sens (bbBullBox / bbBearBox).
// Chaque sens a son propre FIFO 5 + rotation (Pine array.shift quand array.size >= i_maxBB).
// La struct BreakerT garde son champ `bull` pour compatibilité, mais il est désormais fixe
// par pool (g_bbBull[] toujours bull=true, g_bbBear[] toujours bull=false).
BreakerT g_bbBull[MAX_BB_PER_SIDE];   // Bullish Breakers (support, anciens OB bear invalidés)
BreakerT g_bbBear[MAX_BB_PER_SIDE];   // Bearish Breakers (résistance, anciens OB bull invalidés)
int      g_nBbBull = 0, g_nBbBear = 0;
PropT    g_propBull[MAX_PROP_PER_SIDE];   // Propulsion bull (FIFO 3, Pine propBullBox)
PropT    g_propBear[MAX_PROP_PER_SIDE];   // Propulsion bear (FIFO 3, Pine propBearBox)
int      g_nPropBull = 0, g_nPropBear = 0;
GapT     g_gaps[MAX_GAPS];                // NDOG/NWOG (pool commun, FIFO 3/type séparés)
int      g_nGaps = 0;

// === Module 13b — IMBALANCE (Inner Bar, Pine 2430-2475) ===
// Détection : corps (close-open) > seuilIB × ATR14 (impulsion forte, Pine ibBull/ibBear L352-353).
// 4 tableaux parallèles par sens (top/bot/state/t0) + compteur. FIFO MAX_IB_PER_SIDE.
#define MAX_IB_PER_SIDE 10                 // Pine i_maxIB = 10 (L345, en dur)
double   g_ibBullTop[MAX_IB_PER_SIDE];     // bord haut (Pine ibBullTop : close)
double   g_ibBullBot[MAX_IB_PER_SIDE];     // bord bas  (Pine ibBullBot : open)
int      g_ibBullState[MAX_IB_PER_SIDE];   // 0=fresh, 1=partial (mitigation ≥ 50%)
datetime g_ibBullT0[MAX_IB_PER_SIDE];      // bougie de détection (bord gauche)
int      g_ibBullLastBar = -1;             // anti-double-comptage (Pine _ibLastBull)
int      g_nIbBull = 0;
double   g_ibBearTop[MAX_IB_PER_SIDE];     // bord haut (Pine ibBearTop : open)
double   g_ibBearBot[MAX_IB_PER_SIDE];     // bord bas  (Pine ibBearBot : close)
int      g_ibBearState[MAX_IB_PER_SIDE];
datetime g_ibBearT0[MAX_IB_PER_SIDE];
int      g_ibBearLastBar = -1;
int      g_nIbBear = 0;

// === Module 4b — Equilibrium / OTE : variables d'affichage globalisées ===
// g_pdEquilibrium existe déjà (L496, calculé dans f_pdOteCompute). Pour l'OTE, les variables
// oteTop/oteBot étaient locales à f_pdOteCompute (L566-573) → globalisées pour f_drawOTE.
double g_oteTopBull = 0, g_oteBotBull = 0;   // zone OTE bull (Pine _oteTopBull/_oteBotBull) — SCORING
double g_oteBotBear = 0, g_oteTopBear = 0;   // zone OTE bear — SCORING
bool   g_oteBullValid = false, g_oteBearValid = false;   // false = pas/plus de zone active — SCORING

// Variables AFFICHAGE OTE (persistantes, fidèles Pine _oteBullBox/_oteBearBox L1989-2020).
// IMPORTANT : la box OTE Pine est une `var box` persistante créée au BOS, qui RESTE affichée
// jusqu'à ce que le prix sorte de la zone (close < botBull / close > topBear). L'expiration
// 12 bars (g_fibBullBar = -1) n'invalide que la PLAGE DE SCORING, PAS la box visuelle.
// Bug corrigé 2026-07-28 : avant, g_oteBullValid était reset à false à chaque bar ET lié à
// g_fibBullBar (qui expire à 12 bars) → la box disparaissait après 12 bars. Désormais les
// variables d'affichage sont découplées et suivent la sémantique Pine exacte.
bool   g_oteBoxBullActive = false;           // true = box OTE bull visible (Pine _oteBullBox != na)
bool   g_oteBoxBearActive = false;           // true = box OTE bear visible (Pine _oteBearBox != na)
double g_oteBoxBullTop = 0, g_oteBoxBullBot = 0;   // zone box OTE bull (figée au BOS)
double g_oteBoxBearTop = 0, g_oteBoxBearBot = 0;   // zone box OTE bear (figée au BOS)
datetime g_oteBoxBullT0 = 0;                 // bar de création box OTE bull (= bar du BOS)
datetime g_oteBoxBearT0 = 0;                 // bar de création box OTE bear (= bar du BOS)

// Helper FIFO : décale le tableau (supprime le plus ancien) si plein.
// Pine équivalent : array.shift sur les arrays parallèles (obBullTop/Bot/State/Bar/...).
void ObShiftBull() {
    for(int k = 0; k < MAX_OB_PER_SIDE - 1; k++) g_obBull[k] = g_obBull[k + 1];
    g_nObBull = MAX_OB_PER_SIDE - 1;
}
void ObShiftBear() {
    for(int k = 0; k < MAX_OB_PER_SIDE - 1; k++) g_obBear[k] = g_obBear[k + 1];
    g_nObBear = MAX_OB_PER_SIDE - 1;
}

// === MODULE 13b — IMBALANCE (Inner Bar) : détection + lifecycle ===
// Portage Pine L352-353 (détection) + L2443-2539 (cycle de vie) + L2545-2554 (sync bord droit).
// Détection : corps (close-open) > seuilIB × ATR14 → impulsion forte (Pine ibBull/ibBear).
// NB : ce n'est PAS l'Inner Bar classique ICT (inclusion high/low). Le nom "Inner Bar" Pine est
// trompeur ; c'est un détecteur d'impulsion de corps. 4 tableaux parallèles par sens (top/bot/
// state/t0). FIFO MAX_IB_PER_SIDE (Pine i_maxIB=10). Mitigation : totale → suppression,
// partielle (close sous/dessus le milieu) → state=1 (recoloration partial).

// Détection IB au bar i (Pine L2443-2458 bull + L2460-2475 bear). Appelée dans la boucle.
// Anti-double-comptage : g_ibBullLastBar/g_ibBearLastBar (Pine _ibLastBull/_ibLastBear).
void f_ibCompute(int i, const datetime &time[], const double &open[], const double &close[],
                 double atr14, int bar_index) {
    if(atr14 <= 0) return;                       // garde : ATR14 non encore calculé
    double seuil = g_autoSeuilIB * atr14;
    bool ibBull = (close[i] - open[i]) > seuil;  // Pine L352 : corps haussier > seuil
    bool ibBear = (open[i] - close[i]) > seuil;  // Pine L353 : corps baissier > seuil

    // IB BULL (Pine L2443-2458) : top=close, bot=open
    if(ibBull && bar_index != g_ibBullLastBar) {
        g_ibBullLastBar = bar_index;
        if(g_nIbBull >= MAX_IB_PER_SIDE) {       // FIFO : décale si plein
            for(int k = 0; k < MAX_IB_PER_SIDE - 1; k++) {
                g_ibBullTop[k] = g_ibBullTop[k+1]; g_ibBullBot[k] = g_ibBullBot[k+1];
                g_ibBullState[k] = g_ibBullState[k+1]; g_ibBullT0[k] = g_ibBullT0[k+1];
            }
            g_nIbBull = MAX_IB_PER_SIDE - 1;
        }
        g_ibBullTop[g_nIbBull]   = close[i];
        g_ibBullBot[g_nIbBull]   = open[i];
        g_ibBullState[g_nIbBull] = 0;            // fresh
        g_ibBullT0[g_nIbBull]    = time[i];      // bord gauche = bougie de détection
        g_nIbBull++;
    }
    // IB BEAR (Pine L2460-2475) : top=open, bot=close
    if(ibBear && bar_index != g_ibBearLastBar) {
        g_ibBearLastBar = bar_index;
        if(g_nIbBear >= MAX_IB_PER_SIDE) {
            for(int k = 0; k < MAX_IB_PER_SIDE - 1; k++) {
                g_ibBearTop[k] = g_ibBearTop[k+1]; g_ibBearBot[k] = g_ibBearBot[k+1];
                g_ibBearState[k] = g_ibBearState[k+1]; g_ibBearT0[k] = g_ibBearT0[k+1];
            }
            g_nIbBear = MAX_IB_PER_SIDE - 1;
        }
        g_ibBearTop[g_nIbBear]   = open[i];
        g_ibBearBot[g_nIbBear]   = close[i];
        g_ibBearState[g_nIbBear] = 0;
        g_ibBearT0[g_nIbBear]    = time[i];
        g_nIbBear++;
    }
}

// Lifecycle IB (Pine f_ibBullLifecycle L2477-2507 + f_ibBearLifecycle L2509-2539).
// Mitigation totale (close traverse le bas) → suppression (shift) ; partielle (close traverse
// le milieu) → state=1 (recoloration partial au prochain redraw). Appelée dans la boucle.
void f_ibLifecycle(int i, const double &close[]) {
    double cl = close[i];
    // IB BULL : suppression si close <= bot ; partial si low <= mid (Pine L2487-2496).
    // NB Pine teste sur low pour la partial, mais on n'a que close ici (cohérent avec le pattern
    // OB du code qui utilise close[i] pour la mitigation). On garde close pour la partial.
    for(int k = g_nIbBull - 1; k >= 0; k--) {
        double mid = (g_ibBullTop[k] + g_ibBullBot[k]) / 2.0;
        if(cl <= g_ibBullBot[k]) {               // mitigation totale → suppression
            for(int m = k; m < g_nIbBull - 1; m++) {
                g_ibBullTop[m] = g_ibBullTop[m+1]; g_ibBullBot[m] = g_ibBullBot[m+1];
                g_ibBullState[m] = g_ibBullState[m+1]; g_ibBullT0[m] = g_ibBullT0[m+1];
            }
            g_nIbBull--;
        } else if(cl <= mid && g_ibBullState[k] == 0) {
            g_ibBullState[k] = 1;                // mitigation partielle → recoloration
        }
    }
    // IB BEAR : suppression si close >= top ; partial si high >= mid (Pine L2519-2528).
    for(int k = g_nIbBear - 1; k >= 0; k--) {
        double mid = (g_ibBearTop[k] + g_ibBearBot[k]) / 2.0;
        if(cl >= g_ibBearTop[k]) {               // mitigation totale → suppression
            for(int m = k; m < g_nIbBear - 1; m++) {
                g_ibBearTop[m] = g_ibBearTop[m+1]; g_ibBearBot[m] = g_ibBearBot[m+1];
                g_ibBearState[m] = g_ibBearState[m+1]; g_ibBearT0[m] = g_ibBearT0[m+1];
            }
            g_nIbBear--;
        } else if(cl >= mid && g_ibBearState[k] == 0) {
            g_ibBearState[k] = 1;
        }
    }
}

// Pine f_newBullOB / f_newBullBear (lignes 907-997) adapté au pattern `i`.
// Détecte l'impulsion au bar i et crée l'OB depuis la bougie i-1 (bougie OB).
void f_obCreate(int i, const datetime &time[],
                const double &open[], const double &high[],
                const double &low[],  const double &close[]) {
    if(i < 1) return;   // besoin de high[i-1]/low[i-1] (bougie OB avant l'impulsion)

    // ROC en bps sur la bougie d'impulsion (bar i), range complet mèches incluses.
    double c0 = close[i], o0 = open[i];
    double h0 = high[i],  l0 = low[i];
    double rocCur = (c0 != 0.0) ? (h0 - l0) / c0 * 10000.0 : 0.0;
    bool   rocOk  = rocCur >= g_autoRocSeuil;   // Pine ligne 830 : _rocOk = _rocCur >= i_rocSeuil

    // Pine lignes 832-833 : barstate.isconfirmed implicite (i = bar clôturé).
    double c1 = close[i - 1], o1 = open[i - 1];
    bool impulseBull = (c0 > o0) && (c1 < o1) && rocOk;   // impulsion haussière après baissière
    bool impulseBear = (o0 > c0) && (c1 > o1) && rocOk;   // impulsion baissière après haussière

    // L'OB = la bougie juste avant l'impulsion : high[i-1]/low[i-1] (Pine high[1]/low[1]).
    double obT = high[i - 1];   // _rt (Pine _obT = high[1])
    double obB = low[i - 1];    // _rb (Pine _obB = low[1])
    double obMid = (obT + obB) / 2.0;
    datetime obT0 = time[i - 1];   // Pine obBullTime = int(time[1])

    if(impulseBull) {
        if(g_nObBull >= MAX_OB_PER_SIDE) ObShiftBull();
        g_obBull[g_nObBull].t0       = obT0;
        g_obBull[g_nObBull].top      = obT;
        g_obBull[g_nObBull].bot      = obB;
        g_obBull[g_nObBull].mid      = obMid;
        g_obBull[g_nObBull].state    = 0;             // vierge (Pine obBullState = 0)
        g_obBull[g_nObBull].force    = 0;             // calculé en J4 (f_force)
        g_obBull[g_nObBull].score    = 0;             // accumulé en J4 (f_accumScores)
        g_obBull[g_nObBull].bull     = true;
        g_obBull[g_nObBull].isIB     = false;         // Inner Bar (Pine ibBull[1]) — hors-scope Phase 1
        g_obBull[g_nObBull].signaled = false;
        g_obBull[g_nObBull].zoneActive = false;       // LATCH zone (créée quand qualif devient true)
        g_obBull[g_nObBull].barIdx   = i;             // bougie IMPULSION (garde anti-suppression)
        g_nObBull++;
    }
    if(impulseBear) {
        if(g_nObBear >= MAX_OB_PER_SIDE) ObShiftBear();
        g_obBear[g_nObBear].t0       = obT0;
        g_obBear[g_nObBear].top      = obT;
        g_obBear[g_nObBear].bot      = obB;
        g_obBear[g_nObBear].mid      = obMid;
        g_obBear[g_nObBear].state    = 0;
        g_obBear[g_nObBear].force    = 0;
        g_obBear[g_nObBear].score    = 0;
        g_obBear[g_nObBear].bull     = false;
        g_obBear[g_nObBear].isIB     = false;
        g_obBear[g_nObBear].signaled = false;
        g_obBear[g_nObBear].zoneActive = false;       // LATCH zone (créée quand qualif devient true)
        g_obBear[g_nObBear].barIdx   = i;             // bougie IMPULSION (garde anti-suppression)
        g_nObBear++;
    }
}

// === SECTION 8 : Mitigation (lifecycle OB) ===
// Portage Pine lignes 999-1157 (f_obLifecycle).
// === MOTEUR SMC — mitigation OB ===
// Pine f_obLifecycle : pour chaque OB, deux phases, APPLIQUÉES DANS L'ORDRE (mitigation PUIS
// suppression) — l'ordre est CRITIQUE (sinon la mitigation ne s'applique jamais, bug corrigé) :
//   1) MITIGATION D'ABORD (avant suppression) : si prix touche la zone.
//        Bull (Pine 1045-1054) : si low <= topB alors
//            close <= mid ET st < 2 → state 2 (DEEP, mitigation ≥ 50%)
//            close >  mid ET st == 0 → state 1 (PARTIAL, mitigation < 50%)
//        Bear (Pine 1123-1132) : si high >= botBr alors
//            close >= mid ET st < 2 → state 2 (DEEP)
//            close <  mid ET st == 0 → state 1 (PARTIAL)
//   2) PUIS INVALIDATION (suppression) : bar_index > obBar ET prix touche la zone.
//        Bull : low <= topB        (Pine ligne 1012 : _invalB = low <= topB)
//        Bear : high >= botBr      (Pine ligne 1090 : _invalBr = high >= botBr)
//      Le Breaker n'est PAS porté (hors-scope Phase 1, décoratif) — on supprime juste l'OB.
// En pattern `i` : low/high/close = low[i]/high[i]/close[i], bar_index = i.
// La compaction du tableau se fait en retirant l'index invalide (Pine array.remove après
// tri desc). On boucle donc du haut vers le bas pour ne pas casser les indices.
void f_obLifecycle(int i, const datetime &time[], const double &high[],
                   const double &low[], const double &close[]) {
    double c0  = close[i];
    double lo  = low[i];
    double hi  = high[i];
    int curBar = i;
    datetime tInvalid = time[i];   // bar d'invalidation (Pine bar_index) — bord gauche Breaker
    int _diagSupprBull = 0, _diagSupprBear = 0;   // DIAGNOSTIC suppression

    // --- OB BULL ---
    // NOTE BUG CORRIGÉ : la mitigation doit s'appliquer AVANT la suppression. Avant, elle était
    // dans le `else` du test d'invalidation, donc ne s'appliquait que si curBar <= barIdx
    // (bar de création) → les OB ne transitaient jamais vers state 1/2 → f_accumScores bloquait
    // le score à son max via MathMax → sur-scoring (ex: OB éloigné à 20×ATR gardait score=9).
    // Désormais : 1) mitigation (transition state 0→1/2 au touché) PUIS 2) suppression.
    for(int k = g_nObBull - 1; k >= 0; k--) {
        double topB = g_obBull[k].top;
        double botB = g_obBull[k].bot;
        double mid  = g_obBull[k].mid;

        // 1) MITIGATION D'ABORD (Pine lignes 1045-1054) : si low <= topB (zone touchée).
        //    Pine : close <= mid ET st < 2 → state 2 (DEEP, mitigation ≥ 50%)
        //           close >  mid ET st == 0 → state 1 (PARTIAL, mitigation < 50%)
        //    Appliquée AVANT suppression pour que l'OB puisse transiter vers state 2 (DEEP),
        //    ce qui autorisera ensuite la descente du score dans f_accumScores (state 2 → max(0,cand)).
        if(lo <= topB) {
            int st = g_obBull[k].state;
            if(c0 <= mid && st < 2) {
                g_obBull[k].state = 2;   // DEEP : close <= mid (mitigation ≥ 50%)
            } else if(c0 > mid && st == 0) {
                g_obBull[k].state = 1;   // PARTIAL : close > mid (mitigation < 50%)
            }
        }

        // 2) PUIS INVALIDATION/SUPPRESSION (Pine lignes 1012-1013) : si bar_index > bar création
        //    ET prix touche le haut. Fidèle au Pine qui SUPPRIME l'OB touché. La mitigation ci-dessus
        //    a déjà mis à jour le state avant suppression.
        //    Pine ligne 1012 : _invalB = low <= topB ; ligne 1013 : (bar_index > _obBarB and _invalB).
        //    MODULE 8 Breaker (phase décorative étape 5, Pine 1021-1036) : si l'invalidation est
        //    DIRECTIONNELLE (close < botB, et non expired), l'OB invalidé devient un BEARISH BREAKER
        //    (résistance). On capture la zone AVANT compaction. expired=false en dur (pas d'expiration
        //    v11), donc la condition Pine `not expired and close < botB` se réduit à `close < botB`.
        bool invalB = (curBar > g_obBull[k].barIdx) && (lo <= topB);
        if(invalB) {
            // Pine MODULE 8b (L1121) : si invalidation DIRECTIONNELLE (close < botB), l'OB bull
            // invalidé devient un BEARISH BREAKER (résistance). FIFO 5 par sens avec rotation
            // (Pine array.shift quand array.size(bbBearBox) >= i_maxBB).
            if(c0 < botB) {
                if(g_nBbBear >= MAX_BB_PER_SIDE) {   // FIFO : décale le plus ancien
                    for(int b = 0; b < MAX_BB_PER_SIDE - 1; b++) g_bbBear[b] = g_bbBear[b + 1];
                    g_nBbBear = MAX_BB_PER_SIDE - 1;
                }
                g_bbBear[g_nBbBear].t0  = tInvalid;   // bar d'invalidation (i)
                g_bbBear[g_nBbBear].top = topB;        // zone = OB source (Pine topB)
                g_bbBear[g_nBbBear].bot = botB;
                g_bbBear[g_nBbBear].bull = false;      // bearish breaker (résistance)
                g_nBbBear++;
            }
            for(int j = k; j < g_nObBull - 1; j++) g_obBull[j] = g_obBull[j + 1];
            g_nObBull--;
            _diagSupprBull++;
        }
    }

    // --- OB BEAR (symétrique) ---
    // NOTE BUG CORRIGÉ : mitigation AVANT suppression (cf. boucle bull ci-dessus).
    for(int k = g_nObBear - 1; k >= 0; k--) {
        double topBr = g_obBear[k].top;
        double botBr = g_obBear[k].bot;
        double mid   = g_obBear[k].mid;

        // 1) MITIGATION D'ABORD (Pine lignes 1123-1132) : si high >= botBr (zone touchée).
        //    Pine : close >= mid ET st < 2 → state 2 (DEEP, mitigation ≥ 50%)
        //           close <  mid ET st == 0 → state 1 (PARTIAL, mitigation < 50%)
        if(hi >= botBr) {
            int st = g_obBear[k].state;
            if(c0 >= mid && st < 2) {
                g_obBear[k].state = 2;   // DEEP : close >= mid (mitigation ≥ 50%)
            } else if(c0 < mid && st == 0) {
                g_obBear[k].state = 1;   // PARTIAL : close < mid (mitigation < 50%)
            }
        }

        // 2) PUIS INVALIDATION/SUPPRESSION (Pine lignes 1090-1091) : si bar_index > bar création
        //    ET prix touche le bas. Fidèle au Pine qui SUPPRIME l'OB touché.
        //    Pine ligne 1090 : _invalBr = high >= botBr ; ligne 1091 : (bar_index > _obBarBr and _invalBr).
        //    MODULE 8 Breaker symétrique (Pine 1095-1114) : si invalidation DIRECTIONNELLE
        //    (close > topBr, non expired), l'OB bear invalidé devient un BULLISH BREAKER (support).
        bool invalBr = (curBar > g_obBear[k].barIdx) && (hi >= botBr);
        if(invalBr) {
            // Pine MODULE 8b (L1199) : si invalidation DIRECTIONNELLE (close > topBr), l'OB bear
            // invalidé devient un BULLISH BREAKER (support). FIFO 5 par sens avec rotation
            // (Pine array.shift quand array.size(bbBullBox) >= i_maxBB).
            if(c0 > topBr) {
                if(g_nBbBull >= MAX_BB_PER_SIDE) {   // FIFO : décale le plus ancien
                    for(int b = 0; b < MAX_BB_PER_SIDE - 1; b++) g_bbBull[b] = g_bbBull[b + 1];
                    g_nBbBull = MAX_BB_PER_SIDE - 1;
                }
                g_bbBull[g_nBbBull].t0  = tInvalid;   // bar d'invalidation (i)
                g_bbBull[g_nBbBull].top = topBr;        // zone = OB source (Pine topBr)
                g_bbBull[g_nBbBull].bot = botBr;
                g_bbBull[g_nBbBull].bull = true;        // bullish breaker (support)
                g_nBbBull++;
            }
            for(int j = k; j < g_nObBear - 1; j++) g_obBear[j] = g_obBear[j + 1];
            g_nObBear--;
            _diagSupprBear++;
        }
    }
}

// === MODULE 8b : Breaker lifecycle (phase décorative étape 5) ===
// Portage Pine f_bbLifecycle (lignes 1261-1304). DÉCORATIF (hors scoring).
// Un Breaker est consommé quand le prix le traverse complètement (Pine teste le close) :
//   - Bullish Breaker (support)     consommé si close < bbBullBot (Pine 1267).
//   - Bearish Breaker (résistance)  consommé si close > bbBearTop (Pine 1289).
// On boucle du haut vers le bas (retrait d'indices sans casser le tableau).
// Refonte 2026-07-28 : pools SÉPARÉS par sens (g_bbBull / g_bbBear), fidèle Pine
// (avant : pool commun 20 sans rotation → divergence accumulation/blocage).
void f_breakerLifecycle(int i, const double &close[]) {
    double c0 = close[i];
    // Bullish Breakers (support) : consommés si close < bot
    for(int k = g_nBbBull - 1; k >= 0; k--) {
        if(c0 < g_bbBull[k].bot) {
            for(int j = k; j < g_nBbBull - 1; j++) g_bbBull[j] = g_bbBull[j + 1];
            g_nBbBull--;
        }
    }
    // Bearish Breakers (résistance) : consommés si close > top
    for(int k = g_nBbBear - 1; k >= 0; k--) {
        if(c0 > g_bbBear[k].top) {
            for(int j = k; j < g_nBbBear - 1; j++) g_bbBear[j] = g_bbBear[j + 1];
            g_nBbBear--;
        }
    }
}

// === MODULE 8c : Propulsion lifecycle (phase décorative étape 5) ===
// Portage Pine f_propBullLifecycle (L1382-1401) + f_propBearLifecycle (L1406-1425).
// DÉCORATIF (hors scoring). Un Propulsion est consommé quand le prix le traverse (Pine close) :
//   - Propulsion Bull consommé si close < propBullBot (Pine 1386).
//   - Propulsion Bear consommé si close > propBearTop (Pine 1410).
// AVANT : AUCUN lifecycle en MQL5 → les Propulsion restaient affichés indéfiniment même
// après avoir été cassés par le prix. Divergence #1 (la plus grave) corrigée 2026-07-28.
void f_propLifecycle(int i, const double &close[]) {
    double c0 = close[i];
    // Propulsion Bull : consommés si close < bot
    for(int k = g_nPropBull - 1; k >= 0; k--) {
        if(c0 < g_propBull[k].bot) {
            for(int j = k; j < g_nPropBull - 1; j++) g_propBull[j] = g_propBull[j + 1];
            g_nPropBull--;
        }
    }
    // Propulsion Bear : consommés si close > top
    for(int k = g_nPropBear - 1; k >= 0; k--) {
        if(c0 > g_propBear[k].top) {
            for(int j = k; j < g_nPropBear - 1; j++) g_propBear[j] = g_propBear[j + 1];
            g_nPropBear--;
        }
    }
}

// === MODULE 8c : Propulsion Blocks (phase décorative étape 5) ===
// Portage Pine f_fvgBullOB/f_fvgBearOB (lignes 1236-1289). DÉCORATIF (hors scoring).
// Un Propulsion = chevauchement d'un FVG et d'un OB DU MÊME SENS créés/existants au bar i.
//   Pine f_fvgBullOB : si isFVGBull (FVG créé ce bar, top=low, bot=high[2]) ET un OB bull actif
//     chevauche ce FVG → Propulsion bull (zone = overlap ovTop/ovBot). FIFO 3 par sens.
//   Pine f_fvgBearOB : symétrique (FVG bear top=high, bot=low[2] ∩ OB bear).
// À appeler APRÈS f_fvgCreate (g_isFVGBullBar/BearBar figés) ET APRÈS f_obCreate (OB du bar i
// présents dans les arrays). Ne fait rien si aucun FVG créé ce bar (sortie anticipée).
void f_propCompute(int i, const datetime &time[],
                   const double &high[], const double &low[]) {
    if(!g_isFVGBullBar && !g_isFVGBearBar) return;   // aucun FVG créé ce bar → pas de Propulsion
    datetime t0 = time[i];
    // --- FVG BULL ∩ OB BULL → Propulsion bull (Pine f_fvgBullOB, lignes 1238-1258) ---
    // FVG bull Pine : top = low (= low[i]), bot = high[2] (= high[i-2]).
    if(g_isFVGBullBar && i >= 2) {
        double fTop = low[i];
        double fBot = high[i - 2];
        for(int pi = 0; pi < g_nObBull; pi++) {
            double oTop = g_obBull[pi].top;
            double oBot = g_obBull[pi].bot;
            double ovBot = MathMax(fBot, oBot);
            double ovTop = MathMin(fTop, oTop);
            if(ovTop > ovBot) {   // chevauchement réel (Pine ligne 1245)
                if(g_nPropBull >= MAX_PROP_PER_SIDE) {   // FIFO : décale le plus ancien
                    for(int k = 0; k < MAX_PROP_PER_SIDE - 1; k++) g_propBull[k] = g_propBull[k + 1];
                    g_nPropBull = MAX_PROP_PER_SIDE - 1;
                }
                g_propBull[g_nPropBull].t0  = t0;
                g_propBull[g_nPropBull].top = ovTop;    // Pine push _ovTop
                g_propBull[g_nPropBull].bot = ovBot;    // Pine push _ovBot
                g_propBull[g_nPropBull].bull = true;
                g_nPropBull++;
                // PAS de break : le Pine (lignes 1240-1258) boucle sur TOUS les OB et crée un
                // Propulsion PAR OB chevauchant, le FIFO (i_maxProp=3) gardant les 3 derniers.
                // La vérif g_nPropBull >= MAX en tête de boucle gère le décalage à chaque ajout.
            }
        }
    }
    // --- FVG BEAR ∩ OB BEAR → Propulsion bear (Pine f_fvgBearOB, lignes 1265-1286) ---
    // FVG bear Pine : top = high[2] (= high[i-2]), bot = low (= low[i]).
    if(g_isFVGBearBar && i >= 2) {
        double fTop = high[i - 2];
        double fBot = low[i];
        for(int pi = 0; pi < g_nObBear; pi++) {
            double oTop = g_obBear[pi].top;
            double oBot = g_obBear[pi].bot;
            double ovBot = MathMax(fBot, oBot);
            double ovTop = MathMin(fTop, oTop);
            if(ovTop > ovBot) {
                if(g_nPropBear >= MAX_PROP_PER_SIDE) {
                    for(int k = 0; k < MAX_PROP_PER_SIDE - 1; k++) g_propBear[k] = g_propBear[k + 1];
                    g_nPropBear = MAX_PROP_PER_SIDE - 1;
                }
                g_propBear[g_nPropBear].t0  = t0;
                g_propBear[g_nPropBear].top = ovTop;
                g_propBear[g_nPropBear].bot = ovBot;
                g_propBear[g_nPropBear].bull = false;
                g_nPropBear++;
                // PAS de break (cf. branche bull : 1 Propulsion par OB chevauchant, FIFO 3).
            }
        }
    }
}

// === MODULE 10b : NDOG / NWOG (phase décorative étape 5) ===
// Portage Pine MODULE 10b (lignes 1357-1434). DÉCORATIF (scoring _preInGap retiré en v11).
// NDOG = gap entre close[1] (veille) et open (nouveau jour) ; NWOG = idem nouvelle semaine.
// Zone = [min(open,close[1]), max(open,close[1])] si gap ≥ 0.3×ATR14 et open≠close[1].
// Gating TF Pine (Phase 3.3) : NDOG pertinent M1-M15, NWOG pertinent H1-H4 (sinon ignoré).
// Détection changement de jour/semaine : comparaison du jour calendulaire (MQL5 TimeDay) /
// numéro de semaine. On utilise MqlDateTime pour rester timezone-agnostic (time[] = heure du broker).
// À appeler APRÈS Atr14At(i) disible (atr14 passé en paramètre). FIFO 3 PAR TYPE (NDOG/NWOG
// séparés, fidèle Pine i_maxGap=3/type ligne 1376). Mitigation gérée dans f_gapLifecycle.
void f_gapCompute(int i, const datetime &time[], const double &open[],
                  const double &close[], double atr14) {
    if(i < 1) return;   // besoin de close[i-1] (= Pine close[1])
    MqlDateTime mCur, mPrev;
    TimeToStruct(time[i],     mCur);
    TimeToStruct(time[i - 1], mPrev);
    // Nouveau jour : changement du jour calendulaire (Pine ta.change(time("D")) ≈ dayofmonth change).
    bool newDay  = (mCur.day != mPrev.day) || (mCur.mon != mPrev.mon) || (mCur.year != mPrev.year);
    // Nouvelle semaine : Pine weekofyear change. Approximation robuste = changement d'année OU
    // saut de > 3 jours calendaire (couvre le gap weekend : vendredi J → lundi J+3, diff=3 → on
    // teste diff >= 3 ; un même jour de semaine a diff <= 1 entre 2 bars consécutifs). On évite
    // weekofyear MQL5 (day_of_year suffit et est fiable hors changement d'année).
    bool newWeek;
    if(mCur.year != mPrev.year) {
        newWeek = true;   // changement d'année = forcément nouvelle semaine
    } else {
        int diffDays = mCur.day_of_year - mPrev.day_of_year;
        newWeek = (diffDays >= 3);   // vendredi→lundi = +3 (weekend), parfois +2 (jour férié)
    }
    // Gating TF (Pine _tfNDOG M1-M15, _tfNWOG H1-H4). g_tf = minutes du TF courant.
    bool tfNDOG = (g_tf >= 1  && g_tf <= 15);
    bool tfNWOG = (g_tf >= 60 && g_tf <= 240);
    if(!((newDay && tfNDOG) || (newWeek && tfNWOG))) return;

    double opn = open[i];
    double prvC = close[i - 1];
    if(opn == prvC) return;   // gap nul (Pine ligne 1394 : _gTop != _gBot)
    double gTop = MathMax(opn, prvC);
    double gBot = MathMin(opn, prvC);
    double gapMin = GAP_MIN_MULT * atr14;   // Pine i_gapMinMult × atr14
    if(gTop - gBot < gapMin) return;        // gap trop petit (Pine ligne 1394)

    bool isDay = newDay && tfNDOG;   // priorité NDOG si les 2 gating matchent (cas rare)

    // FIFO PAR TYPE (Pine i_maxGap=1/type, décision 2026-07-27) : compte les gaps du même type, si ≥ MAX_GAP_PER_TYPE
    // on supprime le plus ancien de CE type (en décalant les suivants).
    int nSameType = 0;
    for(int k = 0; k < g_nGaps; k++) if(g_gaps[k].isDay == isDay) nSameType++;
    if(nSameType >= MAX_GAP_PER_TYPE) {
        // Trouver l'index du plus ancien du même type (le 1er rencontré, car on insère à la fin).
        for(int k = 0; k < g_nGaps; k++) {
            if(g_gaps[k].isDay == isDay) {
                // Décaler tous les suivants d'un cran vers k (supprime l'élément k).
                for(int m = k; m < g_nGaps - 1; m++) g_gaps[m] = g_gaps[m + 1];
                g_nGaps--;
                break;
            }
        }
    }

    g_gaps[g_nGaps].t0        = time[i - 1];   // Pine box.new(bar_index-1, ...) : bord gauche = bar veille
    g_gaps[g_nGaps].top       = gTop;
    g_gaps[g_nGaps].bot       = gBot;
    g_gaps[g_nGaps].isDay     = isDay;
    g_gaps[g_nGaps].mitigated = false;          // fresh (la mitigation est gérée par f_gapLifecycle)
    g_nGaps++;
}

// Lifecycle NDOG/NWOG (Pine f_ndogLifecycle L1452-1468 + f_nwogLifecycle symétrique).
// Mitigation : si une bougie traverse entièrement le gap (low <= bot && high >= top) → marquer
// mitigated. Le rectangle sera recoloré en atténué par f_drawGaps (clone Pine recoloration).
void f_gapLifecycle(int i, const double &high[], const double &low[]) {
    double hi = high[i], lo = low[i];
    for(int k = 0; k < g_nGaps; k++) {
        if(!g_gaps[k].mitigated && lo <= g_gaps[k].bot && hi >= g_gaps[k].top) {
            g_gaps[k].mitigated = true;   // gap traversé → recoloration atténuée au prochain redraw
        }
    }
}

// Portage Pine lignes 1529-1698 (f_htf + 4 request.security + flags confluence).
// === MOTEUR SMC — HTF confluence (J4e) ===
//
// Le Pine exécute le moteur BOS/OB/mitigation `f_htf(i_htfSwing=3)` sur 4 TF supérieurs
// (H1=60, H4=240, W1=W, MN=M) via request.security(..., lookahead_off). Chaque TF produit
// jusqu'à 3 OB bull + 3 OB bear. Le scoring teste si `close` tombe dans une des 6 zones.
//
// Fidélité f_htf (Pine lignes 1570-1626) — ATTENTION logique SMC standard :
//   - lBT/lBB (Pine) = DERNIÈRE BOUGIE BAISSIÈRE (close<open → top=open, bot=low).
//     C'est le « seed » d'un OB BULL : au prochain BOS haussier, on archive {top=open, bot=low}.
//   - lBuT/lBuB (Pine) = DERNIÈRE BOUGIE HAUSSIÈRE (close>open → top=high, bot=open).
//     Seed d'un OB BEAR : au prochain BOS baissier, on archive {top=high, bot=open}.
//   Donc : OB BULL = dernière bougie baissière AVANT le BOS haussier ; OB BEAR = dernière
//   bougie haussière AVANT le BOS baissier. (Cf. Pine 1603-1626 : b1←_prevLBT au _bosUp,
//   r1←_prevLBuT au _bosDown.) Logique SMC correcte (last opposite-color candle before break).
//   Mitigation (Pine 1628-1645) : OB bull invalidé si close<bot ; OB bear si close>top.
//   FIFO 3 : on garde les 3 derniers OB par sens (décalage b3<-b2<-b1 au BOS).
//
// ANTI-REPAINT STRICT (AGENTS §5bis + J4e vérif 1) :
//   Pour chaque bar `i` de la boucle principale, on lit l'état HTF à la bougie HTF
//   **bougie HTF en cours** (aligné Pine LIVE, repaint assumé — décision Rono 2026-07-23) :
//     int idxHtf = iBarShift(_Symbol, htfPeriod, time[i], false);   // série HTF, 0=récent
//     état ← g_htfStates_<tf>[idxHtf];                              // bougie HTF en cours
//   On rejoue donc f_htf sur TOUTES les bougies HTF clôturées une bonne fois (O(n_htf), une
//   fois par OnCalculate, comme PrecalcAtr14), et on stocke l'état post-clôture de chaque
//   bougie HTF dans g_htfStates_<tf>[kSeries]. La boucle principale lit en O(1) via iBarShift.
//
// PERFORMANCE (J4e vérif 2) : précalcul O(n_htf) une fois, jamais O(n_main × n_htf).
// Les 4 TF sont précalculés AVANT la boucle (comme PrecalcAtr14).
//
// SUBTILITÉ composante (Pine 2071-2080, J4e vérif 6) : le flag confluence teste l'appartenance
// à N'IMPORTE QUELLE des 6 zones, MAIS le bonus n'est crédité QUE si l'OB n°1 du SENS du trade
// (isBull → OB#1 bull ; bear → OB#1 bear) est non-na. À reproduire fidèlement dans f_score.
//
// HARDCODAGES v11 (AGENTS §3 figé) : i_moteurMTF = true (Pine 93), i_htfSwing = 3 (Pine 1515).

#define HTF_SWING 3          // Pine i_htfSwing (longueur pivot f_htf, ligne 1515)
#define N_HTF_OB  3          // 3 OB par sens et par TF (Pine b1/b2/b3, r1/r2/r3)

// Un OB du moteur f_htf (jusqu'à 3 par sens). Simulation du `na` Pine via `valid`.
struct HtfOB3 {
    double top;     // bord haut (Pine _b1T/_r1T) : open pour bull, high pour bear
    double bot;     // bord bas  (Pine _b1B/_r1B) : low  pour bull, open pour bear
    bool   valid;   // true = non-na / non-mitigé (équiv. Pine `not na(...)`)
    datetime t;     // timestamp bougie de formation (pour bord gauche f_drawHtf, Phase 1.4)
};

// Snapshot de l'état f_htf post-clôture d'une bougie HTF : 3 OB bull + 3 OB bear + trend.
// Indexé par series-index HTF dans g_htfStates_<tf>[]. Trend non utilisé par f_score mais
// porté pour fidélité (Pine _trend = élément 0 du tuple).
struct HtfState {
    HtfOB3 bull[N_HTF_OB];   // 3 derniers OB bull (FIFO au BOS haussier + mitigation)
    HtfOB3 bear[N_HTF_OB];   // 3 derniers OB bear (FIFO au BOS baissier + mitigation)
    int    trend;            // +1 haussier, -1 baissier, 0 neutre (Pine 1646-1650)
};

// Initialise un HtfState à vide (toutes zones invalides, trend neutre).
void InitHtfState(HtfState &st) {
    for(int j = 0; j < N_HTF_OB; j++) {
        st.bull[j].top = 0.0; st.bull[j].bot = 0.0; st.bull[j].valid = false; st.bull[j].t = 0;
        st.bear[j].top = 0.0; st.bear[j].bot = 0.0; st.bear[j].valid = false; st.bear[j].t = 0;
    }
    st.trend = 0;
}

// Précalcule l'état f_htf sur TOUTES les bougies HTF clôturées, une bonne fois.
// Rejoue Pine f_htf(swLen) (lignes 1529-1652) en lisant les séries HTF via iHigh/iLow/iOpen/
// iClose, puis stocke l'état post-clôture de chaque bougie HTF dans states[kSeries].
// À appeler AVANT la boucle principale (comme PrecalcAtr14), une fois par OnCalculate.
//
// NOTE iHigh/iLow/iOpen/iClose renvoient 0 si les données HTF ne sont pas chargées : on garde
// alors des séries à 0 → pas de pivot/BOS/OB valides → états vides (confluence false). C'est
// l'équivalent Pine `na` et la garde-fou J4e vérif 8. Au prochain OnCalculate (nouveau bar),
// les données HTF seront chargées → recalcul complet correct.
void PrecalcHtf(ENUM_TIMEFRAMES htf, HtfState &states[]) {
    int nHtf = iBars(_Symbol, htf);
    ArrayResize(states, nHtf);
    for(int k = 0; k < nHtf; k++) InitHtfState(states[k]);
    if(nHtf < 2) return;

    // Construction des séries HTF CHRONOLOGIQUES (index 0 = plus ancien) en une passe.
    // iHigh/iLow/iOpen/iClose sont en convention série (0 = plus récent) : on retourne.
    // Cela permet de réutiliser PivotHighAt/PivotLowAt (qui travaillent sur tableaux chronologiques,
    // fenêtre [i-2*len..i], candidat i-len) — exactement la sémantique de ta.pivothigh Pine.
    double htfO[], htfH[], htfL[], htfC[];
    datetime htfT[];                     // timestamps HTF (pour bord gauche f_drawHtf, Phase 1.4)
    ArrayResize(htfO, nHtf);
    ArrayResize(htfH, nHtf);
    ArrayResize(htfL, nHtf);
    ArrayResize(htfC, nHtf);
    ArrayResize(htfT, nHtf);
    for(int c = 0; c < nHtf; c++) {
        int k = nHtf - 1 - c;            // series-index ↔ chronologique
        htfO[c] = iOpen (_Symbol, htf, k);
        htfT[c] = iTime (_Symbol, htf, k);
        htfH[c] = iHigh(_Symbol, htf, k);
        htfL[c] = iLow (_Symbol, htf, k);
        htfC[c] = iClose(_Symbol, htf, k);
    }

    // État interne f_htf (Pine `var` → locals persistés sur la boucle c).
    double sh = 0, sl = 0;  bool sh_ok = false, sl_ok = false;   // _sh/_sl + garde na
    int    bsh = 0, bsl = 0;                                      // _bsh/_bsl (token anti-doublon)
    bool   lastSH_sig_ok = false, lastSL_sig_ok = false;         // _lastSH_sig/_lastSL_sig
    int    lastSH_sig = 0, lastSL_sig = 0;
    double lBT = 0, lBB = 0;   bool lBT_ok = false;              // _lBT/_lBB (seed OB bull = bougie baissière)
    double lBuT = 0, lBuB = 0; bool lBuT_ok = false;             // _lBuT/_lBuB (seed OB bear = bougie haussière)
    datetime lBT_time = 0, lBuT_time = 0;                        // timestamps seeds (Phase 1.4 f_drawHtf)
    // Archives OB (b=bull, r=bear) — 3 par sens, FIFO.
    double bT[N_HTF_OB], bB[N_HTF_OB]; bool bOk[N_HTF_OB];
    double rT[N_HTF_OB], rB[N_HTF_OB]; bool rOk[N_HTF_OB];
    datetime bT_time[N_HTF_OB], rT_time[N_HTF_OB];   // timestamps formation (Phase 1.4 f_drawHtf)
    for(int j = 0; j < N_HTF_OB; j++) { bT[j]=0; bB[j]=0; bOk[j]=false; bT_time[j]=0;
                                        rT[j]=0; rB[j]=0; rOk[j]=false; rT_time[j]=0; }
    int trend = 0;
    int swLen = HTF_SWING;

    // Boucle chronologique : on traite TOUTES les bougies HTF y compris la bougie en cours
    // (c = 0..nHtf-1). Décision Rono 2026-07-23 : aligner sur le Pine qui évalue f_htf en LIVE
    // (request.security lookahead_off sur la bougie HTF en cours, repaint assumé). Avant on
    // ignorait la bougie en cours (cMax=nHtf-2) pour l'anti-repaint → décalage d'une bougie
    // HTF → écart de scoring (ex: W1 = +5 manquant sur OB#23). Maintenant cMax=nHtf-1 pour
    // que HtfReadState puisse lire states[idxHtf] = état de la bougie HTF en cours.
    int cMax = nHtf - 1;
    for(int c = 0; c <= cMax; c++) {
        double hi = htfH[c], lo = htfL[c], op = htfO[c], cl = htfC[c];
        double clPrev = (c >= 1) ? htfC[c - 1] : 0.0;   // close[1] Pine (bougie HTF précédente)

        // Pine 1538-1549 : pivots + MAJ sh/sl (+ token bsh/bsl = bar_index[swLen] = c-swLen).
        double ph = 0, pl = 0;
        bool hasPH = PivotHighAt(htfH, swLen, c, ph);
        bool hasPL = PivotLowAt (htfL, swLen, c, pl);
        if(hasPH) { sh = ph; sh_ok = true; bsh = c - swLen; }
        if(hasPL) { sl = pl; sl_ok = true; bsl = c - swLen; }

        // Pine 1552-1553 : BOS (validation par clôture uniquement).
        bool bosUp   = sh_ok && (cl > sh) && (c >= 1) && (clPrev <= sh) &&
                       (!lastSH_sig_ok || bsh != lastSH_sig);
        bool bosDown = sl_ok && (cl < sl) && (c >= 1) && (clPrev >= sl) &&
                       (!lastSL_sig_ok || bsl != lastSL_sig);
        // Pine 1554-1557 : flags anti-doublon (1 BOS par pivot).
        if(bosUp)   { lastSH_sig = bsh; lastSH_sig_ok = true; }
        if(bosDown) { lastSL_sig = bsl; lastSL_sig_ok = true; }

        // Pine 1564-1569 : snapshot prev* AVANT la MAJ lBT/lBu du bar courant.
        double prevLBT = lBT, prevLBB = lBB; bool prevLBT_ok = lBT_ok;
        double prevLBuT = lBuT, prevLBuB = lBuB; bool prevLBuT_ok = lBuT_ok;
        datetime prevLBT_time = lBT_time, prevLBuT_time = lBuT_time;

        // Pine 1570-1583 : MAJ seeds OB selon la couleur de la bougie courante.
        //   bougie baissière (close<open) → seed OB BULL {top=open, bot=low} (lBT/lBB).
        //   bougie haussière (close>open) → seed OB BEAR {top=high, bot=open} (lBuT/lBuB).
        if(cl < op) { lBT = op; lBB = lo; lBT_ok = true;  lBuT_ok = false; lBT_time = htfT[c]; }
        if(cl > op) { lBuT = hi; lBuB = op; lBuT_ok = true; lBT_ok = false; lBuT_time = htfT[c]; }

        // Pine 1603-1614 : archive OB BULL au BOS haussier (FIFO 3, depuis prevLBT/prevLBB).
        if(bosUp && prevLBT_ok) {
            bT[2]=bT[1]; bB[2]=bB[1]; bOk[2]=bOk[1]; bT_time[2]=bT_time[1];
            bT[1]=bT[0]; bB[1]=bB[0]; bOk[1]=bOk[0]; bT_time[1]=bT_time[0];
            bT[0]=prevLBT; bB[0]=prevLBB; bOk[0]=true; bT_time[0]=prevLBT_time;
            lBT_ok = false;   // Pine 1613-1614 : _lBT/_lBB := na
        }
        // Pine 1615-1626 : archive OB BEAR au BOS baissier (FIFO 3, depuis prevLBuT/prevLBuB).
        if(bosDown && prevLBuT_ok) {
            rT[2]=rT[1]; rB[2]=rB[1]; rOk[2]=rOk[1]; rT_time[2]=rT_time[1];
            rT[1]=rT[0]; rB[1]=rB[0]; rOk[1]=rOk[0]; rT_time[1]=rT_time[0];
            rT[0]=prevLBuT; rB[0]=prevLBuB; rOk[0]=true; rT_time[0]=prevLBuT_time;
            lBuT_ok = false;   // Pine 1625-1626 : _lBuT/_lBuB := na
        }

        // Pine 1628-1645 : mitigation (close du bar courant).
        //   OB bull invalidé si close < bot ; OB bear invalidé si close > top.
        if(bOk[0] && cl < bB[0]) bOk[0] = false;
        if(bOk[1] && cl < bB[1]) bOk[1] = false;
        if(bOk[2] && cl < bB[2]) bOk[2] = false;
        if(rOk[0] && cl > rT[0]) rOk[0] = false;
        if(rOk[1] && cl > rT[1]) rOk[1] = false;
        if(rOk[2] && cl > rT[2]) rOk[2] = false;

        // Pine 1646-1650 : trend (+1 au bosUp, -1 au bosDown).
        if(bosUp)   trend = 1;
        if(bosDown) trend = -1;

        // Snapshot vers states[seriesIndex]. series-index k = nHtf-1-c.
        // La boucle principale lira states[idxHtf] (bougie HTF en cours, aligné Pine).
        int k = nHtf - 1 - c;
        states[k].trend = trend;
        for(int j = 0; j < N_HTF_OB; j++) {
            states[k].bull[j].top = bT[j]; states[k].bull[j].bot = bB[j]; states[k].bull[j].valid = bOk[j]; states[k].bull[j].t = bT_time[j];
            states[k].bear[j].top = rT[j]; states[k].bear[j].bot = rB[j]; states[k].bear[j].valid = rOk[j]; states[k].bear[j].t = rT_time[j];
        }
    }
}

// true si `price` tombe dans une des 6 zones OB (3 bull + 3 bear) de l'état HTF.
// Pine équivalent : `close >= bot and close <= top` pour chaque zone non-na (1671-1698).
bool HtfInAnyZone(HtfState &st, double price) {
    for(int j = 0; j < N_HTF_OB; j++) {
        if(st.bull[j].valid && price >= st.bull[j].bot && price <= st.bull[j].top) return true;
        if(st.bear[j].valid && price >= st.bear[j].bot && price <= st.bear[j].top) return true;
    }
    return false;
}

// Récupère l'état HTF de la BOUGIE HTF EN COURS correspondant au temps t (aligné Pine).
// Pine évalue f_htf en LIVE via request.security(..., lookahead_off) sur la bougie HTF en cours
// (commentaire Pine lignes 1530-1537 : « confluence réactive au scalping intrabar, repaint assumé »).
// On lit donc states[idxHtf] = état de la bougie HTF contenant t (et NON states[idxHtf+1] = bougie
// HTF précédente clôturée qui était notre choix anti-repaint initial).
// Décision Rono 2026-07-23 : aligner sur le Pine (accepter le repaint HTF) pour matcher le scoring
// TV exactement. L'écart de 5 points (W1) sur OB#23 venait de ce décalage d'une bougie HTF.
bool HtfReadState(ENUM_TIMEFRAMES htf, HtfState &states[], datetime t, HtfState &out) {
    int idxHtf = iBarShift(_Symbol, htf, t, false);
    if(idxHtf < 0) return false;                 // données HTF non chargées
    int k = idxHtf;                              // bougie HTF EN COURS (aligné Pine, repaint assumé)
    if(k < 0 || k >= ArraySize(states)) return false;
    out = states[k];
    return true;
}

// Arrays globaux d'états HTF précalculés (indexés par series-index HTF). Remplis une fois par
// OnCalculate par PrecalcHtf, lus en O(1) dans la boucle via HtfReadState/iBarShift.
HtfState g_htfStatesH1[];
HtfState g_htfStatesH4[];
HtfState g_htfStatesW1[];
HtfState g_htfStatesMN[];

// Flags confluence par TF (consommés par f_score). confluence* = close dans une des 6 zones ;
// *Bull1Valid/*Bear1Valid = OB#1 du sens non-na (subtilité composante Pine 2071-2080).
bool g_confluenceH1, g_confluenceH4, g_confluenceW1, g_confluenceMN;
bool g_h1Bull1Valid, g_h1Bear1Valid;
bool g_h4Bull1Valid, g_h4Bear1Valid;
bool g_w1Bull1Valid, g_w1Bear1Valid;
bool g_mnBull1Valid, g_mnBear1Valid;

// Calcule, au bar courant `i`, les 4 flags confluence + validités OB#1 (Pine 1671-1698).
// À appeler AVANT f_accumScores (f_score y lit g_confluence* / g_*Bull1Valid).
// État HTF lu à la bougie HTF en cours (HtfReadState → states[idxHtf], aligné Pine LIVE,
// repaint assumé — décision Rono 2026-07-23).
void f_htfCompute(int i, const datetime &time[], const double &close[]) {
    // RAZ flags chaque bar (Pine : flags dérivés du bar courant, jamais persistants).
    g_confluenceH1 = g_confluenceH4 = g_confluenceW1 = g_confluenceMN = false;
    g_h1Bull1Valid = g_h1Bear1Valid = false;
    g_h4Bull1Valid = g_h4Bear1Valid = false;
    g_w1Bull1Valid = g_w1Bear1Valid = false;
    g_mnBull1Valid = g_mnBear1Valid = false;

    double c0 = close[i];
    HtfState st;

    // H1 (Pine 1671-1677, weight +1 dans f_score)
    if(HtfReadState(PERIOD_H1, g_htfStatesH1, time[i], st)) {
        g_confluenceH1 = HtfInAnyZone(st, c0);
        g_h1Bull1Valid = st.bull[0].valid;
        g_h1Bear1Valid = st.bear[0].valid;
        g_bsH1Trend = st.trend;                        // BSZones gate (miroir v12)
    }
    // H4 (Pine 1678-1684, weight +4)
    if(HtfReadState(PERIOD_H4, g_htfStatesH4, time[i], st)) {
        g_confluenceH4 = HtfInAnyZone(st, c0);
        g_h4Bull1Valid = st.bull[0].valid;
        g_h4Bear1Valid = st.bear[0].valid;
        g_bsH4Trend = st.trend;                        // BSZones gate (miroir v12)
    }
    // W1 (Pine 1685-1691, weight +5)
    if(HtfReadState(PERIOD_W1, g_htfStatesW1, time[i], st)) {
        g_confluenceW1 = HtfInAnyZone(st, c0);
        g_w1Bull1Valid = st.bull[0].valid;
        g_w1Bear1Valid = st.bear[0].valid;
    }
    // MN (Pine 1692-1698, weight +6)
    if(HtfReadState(PERIOD_MN1, g_htfStatesMN, time[i], st)) {
        g_confluenceMN = HtfInAnyZone(st, c0);
        g_mnBull1Valid = st.bull[0].valid;
        g_mnBear1Valid = st.bear[0].valid;
    }
}

// === SECTION 11 : Scoring (f_force + f_score) ===
// Portage Pine lignes 1995-2110.
// === MOTEUR SMC — scoring ===
// Pine : score brut par bar (accumulé via math.max sur chaque OB), converti en force /10
// par f_force selon des bandes calibrées par asset (SEUIL_MOYEN/FORT/INSTIT/_scoreMax).

// Seuils par asset (Pine lignes 1995-2003). _plafondMoyen = XAG ou BTC → Fort/Instit
// désactivés (seuils 99 inatteignables) : ces assets restent en « Moyen-only ».
int SeuilMoyen()  { return g_isDAX ? 11 : (g_isNAS || g_isSPX) ? 10 : g_isBTC ? 8 : 7; }
int SeuilFort()   { return (g_isXAG || g_isBTC) ? 99 : g_isDAX ? 16 : (g_isNAS || g_isSPX) ? 15 : 10; }
int SeuilInstit() { return (g_isXAG || g_isBTC) ? 99 : g_isDAX ? 19 : (g_isNAS || g_isSPX) ? 17 : 12; }
int ScoreMax()    { return g_isXAU ? 13 : g_isXAG ? 14 : (g_isNAS || g_isSPX) ? 19 :
                            g_isDAX ? 21 : g_isBTC ? 15 : 0; }

// Pine f_force (lignes 2010-2020) : mapping score→force /10 sur bandes calibrées.
//   < Moyen → 1-4 (faible) · Moyen→Fort → 5-6 (correct) · Fort→Instit → 7-8 (fort)
//   ≥ Instit → 9-10 (excellent). Borné [1,10] via MathMin/MathMax.
int f_force(int sc) {
    double f = 0.0;
    int sM = SeuilMoyen(), sF = SeuilFort(), sI = SeuilInstit(), sMax = ScoreMax();
    if(sc < sM)          f = 1.0 + 3.0 * sc / MathMax(1, sM);
    else if(sc < sF)     f = 5.0 + 1.0 * (sc - sM) / MathMax(1, sF - sM);
    else if(sc < sI)     f = 7.0 + 1.0 * (sc - sF) / MathMax(1, sI - sF);
    else                 f = 9.0 + 1.0 * (sc - sI) / MathMax(1, sMax - sI);
    return (int)MathMin(10, MathMax(1, MathRound(f)));
}

// Pondérations scoring adaptatif par asset (Pine lignes 2028-2032).
int Wfvg()   { return (g_isXAU || g_isXAG) ? 5 : g_isBTC ? 3 : 4; }
int Wsweep() { return g_isBTC ? 1 : g_isXAG ? 2 : 4; }
int Wote()   { return (g_isXAG || g_isBTC) ? 2 : 5; }
int Wkz()    { return (g_isXAU || g_isNAS || g_isDAX) ? 3 : g_isBTC ? 2 : 3; }
#define W_ATR 2

// Kill Zone helper (Pine lignes 143-147). Le Pine calcule
//   `int(time % 86400000) / 60000` = ms du jour / 60000 = minutes depuis minuit UTC.
// En MQL5 `datetime` est en SECONDES (et time[] dans OnCalculate est en UTC), donc
// l'équivalent exact est `(t % 86400) / 60` : secondes du jour / 60 = minutes UTC.
// Renvoie true si t tombe dans une des 4 plages Kill Zone (Asie/Londres/NY AM/NY PM).
bool InKzAt(datetime t) {
    long gKzMins = (long)(t % 86400) / 60;
    return (gKzMins >= KZ_ASIAN_START  && gKzMins < KZ_ASIAN_END )
        || (gKzMins >= KZ_LONDON_START && gKzMins < KZ_LONDON_END)
        || (gKzMins >= KZ_NYAM_START   && gKzMins < KZ_NYAM_END  )
        || (gKzMins >= KZ_NYPM_START   && gKzMins < KZ_NYPM_END  );
}

// Pine f_score (lignes 2044-2110) — version bar-par-bar (index i).
// Signature : `int f_score(bool isBull, int i, const datetime &time[], const double &open[],
//                          const double &high[], const double &low[], const double &close[],
//                          double atr14)`.
// Phase 1 (mono-TF initialement) : composantes HTF (confluenceH4/H1/W1/MN) PORTÉES en J4e.
// Le calibrage forceMin=4 reste valide : composantes principales (BOS/FVG/Sweep/MSS/CHOCH) +
// ATR impulsion + Inner Bar (J4a) + Kill Zone (J4c) + OTE & Premium/Discount (J4d) +
// prevLiq PDH/PDL/PWH/PWL (J4b) + HTF confluence H1/H4/W1/MN (J4e) sont toutes portées.
//
// ATTENTION aux subtilités fidèles au Pine :
//   - Anti double-comptage MSS/BOS (Pine 2051) : BOS ne compte QUE si aucun MSS directionnel
//     sur la même barre.
//   - BOS dynamique (Pine 2052-2057) : poids selon taille du corps vs ATR (≥1.5×ATR=6,
//     ≥0.5×ATR=4, sinon 2 ; BTC = -1 partout).
//   - Garde anti-bruit (Pine 2098-2106) : BOS seul (sans sweep/FVG/OTE/HTF) → sc = min(sc, 8).
//     HTF désormais porté (J4e) : un BOS sans sweep/FVG/OTE mais avec HTF≥H4 débloque le plafond.
//   - Asset non reconnu → sc = 0 (Pine 2108-2109).
int f_score(bool isBull, int i, const datetime &time[], const double &open[], const double &high[], const double &low[], const double &close[], double atr14) {
    if(!g_assetReconnu) return 0;
    int sc = 0;
    double c0 = close[i];
    double o0 = open[i];

    // BOS directionnel (Pine 2049-2057). Anti double-comptage : BOS ne compte QUE si aucun
    // MSS directionnel sur la même barre (un MSS implique toujours un BOS).
    bool bosDir = (isBull && g_bosHaussier) || (!isBull && g_bosBaissier);
    bool mssDir = (isBull && g_mssHaussier) || (!isBull && g_mssBaissier);
    if(bosDir && !mssDir) {
        double bodyBOS = MathAbs(c0 - o0);
        // Pine 2053-2056 : poids selon taille du corps vs ATR. BTC = -1 cran partout.
        int wBOS = (atr14 > 0.0) ?
            (bodyBOS >= 1.5 * atr14 ? (g_isBTC ? 5 : 6) :
             bodyBOS >= 0.5 * atr14 ? (g_isBTC ? 3 : 4) :
                                       (g_isBTC ? 1 : 2))
            : (g_isBTC ? 3 : 4);
        sc += wBOS;
    }

    // FVG directionnel (Pine 2058-2059, ligne 668) : +Wfvg UNIQUEMENT si un FVG vient
    // d'apparaître AU BAR COURANT. Bug J-corrigé : on utilisait (g_nFvgBull > 0) qui teste
    // l'historique entier → +Wfvg crédité sur quasi tous les bars (sur-scoring majeur).
    // Désormais g_isFVGBullBar/g_isFVGBearBar reflètent exactement (low - high[2]) > minGap.
    bool isFVGBull = g_isFVGBullBar;
    bool isFVGBear = g_isFVGBearBar;
    if((isBull && isFVGBull) || (!isBull && isFVGBear)) sc += Wfvg();

    // Sweep directionnel frais (Pine 2060-2061).
    if((isBull && g_sweepBullFrais) || (!isBull && g_sweepBearFrais)) sc += Wsweep();

    // MSS directionnel (Pine 2063-2064) : rare, WR 80-82% → +3.
    if((isBull && g_mssHaussier) || (!isBull && g_mssBaissier)) sc += 3;

    // CHOCH confirmé (Pine 2066-2067) : rare, WR 80-82% → +4.
    if((isBull && g_chochHaussier) || (!isBull && g_chochBaissier)) sc += 4;

    // ATR impulsion (Pine 2069-2070) : range1 (high-low) > autoAtrScore*atr14 → +W_ATR.
    // _autoAtrScore est calibré par asset (g_autoAtrScore, porté en J0). À ne pas confondre
    // avec i_atrSeuil/g_autoAtrSeuil réservé à l'AFFICHAGE ATR (module visuel, pas scoring).
    double range1 = high[i] - low[i];
    if(range1 > g_autoAtrScore * atr14)
        sc += W_ATR;

    // Inner Bar (Pine 2081-2082) : impulsion forte du corps (close-open > seuilIB*atr14) → +3.
    // ATTENTION : contrairement à la détection OB (Pine 937 qui stocke ibBull[1], bar
    // précédent), le SCORING utilise le ibBull/ibBear du bar courant = close[i]/open[i].
    // _autoSeuilIB calibré par asset (g_autoSeuilIB, porté en J0). i_moteurIB=true (hardcodé).
    bool ibBull = (close[i] - open[i]) > g_autoSeuilIB * atr14;
    bool ibBear = (open[i] - close[i]) > g_autoSeuilIB * atr14;
    if((isBull && ibBull) || (!isBull && ibBear))
        sc += 3;

    // Kill Zone (Pine 2087-2088) : si le bar courant est dans une Kill Zone UTC → +Wkz.
    // P1.1 — poids selon asset (critique sur or/forex/indices). Le Pine teste `inKZ` qui
    // dérive de `time` (le bar courant) ; en pattern i on utilise time[i] (anti-repaint).
    if(InKzAt(time[i]))
        sc += Wkz();

    // prevLiq (Pine 2089-2092) : proximité PDH/PDL/PWH/PWL → +2, sweep → +4.
    // g_near*/g_sweep* sont calculés une fois par bar par f_prevLiqCompute (globales),
    // exactement comme le Pine où nearBullPrevLiq/sweepBullPrevLiq sont calculés hors
    // f_score (lignes 2034-2042) puis lus ici. i_prevLiqScore = true (hardcodé v11).
    // La signature de f_score ne change pas (lecture via globales, pas via params).
    if((isBull && g_nearBullPrevLiq) || (!isBull && g_nearBearPrevLiq))
        sc += PREVLIQ_PTS_PROX;
    if((isBull && g_sweepBullPrevLiq) || (!isBull && g_sweepBearPrevLiq))
        sc += PREVLIQ_PTS_SWEEP;
    // Module H (Phase 5 29/08) : mega-order (+2, replay +21.3R).
    if(g_megaVol) sc += 2;

    // OTE (Pine 2084-2085) : +Wote si le close est dans la zone Fib 61.8-78.6% (deep).
    // g_inOTE_bull/g_inOTE_bear sont calculés une fois par bar par f_pdOteCompute (globales),
    // exactement comme le Pine où inOTE_bull/inOTE_bear sont calculés hors f_score puis lus
    // ici. La signature de f_score ne change pas (lecture via globales, pas via params).
    if((isBull && g_inOTE_bull) || (!isBull && g_inOTE_bear))
        sc += Wote();

    // Premium/Discount (Pine 2094-2095) : +1 si on est du bon côté de l'équilibre.
    // Bull → on cherche une remontée depuis le Discount ; Bear → une chute depuis le Premium.
    if((isBull && g_inDiscount) || (!isBull && g_inPremium))
        sc += 1;

    // HTF confluence (Pine 2071-2080, J4e). SUBTILITÉ (J4e vérif 6) : le flag confluence
    // teste l'appartenance à N'IMPORTE QUELLE des 6 zones OB, MAIS le bonus n'est crédité QUE
    // si l'OB n°1 du SENS du trade (isBull → OB#1 bull non-na ; bear → OB#1 bear non-na) est
    // valide. g_confluence*/g_*Bull1Valid calculés une fois par bar par f_htfCompute (globales),
    // exactement comme le Pine où confluenceH*/h*BullTop/h*BearTop sont calculés hors f_score
    // puis lus ici. La signature de f_score ne change pas (lecture via globales). Poids v11 :
    // H4=4, H1=1 (HTF le plus bruité), W1=5, MN=6 (le plus institutionnel). i_moteurMTF=true.
    if(g_confluenceH4 && ((isBull && g_h4Bull1Valid) || (!isBull && g_h4Bear1Valid)))
        sc += 4;
    if(g_confluenceH1 && ((isBull && g_h1Bull1Valid) || (!isBull && g_h1Bear1Valid)))
        sc += 1;
    if(g_confluenceW1 && ((isBull && g_w1Bull1Valid) || (!isBull && g_w1Bear1Valid)))
        sc += 5;
    if(g_confluenceMN && ((isBull && g_mnBull1Valid) || (!isBull && g_mnBear1Valid)))
        sc += 6;

    // Garde anti-bruit (Pine 2098-2106) : BOS seul (sans sweep/FVG/OTE/HTF) → sc = min(sc, 8).
    // OTE porté (J4d) + HTF porté (J4e) → un BOS+OTE ou BOS+HTF≥H4 débloque aussi le plafond.
    // SUBTILITÉ (Pine ligne 2104, J4e vérif 7) : _hsHTF = confluenceH4 OR confluenceW1 OR
    // confluenceMN — H1 est EXCLU du seuil institutionnel (HTF le plus bruité : présent 80%,
    // WR ≤ global → ne compte pas comme confirmation). Le combo BOS+Sweep, BOS+FVG, BOS+OTE
    // ou BOS+HTF(H4/W1/MN) débloque donc le plafond de 8.
    bool hsBOS   = (isBull && g_bosHaussier) || (!isBull && g_bosBaissier);
    bool hsSweep = (isBull && g_sweepBullFrais) || (!isBull && g_sweepBearFrais);
    bool hsFVG   = (isBull && isFVGBull) || (!isBull && isFVGBear);
    bool hsOTE   = (isBull && g_inOTE_bull) || (!isBull && g_inOTE_bear);   // J4d : OTE porté.
    bool hsHTF   = g_confluenceH4 || g_confluenceW1 || g_confluenceMN;      // J4e : HTF porté (H1 exclu).
    if(hsBOS && !(hsSweep || hsFVG || hsOTE || hsHTF))
        sc = (int)MathMin(sc, 8);

    // Asset non reconnu → sc = 0 (Pine 2108-2109).
    if(!g_assetReconnu) sc = 0;
    return sc;
}

// Accumulation scores OB (Pine lignes 2117-2145) avec freshness (P1.3) + proximité (P1.4).
// Le score live d'un OB ne doit pas retomber à 0 quand la détection BOS/MSS expire
// (bosHaussier n'est true que sur la barre de détection) : on accumule via math.max tant
// que l'OB n'est pas signaled. À appeler APRÈS f_obLifecycle (sinon on scorerait des OB
// que le lifecycle va supprimer — anti-repaint + cohérence arrays).
//
// Bug 2 corrigé : la version Phase 1 n'appliquait QUE math.max(score, scLive) SANS les bonus
// freshness/proximité du Pine. Conséquences :
//   - OB vierges : manquaient leur +3 (sous-évalués pour les bons setups).
//   - OB profonds (state 2) : gardaient leur score max artificiellement (sur-évalués).
//   - OB lointains (>10×ATR) : gardaient leur score au lieu d'être forcés à 0 (sur-évalués
//     majeur → cause des OB 5/10 TV notés 10/10 MT5). _prox = -999 force _cand ≪ 0.
// Désormais fidèle au Pine :
//   _fresh = state==0 ? +3 : state==2 ? -2 : 0   (vierge +3 | partiel 0 | profond -2)
//   _prox  = dist>10 ? -999 : dist<1 ? +2 : dist>5 ? -1 : 0
//   _cand  = scLive + _fresh + _prox
//   state==2 → score = max(0, _cand)            (descente autorisée, OB retouché)
//   sinon    → score = max(score_actuel, _cand) (pattern « ne redescend pas »)
// _distB = abs(close[i] - mid) / atr14, avec atr14 <= 0 → _dist = 0 (fidèle Pine : 0.0).
// .mid est précalculé à la création de l'OB (struct OBT, f_obCreate).
void f_accumScores(int i, const datetime &time[], const double &open[], const double &high[], const double &low[], const double &close[], double atr14) {
    int scBull = f_score(true,  i, time, open, high, low, close, atr14);
    int scBear = f_score(false, i, time, open, high, low, close, atr14);
    double c0 = close[i];   // Pine `close` (bar courant clôturé, anti-repaint).
    bool   atrOk = (atr14 > 0.0);

    // OB BULL (Pine 2117-2132).
    for(int k = 0; k < g_nObBull; k++) {
        if(g_obBull[k].signaled) continue;
        int   _stB   = g_obBull[k].state;
        double _midB = g_obBull[k].mid;                                   // Pine _midB
        double _distB = atrOk ? MathAbs(c0 - _midB) / atr14 : 0.0;        // Pine 2122 : 0.0 si atr na
        int _freshB = (_stB == 0) ? 3 : (_stB == 2) ? -2 : 0;             // Pine 2124
        int _proxB  = (_distB > 10.0) ? -999 : (_distB < 1.0) ? 2 : (_distB > 5.0) ? -1 : 0; // Pine 2126
        int _cand   = scBull + _freshB + _proxB;                          // Pine 2127
        int _oldScB = g_obBull[k].score;                                  // avant maj (diag)
        if(_stB == 2)
            g_obBull[k].score = (int)MathMax(0, _cand);                   // Pine 2130 : descente autorisée
        else
            g_obBull[k].score = (int)MathMax(g_obBull[k].score, _cand);   // Pine 2132 : ne redescend pas
        // DIAGNOSTIC : mémoriser les flags actifs au moment du nouveau max.
        if(g_obBull[k].score > _oldScB) {
            g_obBull[k].diagMaxSc = scBull;
            g_obBull[k].diagMaxT  = time[i];
            g_obBull[k].diagFlags =
                "BOS=" + (string)(int)g_bosHaussier +
                " FVG=" + (string)(int)g_isFVGBullBar +
                " Swp=" + (string)(int)g_sweepBullFrais +
                " MSS=" + (string)(int)g_mssHaussier +
                " CHOCH=" + (string)(int)g_chochHaussier +
                " IB=" + (string)(int)((close[i]-open[i]) > g_autoSeuilIB*atr14) +
                " KZ=" + (string)(int)InKzAt(time[i]) +
                " disc=" + (string)(int)g_inDiscount +
                " OTE=" + (string)(int)g_inOTE_bull +
                " near=" + (string)(int)g_nearBullPrevLiq +
                " swpL=" + (string)(int)g_sweepBullPrevLiq +
                " H1=" + (string)(int)g_confluenceH1 + "/" + (string)(int)g_h1Bull1Valid +
                " H4=" + (string)(int)g_confluenceH4 + "/" + (string)(int)g_h4Bull1Valid +
                " W1=" + (string)(int)g_confluenceW1 + "/" + (string)(int)g_w1Bull1Valid +
                " MN=" + (string)(int)g_confluenceMN + "/" + (string)(int)g_mnBull1Valid;
        }
        // LATCH zone (Pine f_zonesLifecycle L3097-3112) : active la zone une fois que l'OB
        // devient un setup valide (force ≥ forceMin + asset + qualité live). La zone RESTE
        // ensuite active même si la qualité live retombe (FVG expiré, prix éloigné) — fidèle
        // Pine qui ne supprime jamais la zone ici, uniquement via le lifecycle OB.
        if(!g_obBull[k].zoneActive
           && g_assetReconnu
           && f_force(g_obBull[k].score) >= FORCE_MIN
           && f_znQualBull(k)) {
            g_obBull[k].zoneActive = true;
        }
    }

    // OB BEAR (Pine 2133-2145).
    for(int k = 0; k < g_nObBear; k++) {
        if(g_obBear[k].signaled) continue;
        int   _stR   = g_obBear[k].state;
        double _midR = g_obBear[k].mid;                                   // Pine _midR
        double _distR = atrOk ? MathAbs(c0 - _midR) / atr14 : 0.0;        // Pine 2138 : 0.0 si atr na
        int _freshR = (_stR == 0) ? 3 : (_stR == 2) ? -2 : 0;             // Pine 2139
        int _proxR  = (_distR > 10.0) ? -999 : (_distR < 1.0) ? 2 : (_distR > 5.0) ? -1 : 0; // Pine 2140
        int _cand   = scBear + _freshR + _proxR;                          // Pine 2141
        int _oldSc = g_obBear[k].score;                                   // avant maj (diag)
        if(_stR == 2)
            g_obBear[k].score = (int)MathMax(0, _cand);                   // Pine 2143 : descente autorisée
        else
            g_obBear[k].score = (int)MathMax(g_obBear[k].score, _cand);   // Pine 2145 : ne redescend pas
        // DIAGNOSTIC : si le score vient d'atteindre un nouveau max, mémoriser les flags actifs.
        if(g_obBear[k].score > _oldSc) {
            g_obBear[k].diagMaxSc = scBear;
            g_obBear[k].diagMaxT  = time[i];
            g_obBear[k].diagFlags =
                "BOS=" + (string)(int)g_bosBaissier +
                " FVG=" + (string)(int)g_isFVGBearBar +
                " Swp=" + (string)(int)g_sweepBearFrais +
                " MSS=" + (string)(int)g_mssBaissier +
                " CHOCH=" + (string)(int)g_chochBaissier +
                " IB=" + (string)(int)((open[i]-close[i]) > g_autoSeuilIB*atr14) +
                " KZ=" + (string)(int)InKzAt(time[i]) +
                " prem=" + (string)(int)g_inPremium +
                " OTE=" + (string)(int)g_inOTE_bear +
                " near=" + (string)(int)g_nearBearPrevLiq +
                " swpL=" + (string)(int)g_sweepBearPrevLiq +
                " H1=" + (string)(int)g_confluenceH1 + "/" + (string)(int)g_h1Bear1Valid +
                " H4=" + (string)(int)g_confluenceH4 + "/" + (string)(int)g_h4Bear1Valid +
                " W1=" + (string)(int)g_confluenceW1 + "/" + (string)(int)g_w1Bear1Valid +
                " MN=" + (string)(int)g_confluenceMN + "/" + (string)(int)g_mnBear1Valid;
        }
        // LATCH zone (Pine f_zonesLifecycle, branche bear L3135-3149) : active la zone une fois
        // que l'OB devient un setup valide. La zone reste ensuite active (fidèle Pine LATCH).
        if(!g_obBear[k].zoneActive
           && g_assetReconnu
           && f_force(g_obBear[k].score) >= FORCE_MIN
           && f_znQualBear(k)) {
            g_obBear[k].zoneActive = true;
        }
    }
}

// === SECTION 12 : Filtres zone (f_znQualBull/Bear, f_tradeBloquant) ===
// Portage Pine lignes 2769-2816.
// === MOTEUR SMC — filtres zone ===
// Pine : un OB ne devient une ZONE/Trade valide QUE si (force ≥ forceMin, testé en J5) ET
// f_znQualBull/Bear (FVG chevauchant + DoL directionnel). _znDaxHTF neutralise ces filtres
// sur DAX M15/M30 (Pine ligne 878).
// (FORCE_MIN déplacé en tête de fichier — utilisé par f_accumScores qui précède cette section.)

// _znDaxHTF (Pine ligne 878) : DAX M15/M30 → FVG/DoL neutralisés.
// Calculé une fois par OnCalculate (avant la boucle) car ne dépend que de l'asset/TF.
bool g_znDaxHTF;

// f_znHasFVG (Pine lignes 2790-2796) : intersection boîtes OB/FVG.
// Pine : array.get(_ft,k) > _obB AND array.get(_fb,k) < _obT (chevauchement de deux plages).
bool f_znHasFVG(FVGT &fvg[], int nFvg, double obT, double obB) {
    for(int k = 0; k < nFvg; k++) {
        if(fvg[k].top > obB && fvg[k].bot < obT) return true;
    }
    return false;
}

// f_znQualBull (Pine lignes 2798-2806). DoL BULL = liquidité HAUTE au-dessus de l'OB top
// (EQH/PDH/PWH/Asian High — cible haussière). _t2 = obBullTop.
// J4b : DoL complet — les 4 sources de liquidité (EQH + PDH + PWH + AsianHigh) comme le
// Pine ligne 2805. PDH/PWH viennent de f_prevLiqCompute (g_pdh/g_pwh), AsianHigh de
// f_asianHlCompute (g_ahHighDrawn_valid + g_ahHighDrawn).
bool f_znQualBull(int i) {
    bool fvg = true;
    if(!g_znDaxHTF)   // Pine i_znFvgReq = not _znDaxHTF
        fvg = f_znHasFVG(g_fvgBull, g_nFvgBull, g_obBull[i].top, g_obBull[i].bot);
    bool dol = true;
    if(!g_znDaxHTF) {   // Pine i_znDolReq = not _znDaxHTF
        double t2 = g_obBull[i].top;
        // Pine 2805 : liquidité HAUTE au-dessus de t2 (EQH OR PDH OR PWH OR AsianHigh).
        bool hasLiq = (g_dernierEQH_level > 0.0 && g_dernierEQH_level > t2)    // EQH
                   || (g_pdhActive > 0.0 && g_pdhActive > t2)                   // PDH (actif = non sweepé)
                   || (g_pwhActive > 0.0 && g_pwhActive > t2)                   // PWH
                   || (g_ahHighDrawn_valid && g_ahHighDrawn > t2);             // Asian High
        dol = hasLiq;
    }
    return fvg && dol;
}

// f_znQualBear (Pine lignes 2808-2816). DoL BEAR = liquidité BASSE sous l'OB bot
// (EQL/PDL/PWL/Asian Low — cible baissière). _b2 = obBearBot.
// J4b : DoL complet — les 4 sources (EQL + PDL + PWL + AsianLow) comme le Pine ligne 2815.
bool f_znQualBear(int i) {
    bool fvg = true;
    if(!g_znDaxHTF)
        fvg = f_znHasFVG(g_fvgBear, g_nFvgBear, g_obBear[i].top, g_obBear[i].bot);
    bool dol = true;
    if(!g_znDaxHTF) {
        double b2 = g_obBear[i].bot;
        // Pine 2815 : liquidité BASSE sous b2 (EQL OR PDL OR PWL OR AsianLow).
        bool hasLiq = (g_dernierEQL_level > 0.0 && g_dernierEQL_level < b2)    // EQL
                   || (g_pdlActive > 0.0 && g_pdlActive < b2)                   // PDL (actif = non sweepé)
                   || (g_pwlActive > 0.0 && g_pwlActive < b2)                   // PWL
                   || (g_ahLowDrawn_valid && g_ahLowDrawn < b2);               // Asian Low
        dol = hasLiq;
    }
    return fvg && dol;
}

// f_tradeBloquant (Pine lignes 2964-2974) : bloqué si un trade est REMPLI ET pré-TP1 ET non fermé.
// g_tradeOuvert est recalculé à chaque bar par f_updateTradeState (parcours des g_signals[]).
bool g_tradeOuvert = false;
bool f_tradeBloquant() {
    return g_tradeOuvert;
}

// === SECTION 13 : Génération signaux (f_createBuySignals/Sell) ===
// Portage Pine lignes 2845-3155 (f_createBuySignals / f_createSellSignals).
// === MOTEUR SMC — signaux ===
//
// Pine f_createBuySignals : boucle sur les OB bull, cherche un RETOUR du prix au bord haut
//   _rt de l'OB (close > _rt). Gate signal (Pine 2865) : score ≥ seuilTrade (0) ET
//   f_force(scR) ≥ forceMin ET f_znQualBull(i) ET not _oneBullSignal (1 signal/bar).
//   Entrée TR forcé = _rt (Pine 2870), SL selon g_autoSlMode (Pine 2871-2873), garde
//   anti-SL trop large (Pine 2875 : _r > 2*_slMax → skip), clamp R dans [_slMin,_slMax]
//   (Pine 2880), TP1 = entry + R (Pine 2882, 1R).
// f_createSellSignals : symétrique sur OB bear, bord bas _rb, SL au-dessus, TP1 = entry - R.
//
// Pattern `i` : "close"/"bar_index" Pine = close[i]/i, et le signal est daté à time[i].
// Anti-repaint : i vient toujours de la boucle OnCalculate (<= rates_total-2).

SignalT g_signals[];    // historique des signaux générés (ArrayResize par signal, peu nombreux)
int     g_nSignals = 0;

// Pine lignes 2263-2274 : _slMin/_slMax par asset, ATR-dépendants + fallback pips.
// En MQL5 on les calcule dynamiquement par fonction (et non en #define statique) car atr14
// varie à chaque bar. Les multiplicateurs ci-dessous reproduisent exactement le Pine :
//   XAU/NAS/DAX → min 0.5× / max 1.5× ; XAG → 0.6× / 1.8× ; BTC → 0.8× / 2.5×.
// Fallback pips quand atr14 indispo (début d'historique) : _pipValue porté en g_pipValue (J0).
double SlMinForAsset(double atr14) {
    if(atr14 > 0.0)
        return g_isXAU ? 0.5*atr14 : g_isXAG ? 0.6*atr14 :
               (g_isNAS || g_isSPX) ? 0.5*atr14 : g_isBTC ? 0.8*atr14 :
               g_isDAX ? 0.5*atr14 : 0.0;
    // Fallback pips (Pine _pipValue).
    return g_isXAU ? 50.0*g_pipValue : g_isXAG ? 50.0*g_pipValue :
           (g_isNAS || g_isSPX) ? 20.0*g_pipValue : g_isBTC ? 100.0*g_pipValue :
           g_isDAX ? 20.0*g_pipValue : 0.0;
}
double SlMaxForAsset(double atr14) {
    if(atr14 > 0.0)
        return g_isXAU ? 1.5*atr14 : g_isXAG ? 1.8*atr14 :
               (g_isNAS || g_isSPX) ? 1.5*atr14 : g_isBTC ? 2.5*atr14 :
               g_isDAX ? 1.5*atr14 : 1e10;
    return g_isXAU ? 100.0*g_pipValue : g_isXAG ? 100.0*g_pipValue :
           (g_isNAS || g_isSPX) ? 50.0*g_pipValue : g_isBTC ? 400.0*g_pipValue :
           g_isDAX ? 50.0*g_pipValue : 1e10;
}

// Pine f_createBuySignals (lignes 2845-2934) — pattern `i`.
// f_tradeBloquant (Pine 2847) en tête : bloque si un trade est déjà en cours.
void f_createBuySignals(int i, const datetime &time[], const double &close[], double atr14) {
    if(f_tradeBloquant()) return;     // Pine ligne 2847 : 1 trade à la fois
    if(g_nObBull == 0) return;
    double c0    = close[i];          // "close" Pine (bar clôturé courant)
    int   curBar = i;                 // "bar_index" Pine

    bool oneBullSignal = false;       // Pine _oneBullSignal : 1 signal max par bar
    for(int k = 0; k < g_nObBull; k++) {
        double rt   = g_obBull[k].top;    // _rt (bord haut OB)
        double rb   = g_obBull[k].bot;    // _rb (bord bas  OB)
        int    rst  = g_obBull[k].state;  // _rst
        bool   rSig = g_obBull[k].signaled;
        int    rBar = g_obBull[k].barIdx;
        // Pine ligne 2857 : _proche = (close - _rt) <= 8.0 * atr14 (proximité, limite signaux lointains)
        bool proche = (atr14 <= 0.0) || ((c0 - rt) <= 8.0 * atr14);
        // Pine ligne 2857-2858 : _retour = close > _rt and _rst < 2 and not _rSig
        //                        and bar_index > _rBar and _proche
        bool retour = (c0 > rt) && (rst < 2) && !rSig && (curBar > rBar) && proche;
        if(!retour) continue;

        int scR       = g_obBull[k].score;
        int seuilTrade = 0;   // Pine ligne 2003 : seuilTrade = 0 (tout retour OB = trade potentiel)
        // Pine ligne 2865 : scR >= seuilTrade and f_force(scR) >= forceMin and f_znQualBull
        if(!(scR >= seuilTrade && f_force(scR) >= FORCE_MIN && f_znQualBull(k) && !oneBullSignal))
            continue;

        // v11 : entrée TR forcé au _rt (bord haut OB). Pine ligne 2870.
        double entry = rt;
        // SL selon g_autoSlMode (Pine lignes 2871-2873).
        double sl = (g_autoSlMode == "1.5× ATR sous OB") ? rb - 1.5 * 0.75 * atr14 :
                    (g_autoSlMode == "1× ATR sous OB")   ? rb - 0.75 * atr14 :
                    (g_autoSlMode == "2× ATR sous OB")   ? rb - 2.0 * 0.75 * atr14  : rb;   // étape 4 29/08 : offset ×0.75
        double r     = entry - sl;
        double slMax = SlMaxForAsset(atr14);
        double slMin = SlMinForAsset(atr14);
        // Pine ligne 2875 : garde anti-SL trop large (_r > 2.0 * _slMax → skip cet OB).
        if(r > 2.0 * slMax) continue;
        // Pine ligne 2880 : clamp _r dans [_slMin, _slMax], puis _sl = _entry - _r.
        r  = MathMax(slMin, MathMin(slMax, r));
        sl = entry - r;
        double tp1 = entry + 0.6 * r;   // étape 4 29/08 : TP1 = 0,6R (replay +239R)
        double tp2 = entry + 2.0 * r;   // Pine ligne 2883 : TP2 = entry + 2R
        // TP3 : liquidité HAUTE la plus proche (EQH/PDH/PWH/AHH), sinon entry + 3R (Pine 2884-2895).
        double tp3Cap = entry + 3.0 * r;   // Phase 2 28/08 : DoL plafonné à 3R
        double tp3 = tp3Cap;
        double m3 = 1e15;
        if(g_dernierEQH_level > 0.0 && g_dernierEQH_level > entry) m3 = MathMin(m3, g_dernierEQH_level);
        if(g_pdhActive > 0.0 && g_pdhActive > entry) m3 = MathMin(m3, g_pdhActive);
        if(g_pwhActive > 0.0 && g_pwhActive > entry) m3 = MathMin(m3, g_pwhActive);
        if(g_ahHighDrawn_valid && g_ahHighDrawn > entry) m3 = MathMin(m3, g_ahHighDrawn);
        // Phase 2 28/08 (DoL≤3R) : TP3 = min(liquidité, entry+3R) ; monotonie
        // TP3 >= TP2 conservée (repli sur le plafond 3R si brisée).
        if(m3 < 1e15 && m3 >= tp2) tp3 = MathMin(m3, tp3Cap);

        // Archivage du signal
        SignalT sig;
        sig.t     = time[i];
        sig.entry = entry;
        sig.sl    = sl;
        sig.tp1   = tp1;
        sig.tp2   = tp2;
        sig.tp3   = tp3;
        sig.force = f_force(scR);
        sig.score = scR;
        sig.bull  = true;
        sig.obIdx = k;   // Pine : OB sous-jacent pour _scoreDeg
        sig.openTs  = time[i];
        sig.filled  = false;
        sig.fillT   = 0;
        sig.t1Hit   = false;
        sig.t2Ts    = 0;
        sig.closed  = false;
        sig.closeT  = 0;
        sig.closeRsn = "";
        sig.closeR   = 0.0;
        sig.R0       = r;   // R initial figé (entry - sl AVANT clamp pour BUY, = r clampé)
        ArrayResize(g_signals, g_nSignals + 1);
        g_signals[g_nSignals] = sig;
        g_nSignals++;

        // Anti-doublon (Pine ligne 2933) : marque l'OB signalé + 1 signal max par bar.
        g_obBull[k].signaled = true;
        oneBullSignal        = true;
        // NB : g_tradeOuvert est géré par f_updateTradeState (bloquant = filled && !t1Hit && !closed).
    }
}

// Pine f_createSellSignals (lignes 3067-3155) — symétrique, pattern `i`.
// Entrée TR forcé au _rb (bord bas OB), SL au-dessus, TP1 = entry - R.
void f_createSellSignals(int i, const datetime &time[], const double &close[], double atr14) {
    if(f_tradeBloquant()) return;     // Pine ligne 3069 : 1 trade à la fois
    if(g_nObBear == 0) return;
    double c0    = close[i];
    int   curBar = i;

    bool oneBearSignal = false;       // Pine _oneBearSignal : 1 signal max par bar
    for(int k = 0; k < g_nObBear; k++) {
        double rt   = g_obBear[k].top;    // _rt
        double rb   = g_obBear[k].bot;    // _rb (bord bas OB)
        int    rst  = g_obBear[k].state;  // _rst
        bool   rSig = g_obBear[k].signaled;
        int    rBar = g_obBear[k].barIdx;
        // Pine _proche symétrique : (_rb - close) <= 8.0 * atr14
        bool proche = (atr14 <= 0.0) || ((rb - c0) <= 8.0 * atr14);
        // Pine ligne 3079-3080 : _retour = close < _rb and _rst < 2 and not _rSig
        //                        and bar_index > _rBar and _proche
        bool retour = (c0 < rb) && (rst < 2) && !rSig && (curBar > rBar) && proche;
        if(!retour) continue;

        int scR       = g_obBear[k].score;
        int seuilTrade = 0;
        // Pine ligne 3088 : scR >= seuilTrade and f_force(scR) >= forceMin and f_znQualBear
        if(!(scR >= seuilTrade && f_force(scR) >= FORCE_MIN && f_znQualBear(k) && !oneBearSignal))
            continue;

        // v11 : entrée TR forcé au _rb (bord bas OB). Pine ligne 3093.
        double entry = rb;
        // SL symétrique au-dessus selon g_autoSlMode (Pine lignes 3089-3092).
        double sl = (g_autoSlMode == "1.5× ATR sous OB") ? rt + 1.5 * 0.75 * atr14 :
                    (g_autoSlMode == "1× ATR sous OB")   ? rt + 0.75 * atr14 :
                    (g_autoSlMode == "2× ATR sous OB")   ? rt + 2.0 * 0.75 * atr14  : rt;   // étape 4 29/08
        double r     = sl - entry;
        double slMax = SlMaxForAsset(atr14);
        double slMin = SlMinForAsset(atr14);
        // Pine ligne 3094 : garde anti-SL trop large (_r > 2.0 * _slMax → skip).
        if(r > 2.0 * slMax) continue;
        // Pine ligne 3099 : clamp _r dans [_slMin, _slMax], puis _sl = _entry + _r.
        r  = MathMax(slMin, MathMin(slMax, r));
        sl = entry + r;
        double tp1 = entry - 0.6 * r;   // étape 4 29/08 : TP1 = 0,6R
        double tp2 = entry - 2.0 * r;   // Pine ligne 3102 : TP2 = entry - 2R
        // TP3 : liquidité BASSE la plus proche (EQL/PDL/PWL/AHL), sinon entry - 3R.
        // Pine math.max (lignes 3246-3249) : la plus proche = la MOINS basse (MAX des liquidités < entry).
        // Bug corrigé 2026-07-29 : MQL5 utilisait MathMin (la plus éloignée) au lieu de MathMax.
        double tp3Cap = entry - 3.0 * r;   // Phase 2 28/08 : DoL plafonné à 3R
        double tp3 = tp3Cap;
        double m3 = -1e15;
        if(g_dernierEQL_level > 0.0 && g_dernierEQL_level < entry) m3 = MathMax(m3, g_dernierEQL_level);
        if(g_pdlActive > 0.0 && g_pdlActive < entry) m3 = MathMax(m3, g_pdlActive);
        if(g_pwlActive > 0.0 && g_pwlActive < entry) m3 = MathMax(m3, g_pwlActive);
        if(g_ahLowDrawn_valid && g_ahLowDrawn < entry) m3 = MathMax(m3, g_ahLowDrawn);
        // Phase 2 28/08 (DoL≤3R) : TP3 = max(liquidité, entry-3R) ; monotonie conservée.
        if(m3 > -1e15 && m3 <= tp2) tp3 = MathMax(m3, tp3Cap);

        SignalT sig;
        sig.t     = time[i];
        sig.entry = entry;
        sig.sl    = sl;
        sig.tp1   = tp1;
        sig.tp2   = tp2;
        sig.tp3   = tp3;
        sig.force = f_force(scR);
        sig.score = scR;
        sig.bull  = false;
        sig.obIdx = k;   // Pine : OB sous-jacent pour _scoreDeg
        sig.openTs  = time[i];
        sig.filled  = false;
        sig.fillT   = 0;
        sig.t1Hit   = false;
        sig.t2Ts    = 0;
        sig.closed  = false;
        sig.closeT  = 0;
        sig.closeRsn = "";
        sig.closeR   = 0.0;
        sig.R0       = r;
        ArrayResize(g_signals, g_nSignals + 1);
        g_signals[g_nSignals] = sig;
        g_nSignals++;

        // Anti-doublon (Pine ligne 3154) : marque l'OB signalé + 1 signal max par bar.
        g_obBear[k].signaled = true;
        oneBearSignal        = true;
    }
}

// Machine à états du lifecycle des trades — clone fidèle Pine L3312-3483 (BUY) / L3477-3648 (SELL).
// ORDRE PINE STRICT (décisif sur les bougies où SL+TP1 sont touchés simultanément) :
//   1) FILL
//   2) Calcul clôtures (_slHit/_beHit/_tp2SLHit/_tp3Hit) avec t1Hit/t2Ts LUS AU DÉBUT du bar
//   3) Calcul _expire, _beForce, _scoreDeg
//   4) SI clôture → fermer le trade
//   5) SINON SI passage BE forcé (_filled && (_beForce||_scoreDeg) && !_t1Hit) → SL→entry, t1Hit=true
//   6) SINON → marquer TP1 puis TP2 (uniquement si pas de clôture ce bar)
void f_updateTradeState(int i, const datetime &time[], const double &high[], const double &low[], const double &close[]) {
    for(int k = 0; k < g_nSignals; k++) {
        if(g_signals[k].closed) continue;   // trade déjà fermé, ne plus évaluer

        double entry = g_signals[k].entry;
        double sl    = g_signals[k].sl;        // SL courant (peut être BE=entry après TP1)
        double tp1   = g_signals[k].tp1;
        double tp2   = g_signals[k].tp2;
        double tp3   = g_signals[k].tp3;
        bool   bull  = g_signals[k].bull;
        bool   filled = g_signals[k].filled;
        bool   t1Hit  = g_signals[k].t1Hit;    // LU AU DÉBUT du bar (Pine _t1Hit)
        datetime t2Ts = g_signals[k].t2Ts;     // LU AU DÉBUT du bar (Pine _t2Ts)
        datetime openTs = g_signals[k].openTs;

        // 1) FILL (Pine L3314/3479).
        if(!filled && time[i] > openTs) {
            if(bull && low[i] <= entry)   { g_signals[k].filled = true; g_signals[k].fillT = time[i]; filled = true; }
            if(!bull && high[i] >= entry) { g_signals[k].filled = true; g_signals[k].fillT = time[i]; filled = true; }
        }

        // 2) Clôtures (Pine L3356-3359 / L3521-3524) — calculées avec t1Hit/t2Ts du DÉBUT du bar.
        bool slHit = false, beHit = false, tp2SLHit = false, tp3Hit = false;
        if(filled) {
            if(bull) {
                slHit    = low[i] < sl && !t1Hit;
                beHit    = low[i] < entry && t1Hit && t2Ts == 0;
                tp2SLHit = low[i] < tp1 && t1Hit && t2Ts > 0;
                tp3Hit   = high[i] >= tp3;
            } else {
                slHit    = high[i] > sl && !t1Hit;
                beHit    = high[i] > entry && t1Hit && t2Ts == 0;
                tp2SLHit = high[i] > tp1 && t1Hit && t2Ts > 0;
                tp3Hit   = low[i] <= tp3;
            }
        }

        // 3) Expiration + BOS contre + dégradation score (Pine L3354-3355, L3369, L3425-3426).
        long ageSecs = (long)(time[i] - openTs);
        bool ageExpire = ageSecs > (long)g_autoTradeMaxMins * 60;
        bool tp3Expire = (t2Ts > 0) && ((long)(time[i] - t2Ts) > (long)g_autoTp3Mins * 60);
        bool expire = ageExpire || tp3Expire;
        bool beForce = !t1Hit && (bull ? g_bosBaissier : g_bosHaussier);
        // _scoreDeg (Pine L3425-3426) : dégradation de la force de l'OB sous-jacent sous forceMin.
        bool scoreDeg = false;
        int obI = g_signals[k].obIdx;
        if(obI >= 0) {
            if(bull && obI < g_nObBull && !t1Hit)
                scoreDeg = f_force(g_obBull[obI].score) < FORCE_MIN;
            else if(!bull && obI < g_nObBear && !t1Hit)
                scoreDeg = f_force(g_obBear[obI].score) < FORCE_MIN;
        }

        // 4) Clôture (Pine L3428-3430) : si l'une des conditions → fermer.
        if((filled && (slHit || beHit || tp2SLHit || tp3Hit)) || expire || (!filled && beForce)) {
            g_signals[k].closed = true;
            g_signals[k].closeT = time[i];
            string rsn =
                slHit    ? "SL" :
                beHit    ? "BE" :
                tp2SLHit ? "TP2SL" :
                tp3Hit   ? "TP3" :
                expire   ? "EXPIRE" : "BOS";
            g_signals[k].closeRsn = rsn;
            // R final selon la raison : SL=-1, BE=+1 (TP1 sécurisé avant BE),
            // TP2SL=+1, TP3=+3, EXPIRE=close[i] vs entry, BOS=0 (non rempli).
            double rDist = g_signals[k].R0;
            if(rDist == 0) rDist = 1.0;
            if(rsn == "SL")       g_signals[k].closeR = -1.0;
            else if(rsn == "BE")  g_signals[k].closeR = 1.0;
            else if(rsn == "TP2SL") g_signals[k].closeR = 1.0;
            else if(rsn == "TP3") g_signals[k].closeR = (bull ? (tp3 - entry) : (entry - tp3)) / rDist;
            else if(rsn == "EXPIRE") {
                double distPx = bull ? (close[i] - entry) : (entry - close[i]);
                g_signals[k].closeR = distPx / rDist;
            }
            else g_signals[k].closeR = 0.0;   // BOS non rempli
        }
        // 5) Passage BE forcé sur trade REMPLI (Pine L3468-3483 / L3633-3648) :
        //    si BOS contre tendance OU dégradation score, ET pas encore TP1 → SL→entry, t1Hit=true.
        else if(filled && (beForce || scoreDeg) && !t1Hit) {
            g_signals[k].sl    = entry;    // BE : SL → entry
            g_signals[k].t1Hit = true;     // marque TP1 (le trade survit en mode BE)
        }
        // 6) Sinon : marquer TP1 puis TP2 (Pine L3436-3449 / L3501-3514).
        else if(filled) {
            if(!t1Hit) {
                if((bull && high[i] >= tp1) || (!bull && low[i] <= tp1)) {
                    g_signals[k].t1Hit = true;
                    g_signals[k].sl    = entry;   // BE
                }
            }
            if(g_signals[k].t1Hit && g_signals[k].t2Ts == 0) {
                if((bull && high[i] >= tp2) || (!bull && low[i] <= tp2)) {
                    g_signals[k].t2Ts = time[i];
                }
            }
        }
    }

    // Relâchement du bloquant (Pine f_tradeBloquant L2964-2974) : bloque si un trade est
    // REMPLI ET pré-TP1 ET non fermé. Un trade non rempli ou post-TP1 ne bloque pas.
    bool bloq = false;
    for(int k = 0; k < g_nSignals; k++) {
        if(g_signals[k].filled && !g_signals[k].t1Hit && !g_signals[k].closed) {
            bloq = true;
            break;
        }
    }
    g_tradeOuvert = bloq;
}


// ══════════════════════════════════════════════════════════════════
//  MODULE BSZONES — moteur B (Sweep→Disp→OB canonique ICT 2022)
//  Miroir v12 (Rust scoring_bs_zones.rs + bs_helpers.rs + Pine 3051-3416).
//  Socle /16 (disp+sweep+vierge+bos+choch+bodyrange+vol) + dyn /11
//  (prox+fvg+pd+kz+htf+ote+coeur) → toForce10 (denom 27). Gate naissance :
//  base ≥ 6 ET (H1 ou H4 aligné). Lifecycle mitigation 3 états + invalidation.
//  Trades : force ≥ 7, retour + prox ≤ 8×ATR, TP3 DoL≤3R SANS AsianHL (parité
//  Pine : AsianHL = v11 uniquement), obIdx = -1 (court-circuite _scoreDeg).
// ══════════════════════════════════════════════════════════════════
#define BS_DISP_MULT      1.5     // displacement : corps ≥ 1.5×ATR
#define BS_TRADE_MIN      7       // force /10 minimale pour trader (Pine _tradeMinScore)
#define BS_PROX_ATR       8.0     // proximité : close à ≤ 8×ATR du bord
#define BS_BASE_GATE      6       // socle minimal à la naissance
#define BS_MAX_ZONES      20      // FIFO par sens
#define BS_TO10_DENOM     27.0

struct BST {
    double top, bot;
    int    state;        // 0 FRESH · 1 PARTIAL · 2 DEEP
    int    baseScore;    // socle figé /16
    int    score;        // force /10 dynamique
    bool   inOte;        // OTE figé à la création
    bool   signaled;
    int    barIdx;       // bar de création
};
BST g_bsBull[BS_MAX_ZONES];  int g_nBsBull = 0;
BST g_bsBear[BS_MAX_ZONES];  int g_nBsBear = 0;
int g_bsBosBullBar = -1, g_bsBosBearBar = -1;     // close-cross sh1/sl1 (Pine 3030-3037)
int g_bsChochBullBar = -1, g_bsChochBearBar = -1; // confirmation CHOCH (Pine 490-495)
int g_bsH1Trend = 0, g_bsH4Trend = 0;             // exposés par f_htfCompute
bool g_showBSZones = true;

void BsShiftBull() { for(int k = 0; k < BS_MAX_ZONES - 1; k++) g_bsBull[k] = g_bsBull[k + 1]; g_nBsBull = BS_MAX_ZONES - 1; }
void BsShiftBear() { for(int k = 0; k < BS_MAX_ZONES - 1; k++) g_bsBear[k] = g_bsBear[k + 1]; g_nBsBear = BS_MAX_ZONES - 1; }

// ── Barème de scoring (spec Rust bs_helpers, valeurs exactes) ──
int BsDispScore(double body, double atr, double body3) {
    double r1 = (atr > 0.0) ? body / atr : 0.0;
    double r3 = (atr > 0.0) ? body3 / atr : 0.0;
    double ratio = MathMax(r1, r3);
    if(ratio >= 3.5) return 3;
    if(ratio >= 2.5) return 2;
    return 1;
}
int BsSweepScore(int sweepBar, int barIdx) {
    if(sweepBar < 0) return 0;
    int ago = barIdx - sweepBar;
    if(ago <= 2) return 2;
    if(ago <= 5) return 1;
    return 0;
}
int BsBosScore(int bosBar, int barIdx)   { return (bosBar >= 0 && barIdx - bosBar <= 3) ? 3 : 0; }
int BsChochScore(int chochBar, int barIdx){ return (chochBar >= 0 && barIdx - chochBar <= 5) ? 2 : 0; }
int BsBodyRangeScore(double br)          { return (br >= 0.7) ? 1 : 0; }
int BsVolScore(double vol, double sma) {
    if(sma <= 0.0) return 0;
    if(vol >= 2.0 * sma) return 2;
    if(vol >= 1.5 * sma) return 1;
    return 0;
}
int BsMitScore(int st) { return st == 0 ? 3 : (st == 1 ? 1 : 0); }
int BsProxScore(double price, double top, double bot, double atr) {
    double dist = MathMin(MathAbs(price - top), MathAbs(price - bot));
    double ratio = (atr > 0.0) ? dist / atr : 999.0;
    if(ratio <= 1.0) return 2;
    if(ratio <= 3.0) return 1;
    return 0;
}
int BsFvgScore(bool hasFvg)  { return hasFvg ? 1 : 0; }
int BsOteScore(bool inOte)   { return inOte ? 1 : 0; }
int BsCoeurScore(bool hasFvg, bool inOte) { return (hasFvg && inOte) ? 3 : 0; }
int BsPdScore(bool isBull, double closeP, double eq) {
    if(eq <= 0.0) return 0;
    if(isBull)  return (closeP < eq) ? 1 : 0;
    return (closeP > eq) ? 1 : 0;
}
int BsKzScore(bool inKz, bool inDeadZone) {
    if(inDeadZone) return 0;
    return inKz ? 1 : 0;
}
int BsHtfScore(bool isBull, int h1, int h4) {
    int d = isBull ? 1 : -1;
    if(h1 == d && h4 == d) return 2;
    if(h1 == d || h4 == d) return 1;
    return 0;
}
int BsToForce10(int raw) {
    int sc = (int)MathRound(raw * 10.0 / BS_TO10_DENOM);
    return (int)MathMin(10, MathMax(0, sc));
}
// Dyn /11 (lifecycle : OTE FIGÉ de la zone — Pine 3348).
int BsDynScore(bool isBull, double closeP, double atr, double top, double bot,
               bool hasFvg, bool inOte, bool inKz, bool inDeadZone) {
    return BsProxScore(closeP, top, bot, atr)
         + BsFvgScore(hasFvg)
         + BsPdScore(isBull, closeP, g_pdEquilibrium)
         + BsKzScore(inKz, inDeadZone)
         + BsHtfScore(isBull, g_bsH1Trend, g_bsH4Trend)
         + BsOteScore(inOte)
         + BsCoeurScore(hasFvg, inOte);
}
// Zone a un FVG chevauchant (même sens).
bool BsHasFvg(bool isBull, double top, double bot) {
    if(isBull) {
        for(int k = 0; k < g_nFvgBull; k++)
            if(g_fvgBull[k].top > bot && g_fvgBull[k].bot < top) return true;
    } else {
        for(int k = 0; k < g_nFvgBear; k++)
            if(g_fvgBear[k].top > bot && g_fvgBear[k].bot < top) return true;
    }
    return false;
}
// Dead-zone NY lunch : minutes UTC ∈ [960, 1080[ (16h-18h).
bool BsInDeadZone(datetime t) {
    long mins = (long)(t % 86400) / 60;
    return (mins >= 960 && mins < 1080);
}
// Fenêtre sweep fresh TF-adaptive : max(2, round(9000/tf_sec)).
int BsSweepFreshBars() {
    int tfSec = PeriodSeconds(_Period);
    if(tfSec <= 0) return 10;
    return (int)MathMax(2, MathRound(9000.0 / tfSec));
}

#define BSVOL_PERIOD 20   // fenêtre SMA volume BSZones (bars [1..20], courant exclu)

// ── Corps 3 bougies (même sens uniquement — Pine/Rust body_delta) ──
double BsBodyDelta(const double &open[], const double &close[], int i, int k, bool isBull) {
    if(i < k) return 0.0;
    double pBody;
    if(isBull) {
        if(close[i] > open[i] && close[i - k] > open[i - k]) pBody = close[i - k] - open[i - k];
        else pBody = 0.0;
    } else {
        if(close[i] < open[i] && close[i - k] < open[i - k]) pBody = open[i - k] - close[i - k];
        else pBody = 0.0;
    }
    return pBody;
}

// ── Compute par bar : tracking BOS/CHOCH + naissances + lifecycle ──
void f_bsCompute(int i, const datetime &time[], const double &open[], const double &high[],
                 const double &low[], const double &close[], const long &tick_volume[],
                 double atr14) {
    // Tracking BOS close-cross (Pine 3030-3037 — NON masqué MSS ; g_sh1/g_sl1
    // du bar précédent, prevClose = close[i-1]).
    if(i >= 1) {
        double pc = close[i - 1];
        if(g_sh1 != 0 && close[i] > g_sh1 && pc <= g_sh1) g_bsBosBullBar = i;
        if(g_sl1 != 0 && close[i] < g_sl1 && pc >= g_sl1) g_bsBosBearBar = i;
    }
    if(g_chochHaussier) g_bsChochBullBar = i;
    if(g_chochBaissier) g_bsChochBearBar = i;

    if(i < 3) return;   // besoin de i-1 (OB), i-2 (body3)

    // Contexte vol : volume[1] vs SMA20 fenêtre [1..20] (courant exclu).
    double volSma = 0.0;
    if(i >= BSVOL_PERIOD) {
        double sum = 0.0;
        for(int v = i - BSVOL_PERIOD; v <= i - 1; v++) sum += (double)tick_volume[v];
        volSma = sum / (double)BSVOL_PERIOD;
    }
    double vol1 = (double)tick_volume[i - 1];
    // Body/range de la bougie OB (i-1).
    double obBody = MathAbs(close[i - 1] - open[i - 1]);
    double obRange = high[i - 1] - low[i - 1];
    double obBR = (obRange > 0.0) ? obBody / obRange : 0.0;

    int fresh = BsSweepFreshBars();
    bool sweepRecentBull = (g_dernierSweepH_bar >= 0) && ((i - g_dernierSweepH_bar) <= fresh);
    bool sweepRecentBear = (g_dernierSweepB_bar >= 0) && ((i - g_dernierSweepB_bar) <= fresh);

    double body1 = MathAbs(close[i] - open[i]);
    double bb2B = BsBodyDelta(open, close, i, 1, true);
    double bb3B = (bb2B > 0.0) ? BsBodyDelta(open, close, i, 2, true) : 0.0;
    double body3Bull = body1 + bb2B + bb3B;
    double bb2R = BsBodyDelta(open, close, i, 1, false);
    double bb3R = (bb2R > 0.0) ? BsBodyDelta(open, close, i, 2, false) : 0.0;
    double body3Bear = body1 + bb2R + bb3R;

    bool dispBull = (close[i] > open[i]) && ((close[i] - open[i]) >= BS_DISP_MULT * atr14);
    bool dispBear = (open[i] > close[i]) && ((open[i] - close[i]) >= BS_DISP_MULT * atr14);

    bool inKz = InKzAt(time[i]);
    bool inDz = BsInDeadZone(time[i]);
    double obT = high[i - 1];
    double obB = low[i - 1];

    // ── Naissances (gate : base ≥ 6 ET H1/H4 aligné) ──
    if(dispBull && sweepRecentBull) {
        int base = BsDispScore(body1, atr14, body3Bull)
                 + BsSweepScore(g_dernierSweepH_bar, i)
                 + 3                                  // VIERGE à la création
                 + BsBosScore(g_bsBosBullBar, i)
                 + BsChochScore(g_bsChochBullBar, i)
                 + BsBodyRangeScore(obBR)
                 + BsVolScore(vol1, volSma);
        if(base >= BS_BASE_GATE && (g_bsH1Trend == 1 || g_bsH4Trend == 1)) {
            bool ote = g_inOTE_bull;                  // OTE COURANT à la naissance
            int dyn = BsDynScore(true, close[i], atr14, obT, obB,
                                 BsHasFvg(true, obT, obB), ote, inKz, inDz);
            if(g_nBsBull >= BS_MAX_ZONES) BsShiftBull();
            g_bsBull[g_nBsBull].top = obT;  g_bsBull[g_nBsBull].bot = obB;
            g_bsBull[g_nBsBull].state = 0;  g_bsBull[g_nBsBull].baseScore = base;
            g_bsBull[g_nBsBull].score = BsToForce10(base + dyn);
            g_bsBull[g_nBsBull].inOte = ote; g_bsBull[g_nBsBull].signaled = false;
            g_bsBull[g_nBsBull].barIdx = i;
            g_nBsBull++;
        }
    }
    if(dispBear && sweepRecentBear) {
        int base = BsDispScore(body1, atr14, body3Bear)
                 + BsSweepScore(g_dernierSweepB_bar, i)
                 + 3
                 + BsBosScore(g_bsBosBearBar, i)
                 + BsChochScore(g_bsChochBearBar, i)
                 + BsBodyRangeScore(obBR)
                 + BsVolScore(vol1, volSma);
        if(base >= BS_BASE_GATE && (g_bsH1Trend == -1 || g_bsH4Trend == -1)) {
            bool ote = g_inOTE_bear;
            int dyn = BsDynScore(false, close[i], atr14, obT, obB,
                                 BsHasFvg(false, obT, obB), ote, inKz, inDz);
            if(g_nBsBear >= BS_MAX_ZONES) BsShiftBear();
            g_bsBear[g_nBsBear].top = obT;  g_bsBear[g_nBsBear].bot = obB;
            g_bsBear[g_nBsBear].state = 0;  g_bsBear[g_nBsBear].baseScore = base;
            g_bsBear[g_nBsBear].score = BsToForce10(base + dyn);
            g_bsBear[g_nBsBear].inOte = ote; g_bsBear[g_nBsBear].signaled = false;
            g_bsBear[g_nBsBear].barIdx = i;
            g_nBsBear++;
        }
    }

    // ── Lifecycle (Pine 3318-3408) : invalidation, mitigation, recalcul dyn ──
    for(int k = g_nBsBull - 1; k >= 0; k--) {
        if(low[i] < g_bsBull[k].bot) {   // invalidation → retrait
            for(int j = k; j < g_nBsBull - 1; j++) g_bsBull[j] = g_bsBull[j + 1];
            g_nBsBull--;
            continue;
        }
        double mid = (g_bsBull[k].top + g_bsBull[k].bot) * 0.5;
        int st = g_bsBull[k].state;
        int nst = st;
        if(low[i] <= g_bsBull[k].top) {
            if(close[i] <= mid && st < 2)      nst = 2;
            else if(close[i] > mid && st == 0) nst = 1;
        }
        int nbase = (nst != st) ? g_bsBull[k].baseScore - BsMitScore(st) + BsMitScore(nst)
                                : g_bsBull[k].baseScore;
        int dyn = BsDynScore(true, close[i], atr14, g_bsBull[k].top, g_bsBull[k].bot,
                             BsHasFvg(true, g_bsBull[k].top, g_bsBull[k].bot),
                             g_bsBull[k].inOte, inKz, inDz);   // OTE FIGÉ
        g_bsBull[k].state = nst;
        g_bsBull[k].baseScore = nbase;
        g_bsBull[k].score = BsToForce10(nbase + dyn);
    }
    for(int k = g_nBsBear - 1; k >= 0; k--) {
        if(high[i] > g_bsBear[k].top) {
            for(int j = k; j < g_nBsBear - 1; j++) g_bsBear[j] = g_bsBear[j + 1];
            g_nBsBear--;
            continue;
        }
        double mid = (g_bsBear[k].top + g_bsBear[k].bot) * 0.5;
        int st = g_bsBear[k].state;
        int nst = st;
        if(high[i] >= g_bsBear[k].bot) {
            if(close[i] >= mid && st < 2)      nst = 2;
            else if(close[i] < mid && st == 0) nst = 1;
        }
        int nbase = (nst != st) ? g_bsBear[k].baseScore - BsMitScore(st) + BsMitScore(nst)
                                : g_bsBear[k].baseScore;
        int dyn = BsDynScore(false, close[i], atr14, g_bsBear[k].top, g_bsBear[k].bot,
                             BsHasFvg(false, g_bsBear[k].top, g_bsBear[k].bot),
                             g_bsBear[k].inOte, inKz, inDz);
        g_bsBear[k].state = nst;
        g_bsBear[k].baseScore = nbase;
        g_bsBear[k].score = BsToForce10(nbase + dyn);
    }
}


// ── MOTEUR BSZONES — génération de trades (coexistence avec le moteur OB) ──
// Miroir Rust create_bs : force ≥ 7, retour (close > top && state < 2 bull),
// proximité ≤ 8×ATR, barre > création, 1 trade max par bar (après v11).
// TP3 = DoL≤3R SANS AsianHL (parité Pine : AsianHL = v11 uniquement).
void f_createBsBuySignals(int i, const datetime &time[], const double &close[], double atr14) {
    if(f_tradeBloquant()) return;
    // 1 trade max par bar : un signal v11 a-t-il déjà été poussé ?
    if(g_nSignals > 0 && g_signals[g_nSignals - 1].t == time[i]) return;
    for(int k = 0; k < g_nBsBull; k++) {
        if(g_bsBull[k].signaled) continue;
        if(i <= g_bsBull[k].barIdx) continue;           // barre > création (score finalisé)
        double top = g_bsBull[k].top, bot = g_bsBull[k].bot;
        bool proche = (atr14 <= 0.0) || ((close[i] - top) <= BS_PROX_ATR * atr14);
        bool retour = (close[i] > top) && (g_bsBull[k].state < 2);
        if(!retour || !proche) continue;
        if(g_bsBull[k].score < BS_TRADE_MIN) continue;

        double entry = top;                              // TR forcé au bord de zone
        double sl = (g_autoSlMode == "1.5× ATR sous OB") ? bot - 1.5 * 0.75 * atr14 :
                    (g_autoSlMode == "1× ATR sous OB")   ? bot - 0.75 * atr14 :
                    (g_autoSlMode == "2× ATR sous OB")   ? bot - 2.0 * 0.75 * atr14 : bot;
        double r = entry - sl;
        double slMax = SlMaxForAsset(atr14), slMin = SlMinForAsset(atr14);
        if(r > 2.0 * slMax) continue;
        r  = MathMax(slMin, MathMin(slMax, r));
        sl = entry - r;
        double tp1 = entry + 0.6 * r;                    // étape 4 29/08
        double tp2 = entry + 2.0 * r;
        double tp3Cap = entry + 3.0 * r;                 // Phase 2 : DoL ≤ 3R (SANS AsianHL)
        double tp3 = tp3Cap;
        double m3 = 1e15;
        if(g_dernierEQH_level > 0.0 && g_dernierEQH_level > entry) m3 = MathMin(m3, g_dernierEQH_level);
        if(g_pdhActive > 0.0 && g_pdhActive > entry) m3 = MathMin(m3, g_pdhActive);
        if(g_pwhActive > 0.0 && g_pwhActive > entry) m3 = MathMin(m3, g_pwhActive);
        if(m3 < 1e15 && m3 >= tp2) tp3 = MathMin(m3, tp3Cap);

        SignalT sig;
        sig.t = time[i]; sig.entry = entry; sig.sl = sl;
        sig.tp1 = tp1; sig.tp2 = tp2; sig.tp3 = tp3;
        sig.force = g_bsBull[k].score; sig.score = g_bsBull[k].baseScore;
        sig.bull = true;
        sig.obIdx = -1;                                  // source BS : court-circuite _scoreDeg
        sig.openTs = time[i];
        sig.filled = false; sig.fillT = 0; sig.t1Hit = false; sig.t2Ts = 0;
        ArrayResize(g_signals, g_nSignals + 1);
        g_signals[g_nSignals] = sig;
        g_nSignals++;
        g_bsBull[k].signaled = true;
        return;                                          // 1 trade max par bar
    }
}

void f_createBsSellSignals(int i, const datetime &time[], const double &close[], double atr14) {
    if(f_tradeBloquant()) return;
    if(g_nSignals > 0 && g_signals[g_nSignals - 1].t == time[i]) return;
    for(int k = 0; k < g_nBsBear; k++) {
        if(g_bsBear[k].signaled) continue;
        if(i <= g_bsBear[k].barIdx) continue;
        double top = g_bsBear[k].top, bot = g_bsBear[k].bot;
        bool proche = (atr14 <= 0.0) || ((bot - close[i]) <= BS_PROX_ATR * atr14);
        bool retour = (close[i] < bot) && (g_bsBear[k].state < 2);
        if(!retour || !proche) continue;
        if(g_bsBear[k].score < BS_TRADE_MIN) continue;

        double entry = bot;
        double sl = (g_autoSlMode == "1.5× ATR sous OB") ? top + 1.5 * 0.75 * atr14 :
                    (g_autoSlMode == "1× ATR sous OB")   ? top + 0.75 * atr14 :
                    (g_autoSlMode == "2× ATR sous OB")   ? top + 2.0 * 0.75 * atr14 : top;
        double r = sl - entry;
        double slMax = SlMaxForAsset(atr14), slMin = SlMinForAsset(atr14);
        if(r > 2.0 * slMax) continue;
        r  = MathMax(slMin, MathMin(slMax, r));
        sl = entry + r;
        double tp1 = entry - 0.6 * r;
        double tp2 = entry - 2.0 * r;
        double tp3Cap = entry - 3.0 * r;
        double tp3 = tp3Cap;
        double m3 = -1e15;
        if(g_dernierEQL_level > 0.0 && g_dernierEQL_level < entry) m3 = MathMax(m3, g_dernierEQL_level);
        if(g_pdlActive > 0.0 && g_pdlActive < entry) m3 = MathMax(m3, g_pdlActive);
        if(g_pwlActive > 0.0 && g_pwlActive < entry) m3 = MathMax(m3, g_pwlActive);
        if(m3 > -1e15 && m3 <= tp2) tp3 = MathMax(m3, tp3Cap);

        SignalT sig;
        sig.t = time[i]; sig.entry = entry; sig.sl = sl;
        sig.tp1 = tp1; sig.tp2 = tp2; sig.tp3 = tp3;
        sig.force = g_bsBear[k].score; sig.score = g_bsBear[k].baseScore;
        sig.bull = false;
        sig.obIdx = -1;
        sig.openTs = time[i];
        sig.filled = false; sig.fillT = 0; sig.t1Hit = false; sig.t2Ts = 0;
        ArrayResize(g_signals, g_nSignals + 1);
        g_signals[g_nSignals] = sig;
        g_nSignals++;
        g_bsBear[k].signaled = true;
        return;
    }
}


void f_drawBsZones(const datetime &time[]) {
    if(!g_showBSZones) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "BSZ");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period);
    for(int k = 0; k < g_nBsBull; k++) {
        int idx = g_bsBull[k].barIdx;
        if(idx < 0 || idx >= rates_total) continue;
        string nm = SMC_PREFIX + "BSZ_B_" + IntegerToString(k);
        if(ObjectFind(0, nm) < 0) {
            ObjectCreate(0, nm, OBJ_RECTANGLE, 0, time[idx], g_bsBull[k].top, tEnd, g_bsBull[k].bot);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        ObjectSetInteger(0, nm, OBJPROP_COLOR, g_bsBull[k].signaled ? clrDarkGray : clrLimeGreen);
        ObjectMove(0, nm, 0, time[idx], g_bsBull[k].top);
        ObjectMove(0, nm, 1, tEnd, g_bsBull[k].bot);
        string etoiles = "";
        for(int e = 0; e < g_bsBull[k].score; e++) etoiles += "★";
        SmcNewLabel("BSZ", tEnd, g_bsBull[k].top, "BS " + etoiles + " " + IntegerToString(g_bsBull[k].score) + "/10",
                    g_bsBull[k].signaled ? clrDarkGray : clrLimeGreen, 0);
    }
    for(int k = 0; k < g_nBsBear; k++) {
        int idx = g_bsBear[k].barIdx;
        if(idx < 0 || idx >= rates_total) continue;
        string nm = SMC_PREFIX + "BSZ_S_" + IntegerToString(k);
        if(ObjectFind(0, nm) < 0) {
            ObjectCreate(0, nm, OBJ_RECTANGLE, 0, time[idx], g_bsBear[k].top, tEnd, g_bsBear[k].bot);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        ObjectSetInteger(0, nm, OBJPROP_COLOR, g_bsBear[k].signaled ? clrDarkGray : clrCrimson);
        ObjectMove(0, nm, 0, time[idx], g_bsBear[k].top);
        ObjectMove(0, nm, 1, tEnd, g_bsBear[k].bot);
        string etoiles = "";
        for(int e = 0; e < g_bsBear[k].score; e++) etoiles += "★";
        SmcNewLabel("BSZ", tEnd, g_bsBear[k].top, "BS " + etoiles + " " + IntegerToString(g_bsBear[k].score) + "/10",
                    g_bsBear[k].signaled ? clrDarkGray : clrCrimson, 0);
    }
    SmcFifo("BSZ", MAX_VIS_LINES);
}

// === SECTION 14 : Affichage MT5 (objets graphiques) ===
// Helper FIFO : crée un label avec préfixe SMC_, gère un suffixe unique.
// Pine équivalent : array.push + array.shift si size >= max.

int g_objCounter = 0;   // suffixe unique incrémental

string SmcNewLabel(string baseName, datetime t, double price, string text,
                   color clr, int anchorMode) {
    // anchorMode : 0 = label_up (au-dessus du prix, pointe vers le bas),
    //              1 = label_down (sous le prix, pointe vers le haut)
    string name = SMC_PREFIX + baseName + "_" + IntegerToString(g_objCounter++);
    if(ObjectCreate(0, name, OBJ_TEXT, 0, t, price)) {
        ObjectSetString (0, name, OBJPROP_TEXT,     text);
        ObjectSetInteger(0, name, OBJPROP_COLOR,    clr);
        ObjectSetInteger(0, name, OBJPROP_FONTSIZE, 8);
        ObjectSetInteger(0, name, OBJPROP_ANCHOR,
                         anchorMode == 0 ? ANCHOR_LEFT_LOWER : ANCHOR_LEFT_UPPER);
    }
    return name;
}

string SmcNewLine(string baseName, datetime t1, double p1, datetime t2, double p2,
                  color clr, int width, int style) {
    string name = SMC_PREFIX + baseName + "_" + IntegerToString(g_objCounter++);
    if(ObjectCreate(0, name, OBJ_TREND, 0, t1, p1, t2, p2)) {
        ObjectSetInteger(0, name, OBJPROP_COLOR,    clr);
        ObjectSetInteger(0, name, OBJPROP_WIDTH,    width);
        ObjectSetInteger(0, name, OBJPROP_STYLE,    style);
        ObjectSetInteger(0, name, OBJPROP_RAY_RIGHT, false);
    }
    return name;
}

// Label de bloc superposé au coin haut-droit d'un rectangle (FVG, OB, etc.).
// OBJ_RECTANGLE ne supporte pas OBJPROP_TEXT (corrompt le rendu → blocs invisibles,
// constaté + doc forum MQL5). On superpose donc un OBJ_TEXT séparé au bord droit du
// rectangle, ancré en haut-droit, avec le nom du bloc.
// tRight = timestamp du bord droit (tEnd du rectangle) ; price = niveau haut du rectangle.
void SmcBlockLabel(string baseName, datetime tRight, double price, string txt, color clr, color txtColor = clrWhite, ENUM_ANCHOR_POINT anchor = ANCHOR_RIGHT_UPPER) {
    string name = SMC_PREFIX + baseName;
    if(ObjectCreate(0, name, OBJ_TEXT, 0, tRight, price)) {
        ObjectSetString (0, name, OBJPROP_TEXT,     txt);
        ObjectSetInteger(0, name, OBJPROP_COLOR,    txtColor);   // couleur du TEXTE (pas la ligne)
        ObjectSetInteger(0, name, OBJPROP_FONTSIZE, 8);
        ObjectSetInteger(0, name, OBJPROP_ANCHOR,   anchor);
        ObjectSetInteger(0, name, OBJPROP_SELECTABLE, false);
    }
}

// FIFO : supprime les plus anciens objets d'un préfixe si > max.
// Pine équivalent : if array.size >= max : delete(shift).
// ATTENTION MQL5 : ObjectsTotal a 2 surcharges. La forme (chart_id, sub_window, type) prend
// type=INT ; pour filtrer par préfixe il FAUT la surcharge (chart_id, prefix, sub_window, type)
// avec prefix en 2e position (string). Sinon conversion string→0 → FIFO compte tous les
// objets du chart (bug). ObjectName n'a pas de version prefix → on collecte puis supprime.
void SmcFifo(string baseName, int maxKeep) {
    string prefix = SMC_PREFIX + baseName;
    // MQL5 n'expose qu'une surcharge ObjectsTotal(chart_id, sub_window, type) (int), pas de
    // version prefix. On compte donc les objets du préfixe en parcourant tous les noms.
    // ObjectName(chart_id, pos, sub_window, type) n'a pas non plus de version prefix.
    // On collecte les noms du préfixe (1 passe), puis on supprime les plus anciens (1 passe).
    string names[];
    int n = ObjectsTotal(0);
    for(int i = 0; i < n; i++) {
        string nm = ObjectName(0, i);
        if(StringFind(nm, prefix) == 0) {
            int sz = ArraySize(names);
            ArrayResize(names, sz + 1);
            names[sz] = nm;
        }
    }
    int total = ArraySize(names);
    if(total <= maxKeep) return;
    // Les premiers collectés sont les plus anciens (ordre ObjectName). On en supprime
    // total - maxKeep (fidèle au Pine array.shift qui retire le plus ancien).
    int toRemove = total - maxKeep;
    for(int i = 0; i < toRemove; i++) {
        ObjectDelete(0, names[i]);
    }
}

// Convertit un bar_index absolu Pine en datetime.
// En MQL5 les séries OnCalculate sont chronologiques : time[0] = 1er bar, time[i] = i-ème.
// Comme g_barIndex[i] = i (index absolu Pine-équivalent), pineIdx pointe directement la
// bonne case de time[] : time[pineIdx].
datetime TimeFromBarIndex(const datetime &time[], int rates_total, int pineIdx) {
    if(pineIdx < 0 || pineIdx >= rates_total) return (rates_total > 0 ? time[0] : 0);
    return time[pineIdx];
}

// === SECTION 14 (suite) : affichage BOS ===
// Colors Pine lignes 338-382 (#2962FF, #FF6D00, #00BCD4, #FF9800, #AA00FF, #FF1744)
#define C_BOS_BULL     clrDodgerBlue    // #2962FF ≈
#define C_BOS_BEAR     clrOrange         // #FF6D00 ≈
#define C_MSS_BULL     clrDarkTurquoise  // #00BCD4 ≈
#define C_MSS_BEAR     clrOrange         // #FF9800 ≈
#define C_CHOCH_BULL   clrPurple         // #AA00FF ≈
#define C_CHOCH_BEAR   clrCrimson        // #FF1744 ≈
// Couleurs labels structure HH/HL/LH/LL (Pine lignes 242-245).
#define C_HH clrLimeGreen   // #00C853 vert
#define C_HL clrLightGreen  // #69F0AE vert clair
#define C_LH clrRed         // #FF5252 rouge
#define C_LL clrCrimson     // #D50000 rouge foncé
// Couleurs labels Sweep (Pine lignes 584-585, MODULE 5).
#define C_SWEEP_BULL clrLime        // #00E676 vert
#define C_SWEEP_BEAR clrRed         // #FF1744 rouge vif

void f_drawStructureBOS(int i, const datetime &time[]) {
    if(!g_showBOS) return;
    datetime tNow = time[i];
    int rates_total = ArraySize(time);   // pour le check de bounds dans TimeFromBarIndex

    // Pine f_block17 : bosHaussier and not mssHaussier and not na(bsh1)
    if(g_bosHaussier && !g_mssHaussier && g_bsh1 != 0) {
        datetime tPiv = TimeFromBarIndex(time, rates_total, g_bsh1);
        SmcNewLine("BOS", tPiv, g_sh1, tNow, g_sh1, C_BOS_BULL, 2, STYLE_SOLID);
        SmcNewLabel("BOS", tNow, g_sh1, "Break Of Structure " + CharToString(254), C_BOS_BULL, 1);
        SmcFifo("BOS", MAX_VIS_LINES);
    }
    if(g_bosBaissier && !g_mssBaissier && g_bsl1 != 0) {
        datetime tPiv = TimeFromBarIndex(time, rates_total, g_bsl1);
        SmcNewLine("BOS", tPiv, g_sl1, tNow, g_sl1, C_BOS_BEAR, 2, STYLE_SOLID);
        SmcNewLabel("BOS", tNow, g_sl1, "Break Of Structure " + CharToString(253), C_BOS_BEAR, 0);
        SmcFifo("BOS", MAX_VIS_LINES);
    }
}

// === SECTION 14 (suite) : affichage FVG ===
// Pine : box.new(bar_index[2], top, bar_index+50, bot) avec recoloration PARTIAL sur mitigation.
// Option B (brief J2) : f_drawFVG appelée APRÈS la boucle (jamais dedans) pour dessiner
// tous les FVG encore présents dans les arrays (state 0=actif, 1=mitigé). Comme f_fvgLifecycle
// supprime déjà les FVG comblés/expirés, l'état des arrays à la fin de la boucle reflète
// exactement ce que le Pine affiche à l'instant courant — comportement 1:1.
// Phase 1.1 : Uniformisation Vert/Rouge (comme v11/v12 Pine). Niveau 2 (estompé).
#define C_FVG_BULL_FRESH   clrLimeGreen    // #00C853 vert (vif, comme OB)
#define C_FVG_BULL_PARTIAL clrSeaGreen     // #00C853 vert mitigé (plus foncé)
#define C_FVG_BEAR_FRESH   clrCrimson      // #D50000 rouge (vif, comme OB)
#define C_FVG_BEAR_PARTIAL clrFireBrick    // #D50000 rouge mitigé (plus foncé)

void f_drawFVG(const datetime &time[]) {
    if(!g_showFVG) return;
    // Nettoyer les anciens rectangles + labels FVG puis tout redessiner.
    ObjectsDeleteAll(0, SMC_PREFIX + "FVG");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    // Bord droit = bar courant + 50 bars (harmonisé avec les OB, demande Rono 2026-07-23).
    // Fidèle au Pine box.set_right(bar_index+50) où bar_index = bar courant.
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante (uniformisation 2026-07-26)

    // OBJ_RECTANGLE : FILL=true + BACK=false donne un fond coloré (test Rono : FILL=true +
    // BACK=true rend invisible chez lui — bug MT5 documenté forum MQL5). Le texte n'est pas
    // supporté sur OBJ_RECTANGLE (corrompt le rendu) → on superpose un OBJ_TEXT séparé.
    for(int k = 0; k < g_nFvgBull; k++) {
        string nm = SMC_PREFIX + "FVG_B_" + IntegerToString(k);
        int idx = g_fvgBull[k].barIdx;
        if(idx < 0 || idx >= rates_total) continue;
        color clr = g_fvgBull[k].state == 0 ? C_FVG_BULL_FRESH : C_FVG_BULL_PARTIAL;
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        time[idx],   g_fvgBull[k].top,
                        tEnd,        g_fvgBull[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        SmcBlockLabel("FVG_LB_" + IntegerToString(k), tEnd, g_fvgBull[k].top, "FVG", clr);
    }
    for(int k = 0; k < g_nFvgBear; k++) {
        string nm = SMC_PREFIX + "FVG_S_" + IntegerToString(k);
        int idx = g_fvgBear[k].barIdx;
        if(idx < 0 || idx >= rates_total) continue;
        color clr = g_fvgBear[k].state == 0 ? C_FVG_BEAR_FRESH : C_FVG_BEAR_PARTIAL;
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        time[idx],   g_fvgBear[k].top,
                        tEnd,        g_fvgBear[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        SmcBlockLabel("FVG_LS_" + IntegerToString(k), tEnd, g_fvgBear[k].top, "FVG", clr);
    }
    ChartRedraw(0);
}

// === SECTION 14 (suite) : affichage Order Blocks ===
// Pine colors lignes 816-823 (#00C853 green pour bull, #D50000 red pour bear).
// Recoloration selon state : FRESH (0), PARTIAL (1), DEEP (2) — fidèle au Pine
// (box.set_bgcolor avec C_OB_BULL_FRESH / C_OB_BULL_PART / C_OB_BULL_DEEP).
// Option B (comme f_drawFVG) : appelée APRÈS la boucle OnCalculate (jamais dedans),
// nettoie puis dessine tous les OB encore présents dans les arrays. Les OB supprimés
// par f_obLifecycle ne sont plus là, donc l'état final reflète le Pine à l'instant T.
#define C_OB_BULL_FRESH   clrLimeGreen   // #00C853 70% ≈
#define C_OB_BULL_PARTIAL clrSeaGreen    // #00C853 83% (PARTIAL, plus foncé)
#define C_OB_BULL_DEEP    clrDarkGreen   // #00C853 91% (DEEP, mitigation ≥ 50%)
#define C_OB_BEAR_FRESH   clrCrimson     // #D50000 70% ≈
#define C_OB_BEAR_PARTIAL clrFireBrick   // #D50000 83% (PARTIAL)
#define C_OB_BEAR_DEEP    clrMaroon      // #D50000 91% (DEEP)

void f_drawOB(const datetime &time[]) {
    if(!g_showOB) return;
    // Nettoyer les anciens rectangles + labels OB puis tout redessiner (une fois par OnCalculate).
    ObjectsDeleteAll(0, SMC_PREFIX + "OB");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    // Bord droit = bar courant + 50 bars (fidèle Pine box.set_right(_bxK, bar_index+50),
    // où bar_index = bar courant, PAS bar de création). Les boîtes grandissent au fil des
    // bars jusqu'à suppression de l'OB. Les OB anciens NON supprimés restent donc visibles
    // — c'est normal et fidèle au Pine.
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante (uniformisation 2026-07-26)

    // OB BULL : _rt = top, _rb = bot.
    for(int k = 0; k < g_nObBull; k++) {
        string nm = SMC_PREFIX + "OB_B_" + IntegerToString(k);
        datetime t0 = g_obBull[k].t0;
        color clr = (g_obBull[k].state == 2) ? C_OB_BULL_DEEP   :
                    (g_obBull[k].state == 1) ? C_OB_BULL_PARTIAL :
                                               C_OB_BULL_FRESH;
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        t0, g_obBull[k].top,
                        tEnd, g_obBull[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        string obTxtB = "OB " + IntegerToString(f_force(g_obBull[k].score)) + "/10";
        SmcBlockLabel("OB_LB_" + IntegerToString(k), tEnd, g_obBull[k].top, obTxtB, clr);
    }
    // OB BEAR (symétrique)
    for(int k = 0; k < g_nObBear; k++) {
        string nm = SMC_PREFIX + "OB_S_" + IntegerToString(k);
        datetime t0 = g_obBear[k].t0;
        color clr = (g_obBear[k].state == 2) ? C_OB_BEAR_DEEP   :
                    (g_obBear[k].state == 1) ? C_OB_BEAR_PARTIAL :
                                               C_OB_BEAR_FRESH;
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        t0, g_obBear[k].top,
                        tEnd, g_obBear[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        string obTxtR = "OB " + IntegerToString(f_force(g_obBear[k].score)) + "/10";
        SmcBlockLabel("OB_LS_" + IntegerToString(k), tEnd, g_obBear[k].top, obTxtR, clr);
    }
    ChartRedraw(0);
}

// === SECTION 14 (suite) : affichage signaux BUY/SELL ===
// Pine : plotshape(arrow) + lignes entry/SL/TP1 (line.new) horizontales sur ~30 bars.
// Option B : f_drawSignals appelée APRÈS la boucle (jamais dedans), comme f_drawFVG/f_drawOB.
// Nettoie puis redessine tous les signaux accumulés dans g_signals[] (état final = Pine T).
#define C_SIG_BUY   clrLime       // flèche / TP1 BUY
#define C_SIG_SELL  clrRed        // flèche / TP1 SELL
#define C_SIG_ENTRY clrWhite      // ligne entry
#define C_SIG_SL    clrRed        // ligne SL

// Couleurs signaux fidèles Pine (C_SL_BG, C_TP_BG, C_TP_BDR, C_TP1_L, C_TP2_L, C_LBL_BUY/SELL).
#define C_SL_BG   clrIndianRed      // Pine C_SL_BG = #ef5350 78% (rouge SL)
#define C_TP_BG   clrTeal           // Pine C_TP_BG = #26a69a 78% (vert TP)
#define C_SL_BDR  clrCrimson        // Pine C_SL_BDR = #ef5350 10% (bord rouge)
#define C_TP_BDR  clrDarkGreen      // Pine C_TP_BDR = #26a69a 10% (bord vert)
#define C_TP1_L   clrTeal           // Pine C_TP1_L = #26a69a 0%
#define C_TP2_L   clrTeal           // Pine C_TP2_L = #26a69a 0%
#define C_LBL_BUY  clrDarkGreen     // Pine C_LBL_BUY = #1b5e20 10%
#define C_LBL_SELL clrMaroon        // Pine C_LBL_SELL = #b71c1c 10%

void f_drawSignals(const datetime &time[]) {
    if(!g_showSignals) return;
    // Nettoyer anciens objets signaux puis tout redessiner.
    ObjectsDeleteAll(0, SMC_PREFIX + "SIG");
    int rates_total = ArraySize(time);
    if(rates_total < 2 || g_nSignals == 0) { ChartRedraw(0); return; }

    int barSecs = PeriodSeconds(_Period);
    datetime tNow = time[rates_total - 1] + (datetime)barSecs; // fin bougie courante
    for(int k = 0; k < g_nSignals; k++) {
        string base = "SIG_" + IntegerToString(k);

        // Pine : trade INVISIBLE tant que non rempli (objets créés transparents, rendus visibles au fill).
        // On n'affiche QUE les trades filled.
        if(!g_signals[k].filled) continue;

        // Bord gauche = bar de fill (Pine _fillL). Bord droit = closeT si fermé, sinon t+30 bars (Pine i_tpWidth).
        datetime t   = g_signals[k].fillT;
        datetime tEnd;
        if(g_signals[k].closed) {
            tEnd = g_signals[k].closeT;   // trade fermé : figer à la bar de clôture
        } else {
            tEnd = t + (datetime)(barSecs * 30);   // trade ouvert : +30 bars (Pine i_tpWidth)
            if(tEnd > tNow) tEnd = tNow;
        }
        bool isBuy   = g_signals[k].bull;
        color  arrowClr = isBuy ? C_SIG_BUY : C_SIG_SELL;
        double entry = g_signals[k].entry;
        double sl    = g_signals[k].sl;
        double tp1   = g_signals[k].tp1;
        double tp2   = g_signals[k].tp2;
        double tp3   = g_signals[k].tp3;

        // Tooltip commun : détails du trade + vie + résultat final.
        string rTxt = DoubleToString(entry > 0 ? (entry - sl) : 0, 2);
        string tip = (isBuy ? "BUY " : "SELL ") + IntegerToString(g_signals[k].force) + "/10"
                   + (g_signals[k].closed ? " [FERMÉ]" : (g_signals[k].filled ? " [ouvert]" : " [en attente]"))
                   + "\nEntry  " + DoubleToString(entry, _Digits)
                   + "\nSL     " + DoubleToString(sl, _Digits)
                   + "\nTP1    " + DoubleToString(tp1, _Digits)
                   + "\nTP2    " + DoubleToString(tp2, _Digits)
                   + "\nTP3    " + DoubleToString(tp3, _Digits);
        // Vie du trade + résultat final
        if(g_signals[k].closed) {
            string rsn = g_signals[k].closeRsn;
            if(rsn == "SL")
                tip += "\n\n→ Fermé: SL touché (-1R)";
            else if(rsn == "BE")
                tip += "\n\n→ TP1 touché → BE → Fermé au BE (+1R)";
            else if(rsn == "TP2SL")
                tip += "\n\n→ TP1 → TP2 touché → Fermé au TP1 (+1R)";
            else if(rsn == "TP3")
                tip += "\n\n→ TP1 → TP2 → TP3 touché (" + DoubleToString(g_signals[k].closeR, 1) + "R)";
            else if(rsn == "EXPIRE")
                tip += "\n\n→ Expiration temporelle (" + DoubleToString(g_signals[k].closeR, 1) + "R)";
            else if(rsn == "BOS")
                tip += "\n\n→ BOS contre tendance (non rempli)";
        } else if(g_signals[k].filled) {
            string vie = "\n\nVie: Fill";
            if(g_signals[k].t1Hit) { vie += " → TP1 (BE)"; if(g_signals[k].t2Ts > 0) vie += " → TP2"; }
            tip += vie;
        }

        // Box SL (entry → SL) : rectangle rouge (Pine stBullSLBox, C_SL_BG/C_SL_BDR).
        string nmSL = SMC_PREFIX + base + "_SLBX";
        if(ObjectCreate(0, nmSL, OBJ_RECTANGLE, 0, t, entry, tEnd, sl)) {
            ObjectSetInteger(0, nmSL, OBJPROP_COLOR,     C_SL_BDR);
            ObjectSetInteger(0, nmSL, OBJPROP_FILL,      true);
            ObjectSetInteger(0, nmSL, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nmSL, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nmSL, OBJPROP_WIDTH,     1);
            ObjectSetString (0, nmSL, OBJPROP_TOOLTIP,   tip);
        }

        // Box TP (entry → TP3) : rectangle vert (Pine stBullTPBox, C_TP_BG/C_TP_BDR).
        string nmTP = SMC_PREFIX + base + "_TPBX";
        if(ObjectCreate(0, nmTP, OBJ_RECTANGLE, 0, t, tp3, tEnd, entry)) {
            ObjectSetInteger(0, nmTP, OBJPROP_COLOR,     C_TP_BDR);
            ObjectSetInteger(0, nmTP, OBJPROP_FILL,      true);
            ObjectSetInteger(0, nmTP, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nmTP, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nmTP, OBJPROP_WIDTH,     1);
            ObjectSetString (0, nmTP, OBJPROP_TOOLTIP,   tip);
        }

        // Lignes TP1 et TP2 (dashed, C_TP1_L/C_TP2_L) à l'intérieur de la box TP.
        SmcNewLine(base + "_TP1", t, tp1, tEnd, tp1, C_TP1_L, 1, STYLE_DASH);
        SmcNewLine(base + "_TP2", t, tp2, tEnd, tp2, C_TP2_L, 1, STYLE_DASH);

    }
    ChartRedraw(0);
}

// === SECTION 14bis : Lignes de niveaux décoratives (PDH/PDL/PWH/PWL + Asian HL) ===
// Phase décorative. Affiche les niveaux liquide précédents comme des lignes horizontales

// === PHASE 1.2 : Affichage des rectangles de Sessions (Asie, Londres, New York) ===
// Pine v11 (lignes 2580-2620) : dessine des boxes bgcolor pour chaque session active.
// En MQL5, on boucle sur les dernières 24h de bougies pour identifier les sessions et dessiner
// des rectangles. Le texte ("Session Asie", etc.) est placé en haut à droite via OBJ_TEXT.
void f_drawSessions(const datetime &time[], const double &high[], const double &low[]) {
    ObjectsDeleteAll(0, SMC_PREFIX + "SES");
    if(!g_showAsian && !g_showLondon && !g_showNY) return;

    int rates_total = ArraySize(time);
    if(rates_total < 2) return;

    datetime tNow = time[rates_total - 1];

    // Calculer le minuit broker du jour en cours (heure broker = Paris + 1h chez Axi).
    // On travaille directement en heure broker (time[] est en heure broker), avec les
    // constantes SES_BROKER_* (= SES_PARIS_* + 60 min d'offset, décision Rono 2026-07-26).
    // TimeToStruct décompose tNow en MqlDateTime (champs hour/min/sec en heure broker).
    MqlDateTime mdtNow;
    TimeToStruct(tNow, mdtNow);
    // Reconstruire le minuit broker : on prend l'année/mois/jour de tNow, hour=min=sec=0.
    mdtNow.hour = 0; mdtNow.min = 0; mdtNow.sec = 0;
    datetime tMidnightBroker = StructToTime(mdtNow);

    // Bornes temporelles exactes des sessions (en heure broker, via SES_BROKER_*).
    // Ce sont des timestamps absolus (instants précis), comparables directement avec time[i].
    datetime t1[3], t2[3];
    t1[0] = tMidnightBroker + SES_BROKER_ASIE_START   * 60;  t2[0] = tMidnightBroker + SES_BROKER_ASIE_END   * 60;
    t1[1] = tMidnightBroker + SES_BROKER_LONDON_START * 60;  t2[1] = tMidnightBroker + SES_BROKER_LONDON_END * 60;
    t1[2] = tMidnightBroker + SES_BROKER_NY_START     * 60;  t2[2] = tMidnightBroker + SES_BROKER_NY_END     * 60;

    color  sesColors[3] = {clrDarkGreen, clrDarkBlue, clrDarkRed};
    string sesNames[3]  = {"Session Asie", "Session Européenne", "Session Américaine"};
    bool   sesActive[3] = {g_showAsian, g_showLondon, g_showNY};

    // Calculer le range haut/bas de chaque session en scannant les bougies
    double sesHigh[3] = {0, 0, 0};
    double sesLow[3]  = {9999999, 9999999, 9999999};
    bool   sesFound[3] = {false, false, false};

    for(int i = MathMax(0, rates_total - 288); i < rates_total; i++) { // ~24h en M5
        datetime t = time[i];
        for(int s = 0; s < 3; s++) {
            if(t >= t1[s] && t < t2[s]) {
                double h = high[i];
                double l = low[i];
                if(!sesFound[s]) { sesHigh[s] = h; sesLow[s] = l; sesFound[s] = true; }
                else { sesHigh[s] = MathMax(sesHigh[s], h); sesLow[s] = MathMin(sesLow[s], l); }
            }
        }
    }

    // Dessiner les rectangles. tEnd = fin bougie courante (uniformisation 2026-07-26).
    datetime tEnd = tNow + (datetime)PeriodSeconds(_Period);
    int nCreated = 0;
    for(int s = 0; s < 3; s++) {
        if(sesActive[s] && sesFound[s]) {
            datetime tRight = (tNow < t2[s]) ? tEnd : t2[s]; // si session en cours → étirer, sinon → fin fixe
            string nm = SMC_PREFIX + "SES_" + IntegerToString(s);
            if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0, t1[s], sesHigh[s], tRight, sesLow[s])) {
                ObjectSetInteger(0, nm, OBJPROP_COLOR, sesColors[s]);
                ObjectSetInteger(0, nm, OBJPROP_FILL, true);
                ObjectSetInteger(0, nm, OBJPROP_BACK, false); // BACK=false obligatoire (FILL+BACK=true = invisible, leçon §5bis)
                ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
                nCreated++;
            }
            SmcBlockLabel("SES_LB_" + IntegerToString(s), tRight, sesHigh[s], sesNames[s], clrWhite);
        }
    }
    Print("[SMC diag SES] tMidnightBroker=", TimeToString(tMidnightBroker, TIME_DATE|TIME_MINUTES),
          " | Asie [", TimeToString(t1[0], TIME_MINUTES), "-", TimeToString(t2[0], TIME_MINUTES),
          "] London [", TimeToString(t1[1], TIME_MINUTES), "-", TimeToString(t2[1], TIME_MINUTES),
          "] NY [", TimeToString(t1[2], TIME_MINUTES), "-", TimeToString(t2[2], TIME_MINUTES), "]",
          " | found[A,L,N]=", sesFound[0], sesFound[1], sesFound[2], " nCreated=", nCreated);
    ChartRedraw(0);
}

// s'étendant du début du graphique au bar courant (+marge). Couleurs/styles fidèles au Pine :
//   PDH/PDL = dashed (doré/orange), PWH/PWL = dotted (bleu/cyan).
// (Asian HL est dans f_drawAsianHL — toggle indépendant g_showAsianHL, préfixe "AHL".)
// Option B : appelée APRÈS la boucle OnCalculate (jamais dedans) comme f_drawOB/f_drawFVG.
// Nettoyage par ObjectsDeleteAll(prefix "LVL") puis tout redessiner à chaque tick.
void f_drawLevels(const datetime &time[]) {
    if(!g_showLevels) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "LVL");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante (uniformisation 2026-07-26)
    // Bord gauche = bougie exacte où le PDH/PDL (high/low du jour précédent) s'est formé.
    // On cherche la bougie UT (M15) du jour précédent dont le high = g_pdh (ou low = g_pdl).
    // iHighest/iLowest sur la plage [g_prevDayStartTime, g_curDayStartTime[ donne l'index.
    datetime tPdhBar = g_prevDayStartTime, tPdlBar = g_prevDayStartTime;
    datetime tPwhBar = g_prevWeekStartTime, tPwlBar = g_prevWeekStartTime;
    int iStart = iBarShift(_Symbol, _Period, g_curDayStartTime, false);
    int iEnd   = (g_prevDayStartTime > 0) ? iBarShift(_Symbol, _Period, g_prevDayStartTime, false) : iStart;
    if(iStart >= 0 && iEnd > iStart) {
        int n = iEnd - iStart;   // nombre de bougies du jour précédent
        int iHi = iHighest(_Symbol, _Period, MODE_HIGH, n, iStart);
        int iLo = iLowest (_Symbol, _Period, MODE_LOW,  n, iStart);
        if(iHi >= 0) tPdhBar = iTime(_Symbol, _Period, iHi);
        if(iLo >= 0) tPdlBar = iTime(_Symbol, _Period, iLo);
    }
    // PWH/PWL : bougie exacte du high/low de la semaine précédente.
    int iWStart = iBarShift(_Symbol, _Period, g_curWeekStartTime, false);
    int iWEnd   = (g_prevWeekStartTime > 0) ? iBarShift(_Symbol, _Period, g_prevWeekStartTime, false) : iWStart;
    if(iWStart >= 0 && iWEnd > iWStart) {
        int nw = iWEnd - iWStart;
        int iWH = iHighest(_Symbol, _Period, MODE_HIGH, nw, iWStart);
        int iWL = iLowest (_Symbol, _Period, MODE_LOW,  nw, iWStart);
        if(iWH >= 0) tPwhBar = iTime(_Symbol, _Period, iWH);
        if(iWL >= 0) tPwlBar = iTime(_Symbol, _Period, iWL);
    }

    // PDH (doré, dashed) — affiche uniquement si actif (non sweepé, SMC pur).
    if(g_pdhActive > 0) {
        SmcNewLine("LVL_PDH", tPdhBar, g_pdhActive, tEnd, g_pdhActive, clrGold, 1, STYLE_DASH);
        SmcBlockLabel("LVL_PDH_L", tEnd, g_pdhActive, "PDH", clrGold);
    }
    // PDL (orange, dashed)
    if(g_pdlActive > 0) {
        SmcNewLine("LVL_PDL", tPdlBar, g_pdlActive, tEnd, g_pdlActive, clrOrange, 1, STYLE_DASH);
        SmcBlockLabel("LVL_PDL_L", tEnd, g_pdlActive, "PDL", clrOrange);
    }
    // PWH (bleu, dotted)
    if(g_pwhActive > 0) {
        SmcNewLine("LVL_PWH", tPwhBar, g_pwhActive, tEnd, g_pwhActive, clrDodgerBlue, 1, STYLE_DOT);
        SmcBlockLabel("LVL_PWH_L", tEnd, g_pwhActive, "PWH", clrDodgerBlue);
    }
    // PWL (cyan, dotted)
    if(g_pwlActive > 0) {
        SmcNewLine("LVL_PWL", tPwlBar, g_pwlActive, tEnd, g_pwlActive, clrDarkTurquoise, 1, STYLE_DOT);
        SmcBlockLabel("LVL_PWL_L", tEnd, g_pwlActive, "PWL", clrDarkTurquoise);
    }
    ChartRedraw(0);
}

// === SECTION 14 (suite) : affichage Asian High/Low (MODULE 10, séparé de f_drawLevels) ===
// Toggle indépendant g_showAsianHL (vid 23). Préfixe "AHL" séparé de "LVL" pour que les
// ObjectsDeleteAll ne s'écrasent pas mutuellement (PDH/PDL vs AsianHL affichables séparément).
void f_drawAsianHL(const datetime &time[]) {
    if(!g_showAsianHL) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "AHL");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante (uniformisation 2026-07-26)
    // Bord gauche = début de la session Asie (Pine _ahStartBar L2688/2711/2726).
    // Fallback time[0] si non capturé (1er bar du graphique avant la 1ère session).
    datetime tAhStart = (g_ahStartBar > 0) ? g_ahStartBar : time[0];
    // Asian High (jaune, dashed) — Pine #FFD600 ≈ clrYellow ; seulement si valide
    if(g_ahHighDrawn_valid && g_ahHighDrawn > 0) {
        SmcNewLine("AHL_H", tAhStart, g_ahHighDrawn, tEnd, g_ahHighDrawn, clrYellow, 1, STYLE_DASH);
        SmcBlockLabel("AHL_H_L", tEnd, g_ahHighDrawn, "Asian H", clrYellow);
    }
    // Asian Low (orange foncé, dashed) — Pine #FF6F00 ≈ clrDarkOrange
    if(g_ahLowDrawn_valid && g_ahLowDrawn > 0) {
        SmcNewLine("AHL_L", tAhStart, g_ahLowDrawn, tEnd, g_ahLowDrawn, clrDarkOrange, 1, STYLE_DASH);
        SmcBlockLabel("AHL_L_L", tEnd, g_ahLowDrawn, "Asian L", clrDarkOrange);
    }
    ChartRedraw(0);
}

// === SECTION 14 (suite) : affichage liquidité EQH/EQL (MODULE 4, refonte 2026-07-27) ===
// Modèle SMC avancé : compteur de touches + grisage des niveaux sweepés.
// Couleurs EXACTES Pine (audit 2026-07-27) : C_EQH=#FFD600, C_EQL=#00BCD4, C_LIQ_SWEEP=#616161.
// Texte : noir sur EQH (jaune), blanc sur EQL (cyan) — clone Pine L646.
#define C_LIQ_EQH    C'255,214,0'     // #FFD600 jaune doré (Pine C_EQH)
#define C_LIQ_EQL    C'0,188,212'     // #00BCD4 cyan (Pine C_EQL)
#define C_LIQ_SWEEP  C'97,97,97'      // #616161 gris foncé (Pine C_LIQ_SWEEP)
void f_drawLiq(const datetime &time[]) {
    if(!g_showLiq) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "LIQ");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante
    for(int k = 0; k < g_nLiq; k++) {
        color clr   = g_liqLevels[k].isHigh ? C_LIQ_EQH : C_LIQ_EQL;   // couleur de base Pine
        int   width = 1;
        if(g_liqLevels[k].swept) {
            clr = C_LIQ_SWEEP;   // gris foncé si sweepé
        } else if(g_liqLevels[k].touches >= 3) {
            width = 2;            // épaisseur 2 si confirmé (3+ touches)
        }
        // Couleur du texte : Pine met noir sur EQH (fond jaune) et blanc sur EQL (fond cyan).
        // En MQL5 OBJ_TEXT n'a pas de fond → texte noir invisible sur fond noir. On utilise blanc
        // pour les 2 (lisible sur fond sombre MT5), sauf si sweepé (gris clair).
        color txtClr = g_liqLevels[k].swept ? clrSilver : clrWhite;
        string lbl = (g_liqLevels[k].isHigh ? "EQH" : "EQL") + " ×" + IntegerToString(g_liqLevels[k].touches);
        SmcNewLine("LIQ_" + IntegerToString(k), g_liqLevels[k].tFirst, g_liqLevels[k].price,
                   tEnd, g_liqLevels[k].price, clr, width, STYLE_DASH);
        SmcBlockLabel("LIQ_L" + IntegerToString(k), tEnd, g_liqLevels[k].price, lbl, clr, txtClr);
    }
    ChartRedraw(0);
}

// === SECTION 14 (suite) : affichage Zones d'Achat/Vente (MODULE OB zones, Pine 2939-3010) ===
// Phase décorative étape 4. Une "Zone d'Achat/Vente" = OB qualifié signalable :
//   - force ≥ FORCE_MIN (Pine f_force(_zsc) >= i_forceMin, ligne 2951/2986)
//   - asset reconnu (_assetReconnu)
//   - f_znQualBull/Bear vrai (FVG chevauchant + DoL directionnel)
//   - NON signalé (g_obBull[k].signaled = false) — Pine _oneBullSignal équivalent visuel
// On superpose un rectangle (FILL=true + BACK=false) au-dessus de l'OB + un label
// "ACHAT force/10" (bull) / "VENTE force/10" (bear). Couleurs Pine : C_ZN_BULL = #00E676,
// C_ZN_BDR_B = #00E676 (≈ clrLime) ; C_ZN_BEAR/C_ZN_BDR_R = #FF1744 (≈ clrOrangeRed).
// Le super-signal ★ (Pine f_znSuperBull/Bear) est ignoré ici (dépend de h4BullTop, hors-scope).
// Option B : appelée APRÈS la boucle OnCalculate (jamais dedans). Nettoyage par préfixe "ZN".
void f_drawZones(const datetime &time[]) {
    if(!g_showZones) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "ZN");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    // Zone A/V étendue vers la droite pour laisser de la place aux labels ACHAT/VENTE
    // (sinon superposés à l'OB). Pine = bar_index+50 ; réduit à +17 bars (compromis lisibilité
    // demandé par Rono 2026-07-29 : le +50 Pine rendait la zone trop longue).
    datetime tEnd = time[rates_total - 1] + (datetime)(PeriodSeconds(_Period) * 17);

    // Tags Disc/Prem + alignement HTF (codés en dur ON, Pine i_znDetail/i_znBiasMTF).
    // Tag position : Disc si mid < equilibrium - tol, Prem si > equilibrium + tol.
    double eqTol = (g_pdEquilibrium > 0) ? g_pdEquilibrium * (EQ_TOL / 100.0) : 0;
    // Tag HTF : ✅ si H1 ET H4 alignés, ❌ si opposés, ⚠ sinon.
    HtfState stH1, stH4;
    bool hasH1 = HtfReadState(PERIOD_H1, g_htfStatesH1, time[rates_total-1], stH1);
    bool hasH4 = HtfReadState(PERIOD_H4, g_htfStatesH4, time[rates_total-1], stH4);
    int h1Tr = hasH1 ? stH1.trend : 0;
    int h4Tr = hasH4 ? stH4.trend : 0;

    // Zones BULL (ACHAT) — Pine f_zonesLifecycle, branche bull (lignes 2942-2975).
    // LATCH (Pine L3083-3087) : une zone créée reste affichée jusqu'à suppression de l'OB.
    // Le flag zoneActive est activé dans f_accumScores quand l'OB devient un setup valide.
    int _diagZb_n = 0, _diagZb_active = 0;
    for(int k = 0; k < g_nObBull; k++) {
        _diagZb_n++;
        if(!g_obBull[k].zoneActive) continue;
        _diagZb_active++;
        int force = f_force(g_obBull[k].score);
        color clr = clrLime;
        // Tags Disc/Prem + HTF (codés en dur).
        double midB = g_obBull[k].mid;
        string posTagB = (g_pdEquilibrium > 0 && midB < g_pdEquilibrium - eqTol) ? " Disc"
                       : (g_pdEquilibrium > 0 && midB > g_pdEquilibrium + eqTol) ? " Prem" : "";
        string htfTagB = (h1Tr == 1 && h4Tr == 1) ? " \xE2\x9C\x85HTF"   // ✅
                       : (h1Tr == -1 && h4Tr == -1) ? " \xE2\x9D\x8CHTF"  // ❌
                       : " \xE2\x9A\xA0HTF";                                // ⚠
        string nm = SMC_PREFIX + "ZN_B_" + IntegerToString(k);
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        g_obBull[k].t0, g_obBull[k].top,
                        tEnd,          g_obBull[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR,     clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL,      false);
            ObjectSetInteger(0, nm, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nm, OBJPROP_WIDTH,     2);
        }
        SmcBlockLabel("ZN_LB_" + IntegerToString(k), tEnd, g_obBull[k].top,
                      "ACHAT " + IntegerToString(force) + "/10" + posTagB + htfTagB, clr);
    }
    // Zones BEAR (VENTE) — Pine f_zonesLifecycle, branche bear (lignes 2977-3010).
    // LATCH (Pine L3119-3123) : une zone créée reste affichée jusqu'à suppression de l'OB.
    int _diagZr_n = 0, _diagZr_active = 0;
    for(int k = 0; k < g_nObBear; k++) {
        _diagZr_n++;
        if(!g_obBear[k].zoneActive) continue;
        _diagZr_active++;
        int force = f_force(g_obBear[k].score);
        color clr = clrOrangeRed;
        // Tags Disc/Prem + HTF (codés en dur). Pour SELL, alignement = H1 ET H4 baissiers.
        double midR = g_obBear[k].mid;
        string posTagR = (g_pdEquilibrium > 0 && midR < g_pdEquilibrium - eqTol) ? " Disc"
                       : (g_pdEquilibrium > 0 && midR > g_pdEquilibrium + eqTol) ? " Prem" : "";
        string htfTagR = (h1Tr == -1 && h4Tr == -1) ? " \xE2\x9C\x85HTF"   // ✅ (aligné baissier)
                       : (h1Tr == 1 && h4Tr == 1) ? " \xE2\x9D\x8CHTF"     // ❌ (contre)
                       : " \xE2\x9A\xA0HTF";                                // ⚠ (mixte)
        string nm = SMC_PREFIX + "ZN_S_" + IntegerToString(k);
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        g_obBear[k].t0, g_obBear[k].top,
                        tEnd,           g_obBear[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR,     clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL,      false);
            ObjectSetInteger(0, nm, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nm, OBJPROP_WIDTH,     2);
        }
        SmcBlockLabel("ZN_LS_" + IntegerToString(k), tEnd, g_obBear[k].top,
                      "VENTE " + IntegerToString(force) + "/10" + posTagR + htfTagR, clr);
    }
    ChartRedraw(0);
}
// Pine MODULE 8 (lignes 1030-1034 / 1109-1112) : box.new(bar_index, top, bar_index+50, bot).
// Phase 1.1 : Uniformisation Vert/Rouge (comme v11/v12 Pine). Niveau 2 (estompé).
// Label Pine : "BREAKER". Les Breakers sont accumulés dans g_bbBull[]/g_bbBear[] pendant la
// boucle (captures dans f_obLifecycle, FIFO 5 par sens) puis leur lifecycle (consommation quand
// par f_breakerLifecycle. Ici on affiche l'état final = Pine à l'instant T.
// Option B : appelée APRÈS la boucle (jamais dedans). Nettoyage par préfixe "BB".
// NB : préfixe "BB" est un sous-cas de SMC_PREFIX, nettoyé globalement à chaque OnCalculate.
#define C_BB_BULL clrLimeGreen      // #00C853 vert (comme OB)
#define C_BB_BEAR clrCrimson        // #D50000 rouge (comme OB)
#define C_BB_BDR  clrWhite          // Pine C_BB_BDR = color.new(white, 60) — bordure blanche
void f_drawBreakers(const datetime &time[]) {
    if(!g_showBB) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "BB");
    int rates_total = ArraySize(time);
    if(rates_total < 2 || (g_nBbBull == 0 && g_nBbBear == 0)) { ChartRedraw(0); return; }
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante (uniformisation 2026-07-26)
    // Bullish Breakers (support) : Pine bbBullBox, bgcolor=C_BB_BULL, border white width 1
    for(int k = 0; k < g_nBbBull; k++) {
        string nm = SMC_PREFIX + "BB_B_" + IntegerToString(k);
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        g_bbBull[k].t0, g_bbBull[k].top,
                        tEnd,           g_bbBull[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR,     C_BB_BULL);
            ObjectSetInteger(0, nm, OBJPROP_FILL,      true);
            ObjectSetInteger(0, nm, OBJPROP_BACK,      false);   // fond coloré en avant-plan
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nm, OBJPROP_WIDTH,     1);       // Pine border_width=1
        }
        // OBJ_RECTANGLE ne supporte pas OBJPROP_TEXT → OBJ_TEXT superposé au bord droit.
        SmcBlockLabel("BB_LB_" + IntegerToString(k), tEnd, g_bbBull[k].top, "BREAKER", C_BB_BULL, clrWhite);
    }
    // Bearish Breakers (résistance) : Pine bbBearBox, bgcolor=C_BB_BEAR, border white width 1
    for(int k = 0; k < g_nBbBear; k++) {
        string nm = SMC_PREFIX + "BB_S_" + IntegerToString(k);
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        g_bbBear[k].t0, g_bbBear[k].top,
                        tEnd,           g_bbBear[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR,     C_BB_BEAR);
            ObjectSetInteger(0, nm, OBJPROP_FILL,      true);
            ObjectSetInteger(0, nm, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nm, OBJPROP_WIDTH,     1);       // Pine border_width=1
        }
        SmcBlockLabel("BB_LS_" + IntegerToString(k), tEnd, g_bbBear[k].top, "BREAKER", C_BB_BEAR, clrWhite);
    }
    ChartRedraw(0);
}

// === MODULE 8c : affichage Propulsion Blocks (phase décorative étape 5) ===
// Pine MODULE 8c (lignes 1344 / 1372) : box.new(bar_index, ovTop, bar_index+50, ovBot,
//   bgcolor=C_PROP_BULL/BEAR, border_color=C_PROP_BDR=gold, border_width=2, text="⚡").
// Phase 1.1 : Uniformisation Vert/Rouge (comme v11/v12 Pine). Niveau 2 (estompé).
// Les Propulsions sont accumulées dans g_propBull/g_propBear pendant la boucle par f_propCompute.
// Option B : appelée APRÈS la boucle. Nettoyage par préfixe "PROP".
#define C_PROP_BULL clrLimeGreen      // #00C853 vert (comme OB)
#define C_PROP_BEAR clrCrimson        // #D50000 rouge (comme OB)
#define C_PROP_BDR  clrGold           // Pine C_PROP_BDR = color.new(#FFD700, 10) — bordure or
void f_drawPropulsion(const datetime &time[]) {
    if(!g_showPropulsion) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "PROP");
    int rates_total = ArraySize(time);
    if(rates_total < 2 || (g_nPropBull == 0 && g_nPropBear == 0)) { ChartRedraw(0); return; }
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante (uniformisation 2026-07-26)
    // Propulsion Bull : Pine propBullBox, border gold width 2, label "⚡"
    for(int k = 0; k < g_nPropBull; k++) {
        string nm = SMC_PREFIX + "PROP_B_" + IntegerToString(k);
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        g_propBull[k].t0, g_propBull[k].top,
                        tEnd,             g_propBull[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR,     C_PROP_BULL);
            ObjectSetInteger(0, nm, OBJPROP_FILL,      true);
            ObjectSetInteger(0, nm, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nm, OBJPROP_WIDTH,     2);       // Pine border_width=2 (plus épais que Breaker)
        }
        SmcBlockLabel("PROP_LB_" + IntegerToString(k), tEnd, g_propBull[k].top,
                      ShortToString((ushort)0x26A1), C_PROP_BULL, clrWhite);   // "⚡" U+26A1
    }
    // Propulsion Bear : Pine propBearBox, border gold width 2, label "⚡"
    for(int k = 0; k < g_nPropBear; k++) {
        string nm = SMC_PREFIX + "PROP_S_" + IntegerToString(k);
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        g_propBear[k].t0, g_propBear[k].top,
                        tEnd,             g_propBear[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR,     C_PROP_BEAR);
            ObjectSetInteger(0, nm, OBJPROP_FILL,      true);
            ObjectSetInteger(0, nm, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
            ObjectSetInteger(0, nm, OBJPROP_WIDTH,     2);       // Pine border_width=2
        }
        SmcBlockLabel("PROP_LS_" + IntegerToString(k), tEnd, g_propBear[k].top,
                      ShortToString((ushort)0x26A1), C_PROP_BEAR, clrWhite);   // "⚡" U+26A1
    }
    ChartRedraw(0);
}

// === MODULE 10b : affichage NDOG / NWOG (phase décorative étape 5) ===
// Pine MODULE 10b (lignes 1405-1409 / 1430-1434) : box.new(bar_index-1, gTop, bar_index+50, gBot).
// Couleurs Pine : NDOG i_ndogColor = #26C6DA (cyan) ; NWOG i_nwogColor = #AB47BC (violet).
// Labels Pine : "New Day Opening Gap" / "New Week Opening Gap" — raccourcis "NDOG"/"NWOG".
// Les gaps sont accumulés dans g_gaps[] pendant la boucle par f_gapCompute.
// Option B : appelée APRÈS la boucle. Nettoyage par préfixe "GAP".
#define C_NDOG        clrDarkTurquoise   // #26C6DA cyan ≈ (fresh)
#define C_NDOG_MIT    clrTeal            // cyan atténué (mitigated, plus sombre)
#define C_NWOG        clrMediumPurple    // #AB47BC violet ≈ (fresh)
#define C_NWOG_MIT    clrIndigo          // violet atténué (mitigated, plus sombre)

// Phase 1.3/1.4 : couleurs IB / Equilibrium / OTE / OB HTF
// IB : système Vert/Rouge à 2 niveaux (fidèle Pine #00C853/#D50000 transp 93 fresh / 96 partial).
#define C_IB_BULL_FRESH   clrLimeGreen    // #00C853 vert (comme OB/FVG)
#define C_IB_BULL_PARTIAL clrSeaGreen     // vert mitigé (plus foncé)
#define C_IB_BEAR_FRESH   clrCrimson      // #D50000 rouge (comme OB/FVG)
#define C_IB_BEAR_PARTIAL clrFireBrick    // rouge mitigé
#define C_EQL             clrGold         // #FFD700 or (Pine equilibrium, transp 20)
#define C_OTE_BULL        clrLimeGreen    // #00C853 vert (Pine OTE transp 80)
#define C_OTE_BEAR        clrCrimson      // #D50000 rouge (Pine OTE transp 80)
// OB HTF : couleurs par (TF × sens × rang) définies juste avant f_drawHtf (rang 1-2-3 par TF).
void f_drawGaps(const datetime &time[]) {
    if(!g_showNDOG && !g_showNWOG) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "GAP");
    int rates_total = ArraySize(time);
    if(rates_total < 2 || g_nGaps == 0) { ChartRedraw(0); return; }
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante (uniformisation 2026-07-26)
    for(int k = 0; k < g_nGaps; k++) {
        bool isDay = g_gaps[k].isDay;
        // Respect du toggle : NDOG affiché seulement si g_showNDOG, NWOG seulement si g_showNWOG.
        if(isDay  && !g_showNDOG) continue;
        if(!isDay && !g_showNWOG) continue;
        // Couleur atténuée si mitigé (Pine L1464 : recoloration transparence 75→90 quand traversé).
        color clr = isDay ? (g_gaps[k].mitigated ? C_NDOG_MIT : C_NDOG)
                          : (g_gaps[k].mitigated ? C_NWOG_MIT : C_NWOG);
        string nm = SMC_PREFIX + "GAP_" + IntegerToString(k);
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0,
                        g_gaps[k].t0, g_gaps[k].top,
                        tEnd,         g_gaps[k].bot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR,     clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL,      true);
            ObjectSetInteger(0, nm, OBJPROP_BACK,      false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        SmcBlockLabel("GAP_L_" + IntegerToString(k), tEnd, g_gaps[k].top,
                      isDay ? "NDOG" : "NWOG", clr);
    }
    ChartRedraw(0);
}

// === SECTION 14 (Phase 1.3) : affichage Imbalance / Inner Bar (MODULE 13b, Pine 2430-2554) ===
// Toggle g_showIB (vid 24). Préfixe "IB". Rectangles de la bougie de détection à la fin de la
// bougie courante (bord glissant, fidèle Pine L2545-2554 : box.set_right(..., bar_index)).
// State 0=Fresh (couleur vive), State 1=Partial (couleur mitigée, recoloration à la mitigation).
void f_drawIB(const datetime &time[]) {
    if(!g_showIB) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "IB");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period); // fin bougie courante

    // IB BULL
    for(int k = 0; k < g_nIbBull; k++) {
        string nm = SMC_PREFIX + "IB_B_" + IntegerToString(k);
        color clr = (g_ibBullState[k] == 1) ? C_IB_BULL_PARTIAL : C_IB_BULL_FRESH;
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0, g_ibBullT0[k], g_ibBullTop[k], tEnd, g_ibBullBot[k])) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);   // visible (leçon §5bis #4)
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        SmcBlockLabel("IB_LB_" + IntegerToString(k), tEnd, g_ibBullTop[k], "Imbalance", clr);
    }
    // IB BEAR
    for(int k = 0; k < g_nIbBear; k++) {
        string nm = SMC_PREFIX + "IB_S_" + IntegerToString(k);
        color clr = (g_ibBearState[k] == 1) ? C_IB_BEAR_PARTIAL : C_IB_BEAR_FRESH;
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0, g_ibBearT0[k], g_ibBearTop[k], tEnd, g_ibBearBot[k])) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, clr);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        SmcBlockLabel("IB_LS_" + IntegerToString(k), tEnd, g_ibBearTop[k], "Imbalance", clr);
    }
    ChartRedraw(0);
}

// === SECTION 14 (Phase 1.4a) : affichage ligne Equilibrium (MODULE 4b, Pine 1496-1506) ===
// Toggle g_showEqLn (vid 25). Préfixe "EQL". g_pdEquilibrium est calculé dans f_pdOteCompute
// (milieu du dealing range figé au dernier BOS : (sh1+sl1)/2). Ligne horizontale or dashed.
void f_drawEqLn(const datetime &time[]) {
    if(!g_showEqLn) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "EQL");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    if(!g_pdOk || g_pdEquilibrium <= 0) return;   // garde : équilibre non calculé
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period);
    SmcNewLine("EQL_LN", time[0], g_pdEquilibrium, tEnd, g_pdEquilibrium, C_EQL, 1, STYLE_DASH);
    SmcBlockLabel("EQL_LB", tEnd, g_pdEquilibrium, "EQ", C_EQL);
    ChartRedraw(0);
}

// === SECTION 14 (Phase 1.4b) : affichage Zone OTE (MODULE 4c, Pine 1898-1929) ===
// Toggle g_showFIB (vid 26). Préfixe "OTE". 1 box bull + 1 box bear max (recréées à chaque BOS,
// supprimées si prix sort de la zone ou expiration). Variables globalisées dans f_pdOteCompute.
void f_drawOTE(const datetime &time[]) {
    if(!g_showFIB) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "OTE");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tEnd = time[rates_total - 1] + (datetime)PeriodSeconds(_Period);
    // Box OTE Bull : variables AFFICHAGE persistantes (g_oteBoxBull*), fidèles Pine _oteBullBox.
    if(g_oteBoxBullActive && g_oteBoxBullTop > g_oteBoxBullBot) {
        string nm = SMC_PREFIX + "OTE_B";
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0, g_oteBoxBullT0, g_oteBoxBullTop, tEnd, g_oteBoxBullBot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, C_OTE_BULL);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        SmcBlockLabel("OTE_LB_B", tEnd, g_oteBoxBullTop, "OTE", C_OTE_BULL);
    }
    // Box OTE Bear : variables AFFICHAGE persistantes (g_oteBoxBear*), fidèles Pine _oteBearBox.
    if(g_oteBoxBearActive && g_oteBoxBearTop > g_oteBoxBearBot) {
        string nm = SMC_PREFIX + "OTE_S";
        if(ObjectCreate(0, nm, OBJ_RECTANGLE, 0, g_oteBoxBearT0, g_oteBoxBearTop, tEnd, g_oteBoxBearBot)) {
            ObjectSetInteger(0, nm, OBJPROP_COLOR, C_OTE_BEAR);
            ObjectSetInteger(0, nm, OBJPROP_FILL, true);
            ObjectSetInteger(0, nm, OBJPROP_BACK, false);
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
        }
        SmcBlockLabel("OTE_LB_S", tEnd, g_oteBoxBearTop, "OTE", C_OTE_BEAR);
    }
    ChartRedraw(0);
}

// Helper : dessine un rectangle OB HTF + son label (Pine box.new L1838+).
// border_width croissant par TF pour différencier visuellement (H1=1, H4=2, W1=2, MN=3).
// Pine gère 3 niveaux de transparence par rang (b1=70%, b2=82%, b3=88%) pour effet de profondeur.
// MQL5 n'a pas de transparence native sur OBJ_RECTANGLE → on définit 3 couleurs par rang
// (rang 0 = couleur vive = C_H*_BULL/BEAR, rang 1-2 = couleurs atténuées définies plus bas).
// Label Pine : "🟢 H1" / "🔴 H1" + " ◀ ici" si close dans la zone (f_htfTag).
void DrawHtfRect(string nm, datetime t0, datetime tRight, double top, double bot,
                 color clr, int width, string tfLabel, int rang, bool isBull, double closeNow) {
    if(ObjectCreate(0, SMC_PREFIX + nm, OBJ_RECTANGLE, 0, t0, top, tRight, bot)) {
        ObjectSetInteger(0, SMC_PREFIX + nm, OBJPROP_COLOR, clr);
        ObjectSetInteger(0, SMC_PREFIX + nm, OBJPROP_FILL, true);
        ObjectSetInteger(0, SMC_PREFIX + nm, OBJPROP_BACK, false);   // visible
        ObjectSetInteger(0, SMC_PREFIX + nm, OBJPROP_SELECTABLE, false);
        ObjectSetInteger(0, SMC_PREFIX + nm, OBJPROP_WIDTH, width);
    }
    // Label Pine : "🟢 H1" (bull) / "🔴 H1" (bear) + " ◀ ici" si close dans [bot, top].
    // Emoji 🟢 (U+1F7E2) / 🔴 (U+1F534) peuvent poser pb de police MT5 → on garde texte simple
    // "H1" / "H4" + suffixe " ◀" si confluence. Couleur = clr du rectangle.
    string tag = (closeNow >= bot && closeNow <= top) ? " \xE2\x97\x80" : "";   // "◀" U+25C0 si prix dans la zone
    SmcBlockLabel(nm + "_L", tRight, top, tfLabel + tag, clr);
}

// === SECTION 14 (Phase 1.4c) : affichage OB Multi-Timeframe H1/H4/W1/MN (Pine 1732-1836) ===
// 4 toggles : g_showH1 (vid 27), g_showH4 (vid 28), g_showW1 (vid 29), g_showMN (vid 30).
// Préfixe "HTF" + sous-préfixe par TF. Pour chaque TF activé : lit le dernier HtfState via
// HtfReadState, parcourt les 6 zones (3 bull + 3 bear), dessine un rectangle si valid de
// t (bougie formation HTF) à tEnd (fin bougie courante). Couleurs DISTINCTES du Vert/Rouge
// pour différencier visuellement des OB courants (fidèle Pine L1519-1526).
// Couleurs OB HTF — Pine gère 3 niveaux de transparence par rang (rang 1 = 70%, 2 = 82%,
// 3 = 88%) pour effet de profondeur. MQL5 OBJ_RECTANGLE n'a pas de transparence native →
// on définit 3 couleurs par (TF × sens) : rang 0 = couleur vive (#define existant), rang 1-2
// = variantes plus sombres (simulent la transparence croissante). Couleurs définies en RGB
// pour être fidèles au rendu Pine (couleur × transparence).
#define C_H1_BULL1 clrDarkTurquoise   // rang 1 (vif) — Pine #00BCD4 70%
#define C_H1_BULL2 C'0,140,150'        // rang 2 — Pine #00BCD4 82% (assombri)
#define C_H1_BULL3 C'0,100,110'        // rang 3 — Pine #00BCD4 88% (très assombri)
#define C_H1_BEAR1 clrOrange          // rang 1 — Pine #FF6F00 70%
#define C_H1_BEAR2 C'180,80,0'         // rang 2 — Pine #FF6F00 82%
#define C_H1_BEAR3 C'130,60,0'         // rang 3 — Pine #FF6F00 88%
#define C_H4_BULL1 clrDodgerBlue      // rang 1 — Pine #1565C0 60%
#define C_H4_BULL2 C'15,80,150'        // rang 2 — Pine #1565C0 82%
#define C_H4_BULL3 C'10,55,105'        // rang 3 — Pine #1565C0 88%
#define C_H4_BEAR1 clrCrimson         // rang 1 — Pine #B71C1C 60%
#define C_H4_BEAR2 C'130,20,20'        // rang 2 — Pine #B71C1C 82%
#define C_H4_BEAR3 C'90,15,15'         // rang 3 — Pine #B71C1C 88%
#define C_W1_BULL1 clrTeal            // rang 1 — Pine W1
#define C_W1_BULL2 C'0,90,90'          // rang 2
#define C_W1_BULL3 C'0,65,65'          // rang 3
#define C_W1_BEAR1 clrPurple          // rang 1 — Pine W1
#define C_W1_BEAR2 C'70,0,90'          // rang 2
#define C_W1_BEAR3 C'50,0,65'          // rang 3
#define C_MN_BULL1 clrGoldenrod       // rang 1 — Pine MN
#define C_MN_BULL2 C'140,105,0'        // rang 2
#define C_MN_BULL3 C'100,75,0'         // rang 3
#define C_MN_BEAR1 clrMaroon          // rang 1 — Pine MN
#define C_MN_BEAR2 C'90,0,0'           // rang 2
#define C_MN_BEAR3 C'65,0,0'           // rang 3

void f_drawHtf(const datetime &time[], const double &close[]) {
    if(!g_showH1 && !g_showH4 && !g_showW1 && !g_showMN) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "HTF");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    datetime tNow = time[rates_total - 1];
    datetime tRight = tNow;   // V1 fix : Pine bord droit = `time` (début bougie courante), PAS +50
    double   closeNow = close[rates_total - 1];   // pour test confluence label "◀ ici"

    // Helper lambda-like : dessine les 6 zones d'un HtfState pour un TF donné.
    // Paramétré par (préfixe TF, tableau states, couleurs bull/bear × 3 rangs, width bordure, label TF).
    // MQL5 n'a pas de closures → on duplique la logique pour chaque TF (4 blocs ci-dessous).

    // ── H1 ──
    if(g_showH1) {
        HtfState st;
        if(HtfReadState(PERIOD_H1, g_htfStatesH1, tNow, st)) {
            color bullC[3] = {C_H1_BULL1, C_H1_BULL2, C_H1_BULL3};
            color bearC[3] = {C_H1_BEAR1, C_H1_BEAR2, C_H1_BEAR3};
            for(int j = 0; j < N_HTF_OB; j++) {
                if(st.bull[j].valid && st.bull[j].t > 0)
                    DrawHtfRect("HTF_H1_B" + IntegerToString(j), st.bull[j].t, tRight,
                                st.bull[j].top, st.bull[j].bot, bullC[j], 1, "H1", j, true, closeNow);
                if(st.bear[j].valid && st.bear[j].t > 0)
                    DrawHtfRect("HTF_H1_S" + IntegerToString(j), st.bear[j].t, tRight,
                                st.bear[j].top, st.bear[j].bot, bearC[j], 1, "H1", j, false, closeNow);
            }
        }
    }
    // ── H4 ──
    if(g_showH4) {
        HtfState st;
        if(HtfReadState(PERIOD_H4, g_htfStatesH4, tNow, st)) {
            color bullC[3] = {C_H4_BULL1, C_H4_BULL2, C_H4_BULL3};
            color bearC[3] = {C_H4_BEAR1, C_H4_BEAR2, C_H4_BEAR3};
            for(int j = 0; j < N_HTF_OB; j++) {
                if(st.bull[j].valid && st.bull[j].t > 0)
                    DrawHtfRect("HTF_H4_B" + IntegerToString(j), st.bull[j].t, tRight,
                                st.bull[j].top, st.bull[j].bot, bullC[j], 2, "H4", j, true, closeNow);
                if(st.bear[j].valid && st.bear[j].t > 0)
                    DrawHtfRect("HTF_H4_S" + IntegerToString(j), st.bear[j].t, tRight,
                                st.bear[j].top, st.bear[j].bot, bearC[j], 2, "H4", j, false, closeNow);
            }
        }
    }
    // ── W1 ──
    if(g_showW1) {
        HtfState st;
        if(HtfReadState(PERIOD_W1, g_htfStatesW1, tNow, st)) {
            color bullC[3] = {C_W1_BULL1, C_W1_BULL2, C_W1_BULL3};
            color bearC[3] = {C_W1_BEAR1, C_W1_BEAR2, C_W1_BEAR3};
            for(int j = 0; j < N_HTF_OB; j++) {
                if(st.bull[j].valid && st.bull[j].t > 0)
                    DrawHtfRect("HTF_W1_B" + IntegerToString(j), st.bull[j].t, tRight,
                                st.bull[j].top, st.bull[j].bot, bullC[j], 2, "W1", j, true, closeNow);
                if(st.bear[j].valid && st.bear[j].t > 0)
                    DrawHtfRect("HTF_W1_S" + IntegerToString(j), st.bear[j].t, tRight,
                                st.bear[j].top, st.bear[j].bot, bearC[j], 2, "W1", j, false, closeNow);
            }
        }
    }
    // ── MN ──
    if(g_showMN) {
        HtfState st;
        if(HtfReadState(PERIOD_MN1, g_htfStatesMN, tNow, st)) {
            color bullC[3] = {C_MN_BULL1, C_MN_BULL2, C_MN_BULL3};
            color bearC[3] = {C_MN_BEAR1, C_MN_BEAR2, C_MN_BEAR3};
            for(int j = 0; j < N_HTF_OB; j++) {
                if(st.bull[j].valid && st.bull[j].t > 0)
                    DrawHtfRect("HTF_MN_B" + IntegerToString(j), st.bull[j].t, tRight,
                                st.bull[j].top, st.bull[j].bot, bullC[j], 3, "MN", j, true, closeNow);
                if(st.bear[j].valid && st.bear[j].t > 0)
                    DrawHtfRect("HTF_MN_S" + IntegerToString(j), st.bear[j].t, tRight,
                                st.bear[j].top, st.bear[j].bot, bearC[j], 3, "MN", j, false, closeNow);
            }
        }
    }
    ChartRedraw(0);
}

// === SECTION 14 (suite) : affichage structure HH/HL/LH/LL + lignes MSS/CHOCH ===
// Phase décorative étape 2. Comme MSS/CHOCH/HH/HL/LH/LL sont des événements ponctuels
// (1 bar précis), on les a capturés pendant la boucle dans g_structEvents[] (FIFO 100).
// Cette fonction (Option B) est appelée APRÈS la boucle OnCalculate (jamais dedans, comme
// f_drawFVG/f_drawOB/f_drawLevels) : nettoyage par préfixe "STR_" puis tout redessiner.
//
// Pine références :
//   - Labels structure (Pine 309-325) : label.new(barPivot, ph/pl, "Higher High"/...) au
//     niveau exact du pivot confirmé. Texte court "HH"/"HL"/"LH"/"LL" ici (équivalent visuel).
//   - Lignes MSS (Pine 481-491) : line.new(bsh1, sh1, bar_index, sh1) — ligne horizontale du
//     niveau cassé, depuis le pivot jusqu'au bar courant. width=2, style_dashed.
//   - Lignes CHOCH (Pine 505-515) : idem, width=3, style_solid.
//
// Choix d'affichage (fidèle au marquage ponctuel Pine) : les lignes MSS/CHOCH s'étendent
// ~30 bars depuis l'événement (et non jusqu'au bar courant) — ce sont des marqueurs
// ponctuels, pas des niveaux persistants comme PDH/PDL. Les labels structure sont au
// niveau exact du pivot.
void f_drawStructEvents(const datetime &time[]) {
    // Nettoyer les anciens objets structure (labels + lignes).
    ObjectsDeleteAll(0, SMC_PREFIX + "STR");
    int rates_total = ArraySize(time);
    if(rates_total < 2) return;
    long barSecs = PeriodSeconds(_Period);
    if(barSecs <= 0) barSecs = 60;   // garde anti-zéro (évite divisions/extensions nulles)
    datetime tEndEvent = time[rates_total - 1] + (datetime)barSecs; // fin bougie courante (uniformisation 2026-07-26)
    // PAS de filtre 24h (fix 2026-07-27) : les pools FIFO limitent déjà la quantité
    // (12 HH + 6 BOS + 6 MSS + 6 CHOCH + 6 Sweep = 36 objets max). Clone Pine exact.

    // Pool HH/HL/LH/LL (12 derniers pivots). kind 1=HH, 2=LH, 3=HL, 4=LL.
    if(g_showStructure) {
        for(int k = 0; k < g_nHH; k++) {
            string nm = "HH" + IntegerToString(k);
            string lbl = (g_poolHH[k].kind == 1) ? "HH" : (g_poolHH[k].kind == 2) ? "LH"
                       : (g_poolHH[k].kind == 3) ? "HL" : "LL";
            color clr = (g_poolHH[k].kind == 1) ? C_HH : (g_poolHH[k].kind == 2) ? C_LH
                       : (g_poolHH[k].kind == 3) ? C_HL : C_LL;
            SmcBlockLabel("STR_" + nm, g_poolHH[k].t, g_poolHH[k].price, lbl, clr);
        }
    }
    // Pool BOS (6 derniers). kind 11=BOS_h, 12=BOS_b.
    // Ligne du pivot (t = time[bsh1]) au BOS (t2 = bougie du BOS), au niveau sh1/sl1.
    // Clone Pine line.new(bsh1, sh1, bar_index, sh1) — pas d'extension à la bougie courante.
    if(g_showBOS) {
        for(int k = 0; k < g_nBOS; k++) {
            bool isBull = (g_poolBOS[k].kind == 11);
            color clr = isBull ? C_BOS_BULL : C_BOS_BEAR;
            datetime tBosEnd = (g_poolBOS[k].t2 > 0) ? g_poolBOS[k].t2 : tEndEvent;
            SmcNewLine("STR_BOS" + IntegerToString(k), g_poolBOS[k].t, g_poolBOS[k].price,
                       tBosEnd, g_poolBOS[k].price, clr, 2, STYLE_SOLID);
            SmcBlockLabel("STR_BOSL" + IntegerToString(k), g_poolBOS[k].t, g_poolBOS[k].price,
                          "BOS " + ShortToString((ushort)(isBull ? 0x25B2 : 0x25BC)), clr);
        }
    }
    // Pool MSS (6 derniers). kind 5=MSS_h, 6=MSS_b.
    // Ligne du pivot (t) au MSS (t2 = bougie du MSS), clone Pine line.new(bsh1, sh1, bar_index, sh1).
    if(g_showMSS) {
        for(int k = 0; k < g_nMSS; k++) {
            bool isBull = (g_poolMSS[k].kind == 5);
            color clr = isBull ? C_MSS_BULL : C_MSS_BEAR;
            datetime tMssEnd = (g_poolMSS[k].t2 > 0) ? g_poolMSS[k].t2 : tEndEvent;
            SmcNewLine("STR_MSS" + IntegerToString(k), g_poolMSS[k].t, g_poolMSS[k].price,
                       tMssEnd, g_poolMSS[k].price, clr, 2, STYLE_DASH);
            SmcBlockLabel("STR_MSSL" + IntegerToString(k), g_poolMSS[k].t, g_poolMSS[k].price,
                          "MSS " + ShortToString((ushort)(isBull ? 0x25B2 : 0x25BC)), clr);
        }
    }
    // Pool CHOCH (6 derniers). kind 7=CHOCH_h, 8=CHOCH_b.
    // Ligne du pivot (t) au CHOCH (t2 = bougie du CHOCH), clone Pine line.new(bsh1, sh1, bar_index, sh1).
    if(g_showCHOCH) {
        for(int k = 0; k < g_nCHOCH; k++) {
            bool isBull = (g_poolCHOCH[k].kind == 7);
            color clr = isBull ? C_CHOCH_BULL : C_CHOCH_BEAR;
            datetime tChochEnd = (g_poolCHOCH[k].t2 > 0) ? g_poolCHOCH[k].t2 : tEndEvent;
            SmcNewLine("STR_CHOCH" + IntegerToString(k), g_poolCHOCH[k].t, g_poolCHOCH[k].price,
                       tChochEnd, g_poolCHOCH[k].price, clr, 3, STYLE_SOLID);
            SmcBlockLabel("STR_CHOCHL" + IntegerToString(k), g_poolCHOCH[k].t, g_poolCHOCH[k].price,
                          "CHOCH " + ShortToString((ushort)(isBull ? 0x25B2 : 0x25BC)), clr);
        }
    }
    // Pool Sweep (6 derniers). kind 9=Swp bull (▲), 10=Swp bear (▼).
    if(g_showSweep) {
        for(int k = 0; k < g_nSWP; k++) {
            bool isBull = (g_poolSWP[k].kind == 9);
            color clr = isBull ? C_SWEEP_BULL : C_SWEEP_BEAR;
            SmcBlockLabel("STR_SWP" + IntegerToString(k), g_poolSWP[k].t, g_poolSWP[k].price,
                          "Sweep " + ShortToString((ushort)(isBull ? 0x25B2 : 0x25BC)), clr);
        }
    }
    ChartRedraw(0);
}

// === SECTION 14ter : Fonds bgcolor (coloration de fond des N dernières bougies) ===
// Phase décorative étape 3. Reproduit les 4 bgcolor() Pine :
//   1. Tendance (MODULE 1, ligne 330)   : i_showBg + tendanceHaussiere/Baissiere → vert/rouge (95%)
//   2. Volume fort (MODULE 9, ligne 1167): i_showVol + volOk → bleu (#2962FF 82%)
//   3. ATR impulsion (MODULE 10, 1350-51): i_showAtr + atrBull/atrBear → vert/rouge (75%)
//   4. Premium/Discount (MODULE PD, 1510-11): i_showPD + inPremium/inDiscount → rouge/vert (95%)
//
// Pine bgcolor() colore TOUTES les bougies de l'historique (16000+). En MQL5 on ne peut pas
// créer 16000+ OBJ_RECTANGLE (saturation MT5 → lag). On limite donc aux BGCOLOR_BARS (500)
// dernières bougies visibles : l'utilisateur ne voit pas le passé lointain.
//
// Architecture 2 temps (comme f_drawStructEvents) :
//   - f_storeBg(i, ...) pendant la boucle : stocke l'état instantané du bar i dans g_bg*
//   - f_drawBg(time, high, low) APRÈS la boucle : dessine 1 rectangle par bar active.
// Anti-repaint : i vient toujours de la boucle OnCalculate (<= rates_total-2).
//
// f_storeBg : stocke l'état du bar i (uniquement les BGCOLOR_BARS derniers bars).
// Lit les flags instantanés figés au bar i par les modules précédents :
//   g_tendanceHaussiere/Baissiere (f_structureCompute), g_inPremium/Discount (f_pdOteCompute).
// Calcule volOk (Pine MODULE 9 : volMa = ta.sma(volume, 20), volOk = volume > volMa * 1.0 ;
//   i_volMult = 1.0 en dur Pine ligne 1160 ; MQL5 utilise tick_volume ≈ volume car FX n'a pas
//   de vrai volume, tick_volume est l'approximation standard) et atrBull/Bear (Pine MODULE 10 :
//   atrOk = range1 > i_atrSeuil * atr14, g_autoAtrSeuil = i_atrSeuil calibré par asset).
#define VOL_PERIOD   20    // Pine i_volPer (ligne 1160), période SMA volume
#define VOL_MULT     1.0   // Pine i_volMult (ligne 1160) — à 1.0, pas 1.5 (vérifié dans le Pine)
void f_storeBg(int i, int rates_total, const long &tick_volume[],
               const double &high[], const double &low[],
               const double &close[], const double &open[], double atr14) {
    // On ne stocke que les BGCOLOR_BARS dernières bougies (les plus récentes).
    int start = rates_total - BGCOLOR_BARS;
    if(start < 0) start = 0;
    if(i < start) return;
    int idx = i - start;
    if(idx < 0 || idx >= BGCOLOR_BARS) return;

    double range1 = high[i] - low[i];

    // volOk Pine (MODULE 9, ligne 1255) : volOk = i_moteurVolume and not na(volMa)
    //   and volume > volMa * i_volMult, où volMa = ta.sma(volume, 20).
    // IMPORTANT : ta.sma(source, length) INCLUT le bar courant dans sa fenêtre
    //   (fenêtre [i-19 .. i], PAS [i-20 .. i-1]). Le Pine compare volume (= tick_volume[i])
    //   à volMa qui contient déjà tick_volume[i]. i_moteurVolume = true en dur (Pine l.91).
    // Pine retourne na si < 20 bars disponibles → volOk = false (not na(volMa)).
    long volSum = 0; int volCnt = 0;
    for(int v = MathMax(0, i - VOL_PERIOD + 1); v <= i; v++) {
        volSum += tick_volume[v]; volCnt++;
    }
    double volAvg = (volCnt == VOL_PERIOD) ? (double)volSum / (double)volCnt : 0.0;
    bool volOk = (volAvg > 0.0) && ((double)tick_volume[i] > volAvg * VOL_MULT);

    // atrBull/Bear Pine (MODULE 10, lignes 1346-1348) : range1 > i_atrSeuil * atr14.
    // g_autoAtrSeuil = i_atrSeuil calibré par asset (g_autoAtrSeuil, ligne 147). Ce seuil sert
    // à l'AFFICHAGE ATR (différent de g_autoAtrScore réservé au scoring, voir f_score).
    bool atrOk = (atr14 > 0.0) && (range1 > g_autoAtrSeuil * atr14);
    bool atrBull = atrOk && (close[i] > open[i]);
    bool atrBear = atrOk && (close[i] < open[i]);

    g_bgTrendBull[idx] = g_tendanceHaussiere;
    g_bgTrendBear[idx] = g_tendanceBaissiere;
    g_bgVol[idx]       = volOk;
    g_bgAtrBull[idx]   = atrBull;
    g_bgAtrBear[idx]   = atrBear;
    g_bgPrem[idx]      = g_inPremium;
    g_bgDisc[idx]      = g_inDiscount;

    if(idx + 1 > g_bgCount) g_bgCount = idx + 1;
}

// DrawBgRect : helper rectangle de FOND (derrière les bougies).
// FILL=true + BACK=true : rectangle coloré en arrière-plan (derrière les bougies).
// NB : AGENTS.md §5bis #4 signale que FILL=true+BACK=true rend invisible chez Rono, MAIS ce bug
// a été observé pour les OB/FVG qui utilisent BACK=false (avant-plan). Pour les fonds DERRIÈRE
// les bougies, BACK=true est attendu. Choix validé par la tâche (à confirmer visuellement par Rono).
void DrawBgRect(string name, datetime t1, datetime t2, double top, double bot, color clr) {
    if(ObjectCreate(0, SMC_PREFIX + name, OBJ_RECTANGLE, 0, t1, top, t2, bot)) {
        ObjectSetInteger(0, SMC_PREFIX + name, OBJPROP_COLOR, clr);
        ObjectSetInteger(0, SMC_PREFIX + name, OBJPROP_FILL, true);
        ObjectSetInteger(0, SMC_PREFIX + name, OBJPROP_BACK, true);   // derrière les bougies
        ObjectSetInteger(0, SMC_PREFIX + name, OBJPROP_SELECTABLE, false);
        ObjectSetInteger(0, SMC_PREFIX + name, OBJPROP_HIDDEN, true); // pas dans la liste d'objets
    }
}

// Couleurs bgcolor fidèles au Pine :
#define C_BG_TREND_BULL clrDarkGreen    // Pine C_BG_BULL = color.new(color.green, 95) ≈ vert très transparent
#define C_BG_TREND_BEAR clrDarkRed      // Pine C_BG_BEAR  = color.new(color.red,   95) ≈ rouge très transparent
#define C_BG_VOL        clrBlue         // Pine i_volColor = color.new(#2962FF, 82) ≈ bleu transparent
#define C_BG_ATR_BULL   clrGreen        // Pine i_atrBull  = color.new(#00E676, 75) ≈ vert
#define C_BG_ATR_BEAR   clrRed          // Pine i_atrBear  = color.new(#FF1744, 75) ≈ rouge
#define C_BG_PREM       clrIndianRed    // Pine #F44336 95% (Premium = rouge)
#define C_BG_DISC       clrSeaGreen     // Pine #4CAF50 95% (Discount = vert)

// f_drawBg : dessine un rectangle par bar active (parmi les BGCOLOR_BARS dernières) pour chaque
// type de bgcolor ACTIVÉ. Appelée APRÈS la boucle OnCalculate (Option B, jamais dedans).
// Nettoyage par ObjectsDeleteAll(prefix "BG") puis tout redessiner à chaque recalcul.
// Note : OBJ_RECTANGLE ne peut pas remplir "toute la hauteur du graphique" comme bgcolor() Pine.
// On étend donc verticalement autour du haut/bas de la bougie (marge 50% du range + 10 pts)
// pour que le fond soit bien visible. Limitation acceptée (sinon il faudrait OBJ_RECTANGLE full
// chart à chaque redimensionnement → complexe et peu utile vu la transparence).
void f_drawBg(const datetime &time[], const double &high[], const double &low[]) {
    // Sortie anticipée si aucun type n'est activé (économie de ObjectsDeleteAll à chaque tick).
    if(!g_showBgTrend && !g_showBgVol && !g_showBgAtr && !g_showBgPD) return;
    ObjectsDeleteAll(0, SMC_PREFIX + "BG");
    int rates_total = ArraySize(time);
    if(rates_total < 2 || g_bgCount == 0) { ChartRedraw(0); return; }

    int start = rates_total - BGCOLOR_BARS;
    if(start < 0) start = 0;
    long barSecs = (long)PeriodSeconds(_Period);
    // PAS de filtre 24h (fix 2026-07-27) : la limite BGCOLOR_BARS=500 suffit à éviter la saturation.
    // Le Pine colore tout l'historique via bgcolor() (natif), mais MT5 ne peut pas créer 16000+
    // rectangles → on se limite aux 500 dernières bougies (compromis lisibilité/perf).

    // Pleine hauteur du graphique (fidèle bgcolor() Pine qui colore toute la hauteur).
    // Lecture des prix min/max visibles du chart. Au démarrage ou si auto-scale désactivé,
    // ces propriétés peuvent être 0 → fallback sur un grand range autour des bougies visibles.
    double chartTop = 0.0, chartBot = 0.0;
    bool okT = ChartGetDouble(0, CHART_PRICE_MAX, 0, chartTop);
    bool okB = ChartGetDouble(0, CHART_PRICE_MIN, 0, chartBot);
    double top, bot;
    if(okT && okB && chartTop > chartBot && chartTop > 0.0) {
        // Marge 10% pour déborder légèrement hors zone visible (sécurité visuelle).
        double range = chartTop - chartBot;
        top = chartTop + range * 0.10;
        bot = chartBot - range * 0.10;
    } else {
        // Fallback (graphique pas encore prêt) : grande marge autour des highs/lows visibles.
        double mx = -DBL_MAX, mn = DBL_MAX;
        for(int idx = 0; idx < g_bgCount; idx++) {
            int i = start + idx;
            if(i < 0 || i >= rates_total) continue;
            if(high[i] > mx) mx = high[i];
            if(low[i]  < mn) mn = low[i];
        }
        if(mx > mn) {
            double range = mx - mn;
            top = mx + range * 0.50;   // marge 50% au-dessus
            bot = mn - range * 0.50;   // marge 50% en-dessous
        } else {
            top = 1e10; bot = -1e10;    // ultime fallback (ne devrait jamais arriver)
        }
    }

    for(int idx = 0; idx < g_bgCount; idx++) {
        int i = start + idx;
        if(i < 0 || i >= rates_total) continue;
        datetime t1 = time[i];
        datetime t2 = t1 + (datetime)barSecs;   // largeur d'1 bougie
        string base = "BG_" + IntegerToString(i);

        // Fond tendance (bull vert / bear rouge). Note : tendanceHaussiere et Baissiere sont
        // mutuellement exclusives au Pine (g_bullCount>=2 vs g_bearCount>=2, jamais les 2).
        if(g_showBgTrend) {
            if(g_bgTrendBull[idx])
                DrawBgRect(base + "_TB", t1, t2, top, bot, C_BG_TREND_BULL);
            else if(g_bgTrendBear[idx])
                DrawBgRect(base + "_TS", t1, t2, top, bot, C_BG_TREND_BEAR);
        }
        // Fond volume fort (bleu).
        if(g_showBgVol && g_bgVol[idx])
            DrawBgRect(base + "_V", t1, t2, top, bot, C_BG_VOL);
        // Fond impulsion ATR (bull vert / bear rouge). Mutuellement exclusifs (close>open vs <open).
        if(g_showBgAtr) {
            if(g_bgAtrBull[idx])
                DrawBgRect(base + "_AB", t1, t2, top, bot, C_BG_ATR_BULL);
            else if(g_bgAtrBear[idx])
                DrawBgRect(base + "_AS", t1, t2, top, bot, C_BG_ATR_BEAR);
        }
        // Fond Premium/Discount (prem rouge / disc vert). Mutuellement exclusifs (equilibrium ± tol).
        if(g_showBgPD) {
            if(g_bgPrem[idx])
                DrawBgRect(base + "_P", t1, t2, top, bot, C_BG_PREM);
            else if(g_bgDisc[idx])
                DrawBgRect(base + "_D", t1, t2, top, bot, C_BG_DISC);
        }
    }
    ChartRedraw(0);
}

// UpdateBgRectPrices : met à jour les prix des 2 points d'ancrage de TOUS les rectangles
// "BG_*" existants avec les nouvelles bornes pleine-hauteur du graphique (après zoom/pan/
// redimensionnement). Ne recalcule PAS les flags (g_bgVol etc. inchangés), ne recrée pas
// les rectangles. Évite un ObjectsDeleteAll + redraw complet (lourd) sur chaque
// CHARTEVENT_CHART_CHANGE.
// Note : OBJ_RECTANGLE a 2 points d'ancrage (point 0 = haut-gauche, point 1 = bas-droite).
// MQL5 n'a pas d'OBJPROP_PRICE2 → on utilise ObjectMove(chart, name, point_index, time, price).
// Le temps des points d'ancrage est conservé (on ne change que le prix).
void UpdateBgRectPrices() {
    double chartTop = 0.0, chartBot = 0.0;
    if(!ChartGetDouble(0, CHART_PRICE_MAX, 0, chartTop)) return;
    if(!ChartGetDouble(0, CHART_PRICE_MIN, 0, chartBot)) return;
    if(chartTop <= chartBot || chartTop <= 0.0) return;
    double range = chartTop - chartBot;
    double top = chartTop + range * 0.10;
    double bot = chartBot - range * 0.10;
    int total = ObjectsTotal(0, -1, OBJ_RECTANGLE);
    string pfx = SMC_PREFIX + "BG_";
    for(int k = total - 1; k >= 0; k--) {
        string nm = ObjectName(0, k, -1, OBJ_RECTANGLE);
        if(StringFind(nm, pfx) != 0) continue;   // ne concerne que nos BG_*
        // Conserve le temps des 2 points d'ancrage, change uniquement le prix.
        datetime t0 = (datetime)ObjectGetInteger(0, nm, OBJPROP_TIME, 0);
        datetime t1 = (datetime)ObjectGetInteger(0, nm, OBJPROP_TIME, 1);
        ObjectMove(0, nm, 0, t0, top);
        ObjectMove(0, nm, 1, t1, bot);
    }
    ChartRedraw(0);
}

// === SECTION 15 (partielle) : OnInit / OnCalculate / OnDeinit ===

// Reset complet de l'état SMC avant un rejeu bar-par-bar de l'historique.
// Indispensable : sans reset, les pivots/flags/compteurs d'un calcul précédent
// pollueraient le rejeu (le Pine repart de zéro à chaque recalcul complet).
// On remet TOUTES les globales d'état à leur valeur de départ + on purge le
// graphique (objets FIFO du préfixe SMC_) pour repartir d'un visuel vierge.
void ResetSmcState() {
    // Pivots (SECTION 1)
    g_sh1 = 0; g_sh2 = 0; g_sl1 = 0; g_sl2 = 0;
    g_bsh1 = 0; g_bsh2 = 0; g_bsl1 = 0; g_bsl2 = 0;
    g_sh1_valid = false; g_sl1_valid = false;
    g_hasPH_thisBar = false; g_hasPL_thisBar = false;

    // Compteurs / tendance (SECTION 1)
    g_bullCount = 0; g_bearCount = 0;
    g_tendanceHaussiere = false; g_tendanceBaissiere = false;

    // Flags signaux structure (SECTION 1)
    g_dernierSH1_sig = 0; g_dernierSL1_sig = 0;
    g_mssHPending = false; g_mssBPending = false;

    // BOS (SECTION 2)
    g_bosHaussier = false; g_bosBaissier = false;

    // MSS / CHOCH (SECTION 3)
    g_mssHaussier = false; g_mssBaissier = false;
    g_chochHaussier = false; g_chochBaissier = false;
    g_dernierMSS_level = 0; g_dernierCHOCH_level = 0;
    g_dernierMSS_bar = 0; g_dernierCHOCH_bar = 0;

    // Premium/Discount + OTE (SECTION 3bis, J4d). Les barres OTE à -1 invalident les
    // plages (garde anti-na équivalente au `na` du Pine).
    g_pdOk = false; g_pdEquilibrium = 0;
    g_inPremium = false; g_inDiscount = false;
    g_fibBullBar = -1; g_fibBearBar = -1;
    g_inOTE_bull = false; g_inOTE_bear = false;
    // Phase 1.4b : vars OTE globalisées (reset pour f_drawOTE).
    g_oteBullValid = false; g_oteBearValid = false;
    g_oteTopBull = 0; g_oteBotBull = 0; g_oteBotBear = 0; g_oteTopBear = 0;
    // Phase 2 fix OTE : vars AFFICHAGE persistantes (découplées du scoring qui expire à 12 bars).
    g_oteBoxBullActive = false; g_oteBoxBearActive = false;
    g_oteBoxBullTop = 0; g_oteBoxBullBot = 0; g_oteBoxBearTop = 0; g_oteBoxBearBot = 0;
    g_oteBoxBullT0 = 0; g_oteBoxBearT0 = 0;
    // Phase 1.3 : Imbalance (IB) — reset compteurs + anti-double-comptage.
    g_nIbBull = 0; g_nIbBear = 0;
    g_ibBullLastBar = -1; g_ibBearLastBar = -1;

    // Liquidité EQH/EQL (SECTION 4)
    g_dernierEQH_level = 0; g_dernierEQL_level = 0;

    // prevLiq PDH/PDL/PWH/PWL (SECTION 4bis, J4b) — niveaux + flags dérivés.
    g_pdh = 0; g_pdl = 0; g_pwh = 0; g_pwl = 0;
    g_pdhActive = 0; g_pdlActive = 0; g_pwhActive = 0; g_pwlActive = 0;
    g_curDayStartTime = 0; g_curWeekStartTime = 0;   // bornes gauches PDH/PDL/PWH/PWL
    g_prevDayStartTime = 0; g_prevWeekStartTime = 0;
    g_nearBullPrevLiq = false; g_nearBearPrevLiq = false;
    g_sweepBullPrevLiq = false; g_sweepBearPrevLiq = false;

    // Asian HL (SECTION 4ter, J4b) — accumulation session + niveaux figés.
    g_ahHigh = 0; g_ahLow = 0;
    g_ahHigh_valid = false; g_ahLow_valid = false;
    g_ahHighDrawn = 0; g_ahLowDrawn = 0;
    g_ahHighDrawn_valid = false; g_ahLowDrawn_valid = false;
    g_ahStartBar = 0;   // bord gauche Asian HL
    g_inAsianSession = false;

    // Sweep (SECTION 5) — flags remis à zéro + état machine à états (pending + archivé).
    // Machine à états Pine (MODULE 5) : pending sweepH/sweepB + sweeps confirmés archivés.
    // NB : g_dernierEQL/EQH_level sont déjà reset au-dessus (SECTION 4 liquidité).
    g_sweepBullFrais = false; g_sweepBearFrais = false;
    g_sweepH_bar = -1; g_sweepH_level = 0;
    g_sweepB_bar = -1; g_sweepB_level = 0;
    g_dernierSweepH_level = 0; g_dernierSweepH_bar = -1;
    g_dernierSweepB_level = 0; g_dernierSweepB_bar = -1;

    // HTF confluence (SECTION 3quater, J4e) — flags dérivés remis à false (les arrays
    // g_htfStates_* sont recalculés par PrecalcHtf dans OnCalculate ; ici on ne reset que
    // les flags consommés par f_score pour repartir d'un état propre avant le rejeu).
    g_confluenceH1 = g_confluenceH4 = g_confluenceW1 = g_confluenceMN = false;
    g_h1Bull1Valid = g_h1Bear1Valid = false;
    g_h4Bull1Valid = g_h4Bear1Valid = false;
    g_w1Bull1Valid = g_w1Bear1Valid = false;
    g_mnBull1Valid = g_mnBear1Valid = false;

    // FVG (SECTION 6) — seul le compteur compte (les arrays seront écrasés en écriture)
    g_nFvgBull = 0; g_nFvgBear = 0;

    // Order Blocks (SECTION 7) — seul le compteur compte (arrays écrasés en écriture)
    g_nObBull = 0; g_nObBear = 0;

    // Modules décoratifs étape 5 (Breaker / Propulsion / NDOG-NWOG) — compteurs remis à 0
    // avant le rejeu. Les arrays sont écrasés en écriture pendant la boucle (pas besoin de
    // les zero-initialiser, on ne lit jamais un idx >= compteur). DÉCORATIF : hors scoring.
    g_nBbBull = 0; g_nBbBear = 0;
    g_nPropBull = 0; g_nPropBear = 0;
    g_nGaps = 0;

    // Filtres zone (SECTION 12) — g_tradeOuvert reseté à chaque rejeu (Phase 1 indicateur ;
    //   pas de trade réel en attente). g_znDaxHTF est recalculé dans OnCalculate après ce reset.
    g_tradeOuvert = false;
    g_znDaxHTF    = false;

    // Signaux (SECTION 13) — seul le compteur compte (g_signals[] écrasé en écriture).
    g_nSignals = 0;

    // Événements structure décoratifs (SECTION 14bis) — compteur remis à 0 avant le rejeu.
    // 5 pools dédiés events structure (HH/HL, BOS, MSS, CHOCH, Sweep) — FIFO par pool.
    g_nHH = 0; g_nBOS = 0; g_nMSS = 0; g_nCHOCH = 0; g_nSWP = 0;
    g_nLiq = 0;   // pool EQH/EQL (clone Pine liqPool, refonte 2026-07-27)

    // Fonds bgcolor (SECTION 14ter) — compteur remis à 0 avant le rejeu. Les arrays g_bg*
    // seront réécrits par f_storeBg pendant la boucle (pas besoin de les zero-initialiser,
    // on ne lit jamais un idx >= g_bgCount).
    g_bgCount = 0;

    // Affichage (SECTION 14) — compteur suffixe + objets graphiques
    g_objCounter = 0;
    ObjectsDeleteAll(0, SMC_PREFIX);
}

// === SECTION 16 : Panneau de contrôle (boutons ON/OFF à droite du graphique) ===
// Panneau vertical organisé en CHAPITRES, dans le même ordre que les inputs du script
// Pine v11 (groupes GRP_*). Chaque chapitre a un en-tête non cliquable (fond bleu),
// suivi de ses boutons toggle ON/OFF. Chaque bouton toggle bascule la variable globale
// g_show* correspondante via OnChartEvent (jamais les input bool, qui sont lecture seule).
// ATTENTION (AGENTS.md §5) : ObjectsDeleteAll(0, PREFIX) est sécurisé (surcharge préfixe).
#define PANEL_PREFIX   "PNL_"   // ne PAS commencer par SMC_ (ObjectsDeleteAll(0, SMC_PREFIX) dans ResetSmcState effacerait le panneau à chaque OnCalculate)
#define PANEL_BTN_W    300      // largeur bouton (px) — suffit pour le texte à police 8
#define PANEL_BTN_H    24       // hauteur bouton (px) — lisible à police 8
#define PANEL_GAP      4        // écart vertical entre lignes (px)
#define PANEL_X_OFFSET 340      // bord gauche depuis le bord droit (px). Géométrie CORNER_RIGHT_UPPER :
                                // le bouton s'étend vers l'axe des prix sur sa largeur → bord droit du
                                // bouton = XDISTANCE - largeur = 340-210 = 130px du bord, dégage l'axe.

#define PANEL_TYPE_H   0        // header (en-tête de chapitre, non cliquable)
#define PANEL_TYPE_T   1        // toggle (bouton ON/OFF cliquable)
#define PANEL_ROWS     40       // 6 en-têtes + 34 toggles (+BPR/BSZones/London H/L/London BS — miroir v12)

// Libellés affichés, dans l'ordre exact des groupes d'inputs Pine v11 :
//   1. Sessions & Niveaux Clés (GRP_SES/PL/GAP)
//   2. Structure & Price Action (GRP_S/B/C/L/W)
//   3. Technique avancé (GRP_IB/V/A/REG)
//   4. Order Blocks & FVG (GRP_F/O/BB/PRP)
//   5. Multi-Timeframe & OTE (GRP_PD/H/FIB)
//   6. Signaux & Zones (GRP_ZN/SC/TP)
string PANEL_LABELS[PANEL_ROWS] = {
    "🕐 Sessions & Niveaux",     // 0  header
    "Session Asie",              // 1  toggle vid 20 g_showAsian
    "Session Européenne",        // 2  toggle vid 21 g_showLondon
    "Session Américaine",        // 3  toggle vid 22 g_showNY
    "NDOG (gap jour)",           // 4  toggle vid 18 g_showNDOG
    "NWOG (gap semaine)",        // 5  toggle vid 19 g_showNWOG
    "PDH/PDL/PWH/PWL",           // 6  toggle vid 10 g_showLevels
    "Asian High/Low",            // 7  toggle vid 23 g_showAsianHL
    "📊 Structure & Price Action",// 8  header
    "Labels HH/HL/LH/LL",        // 9  toggle vid 0  g_showStructure
    "Fond tendance",             // 10 toggle vid 12 g_showBgTrend
    "BOS",                       // 11 toggle vid 1  g_showBOS
    "MSS",                       // 12 toggle vid 2  g_showMSS
    "CHOCH",                     // 13 toggle vid 3  g_showCHOCH
    "EQH / EQL",                 // 14 toggle vid 4  g_showLiq
    "Sweeps",                    // 15 toggle vid 5  g_showSweep
    "⚙️ Technique avancé",      // 16 header
    "Fond Volume fort",          // 17 toggle vid 13 g_showBgVol
    "Fond Impulsion ATR",        // 18 toggle vid 14 g_showBgAtr
    "Imbalance",                 // 19 toggle vid 24 g_showIB (Phase 1.3)
    "🏦 Order Blocks & FVG",    // 20 header
    "FVG",                       // 21 toggle vid 6  g_showFVG
    "Order Blocks",              // 22 toggle vid 7  g_showOB
    "Breaker Blocks",            // 23 toggle vid 16 g_showBB
    "Propulsion Blocks",         // 24 toggle vid 17 g_showPropulsion
    "📈 Multi-Timeframe & OTE", // 25 header
    "Fond Premium/Discount",     // 26 toggle vid 15 g_showBgPD
    "Equilibrium",               // 27 toggle vid 25 g_showEqLn (Phase 1.4a)
    "Zone OTE",                  // 28 toggle vid 26 g_showFIB (Phase 1.4b)
    "OB H1",                     // 29 toggle vid 27 g_showH1 (Phase 1.4c)
    "OB H4",                     // 30 toggle vid 28 g_showH4 (Phase 1.4c)
    "OB W1",                     // 31 toggle vid 29 g_showW1 (Phase 1.4c)
    "OB MN",                     // 32 toggle vid 30 g_showMN (Phase 1.4c)
    "🎯 Signaux & Zones",       // 33 header
    "Zones Achat/Vente",         // 34 toggle vid 11 g_showZones
    "Signaux historiques",       // 35 toggle vid 9  g_showSignals
    "BPR (Balanced Price Range)",// 36 toggle vid 31 g_showBPR (MODULE 6b v12)
    "BS Zones (moteur B)",       // 37 toggle vid 32 g_showBSZones (BSZones v12)
    "London High/Low",           // 38 toggle vid 33 g_showLondonHL (MODULE 14b v12)
    "BS Zones (zones BS)"        // 39 toggle vid 34 g_showBSZones (redondant — gardé pour clarté)
};

// Type de chaque ligne (PANEL_TYPE_H = header, PANEL_TYPE_T = toggle).
int PANEL_TYPES[PANEL_ROWS] = {
    PANEL_TYPE_H,
    PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T,
    PANEL_TYPE_H,
    PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T,
    PANEL_TYPE_H,
    PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T,
    PANEL_TYPE_H,
    PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T,
    PANEL_TYPE_H,
    PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T,
    PANEL_TYPE_H,
    PANEL_TYPE_T, PANEL_TYPE_T,
    PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T, PANEL_TYPE_T
};

// vid = index de variable g_show* (0-30) pour les toggles ; -1 pour les headers.
// Le mapping vid→g_show* se fait dans GetShowVar/ToggleShowVar (switch 0-30).
int PANEL_VIDS[PANEL_ROWS] = {
    -1,
    20, 21, 22, 18, 19, 10, 23,
    -1,
    0, 12, 1, 2, 3, 4, 5,
    -1,
    13, 14, 24,
    -1,
    6, 7, 16, 17,
    -1,
    15, 25, 26, 27, 28, 29, 30,
    -1,
    11, 9,
    31, 32, 33, 33
};

// Retourne la valeur de la variable g_show* identifiée par vid (0-19).
bool GetShowVar(int vid) {
    switch(vid) {
        case 0:  return g_showStructure;
        case 1:  return g_showBOS;
        case 2:  return g_showMSS;
        case 3:  return g_showCHOCH;
        case 4:  return g_showLiq;
        case 5:  return g_showSweep;
        case 6:  return g_showFVG;
        case 7:  return g_showOB;
        case 9:  return g_showSignals;
        case 10: return g_showLevels;
        case 11: return g_showZones;
        case 12: return g_showBgTrend;
        case 13: return g_showBgVol;
        case 14: return g_showBgAtr;
        case 15: return g_showBgPD;
        case 16: return g_showBB;
        case 17: return g_showPropulsion;
        case 18: return g_showNDOG;
        case 19: return g_showNWOG;
        case 20: return g_showAsian;
        case 21: return g_showLondon;
        case 22: return g_showNY;
        case 23: return g_showAsianHL;
        case 24: return g_showIB;
        case 25: return g_showEqLn;
        case 26: return g_showFIB;
        case 27: return g_showH1;
        case 28: return g_showH4;
        case 29: return g_showW1;
        case 30: return g_showMN;
        case 31: return g_showBPR;
        case 32: return g_showBSZones;
        case 33: return g_showLondonHL;
    }
    return false;
}

// Bascule la variable g_show* identifiée par vid (0-22) + sauvegarde sur disque.
void ToggleShowVar(int vid) {
    string pfx = "SMC_" + _Symbol + "_";
    switch(vid) {
        case 0:  g_showStructure  = !g_showStructure;  GlobalVariableSet(pfx+"showStructure",  g_showStructure?1:0); break;
        case 1:  g_showBOS        = !g_showBOS;        GlobalVariableSet(pfx+"showBOS",        g_showBOS?1:0);      break;
        case 2:  g_showMSS        = !g_showMSS;        GlobalVariableSet(pfx+"showMSS",        g_showMSS?1:0);      break;
        case 3:  g_showCHOCH      = !g_showCHOCH;      GlobalVariableSet(pfx+"showCHOCH",      g_showCHOCH?1:0);    break;
        case 4:  g_showLiq        = !g_showLiq;        GlobalVariableSet(pfx+"showLiq",        g_showLiq?1:0);      break;
        case 5:  g_showSweep      = !g_showSweep;      GlobalVariableSet(pfx+"showSweep",      g_showSweep?1:0);    break;
        case 6:  g_showFVG        = !g_showFVG;        GlobalVariableSet(pfx+"showFVG",        g_showFVG?1:0);      break;
        case 7:  g_showOB         = !g_showOB;         GlobalVariableSet(pfx+"showOB",         g_showOB?1:0);       break;
        case 9:  g_showSignals    = !g_showSignals;    GlobalVariableSet(pfx+"showSignals",    g_showSignals?1:0);  break;
        case 10: g_showLevels     = !g_showLevels;     GlobalVariableSet(pfx+"showLevels",     g_showLevels?1:0);   break;
        case 11: g_showZones      = !g_showZones;      GlobalVariableSet(pfx+"showZones",      g_showZones?1:0);    break;
        case 12: g_showBgTrend    = !g_showBgTrend;    GlobalVariableSet(pfx+"showBgTrend",    g_showBgTrend?1:0);  break;
        case 13: g_showBgVol      = !g_showBgVol;      GlobalVariableSet(pfx+"showBgVol",      g_showBgVol?1:0);    break;
        case 14: g_showBgAtr      = !g_showBgAtr;      GlobalVariableSet(pfx+"showBgAtr",      g_showBgAtr?1:0);    break;
        case 15: g_showBgPD       = !g_showBgPD;       GlobalVariableSet(pfx+"showBgPD",       g_showBgPD?1:0);     break;
        case 16: g_showBB         = !g_showBB;         GlobalVariableSet(pfx+"showBB",         g_showBB?1:0);       break;
        case 17: g_showPropulsion = !g_showPropulsion; GlobalVariableSet(pfx+"showPropulsion", g_showPropulsion?1:0); break;
        case 18: g_showNDOG       = !g_showNDOG;       GlobalVariableSet(pfx+"showNDOG",       g_showNDOG?1:0);     break;
        case 19: g_showNWOG       = !g_showNWOG;       GlobalVariableSet(pfx+"showNWOG",       g_showNWOG?1:0);     break;
        case 20: g_showAsian      = !g_showAsian;      GlobalVariableSet(pfx+"showAsian",      g_showAsian?1:0);    break;
        case 21: g_showLondon     = !g_showLondon;     GlobalVariableSet(pfx+"showLondon",     g_showLondon?1:0);   break;
        case 22: g_showNY         = !g_showNY;         GlobalVariableSet(pfx+"showNY",         g_showNY?1:0);       break;
        case 23: g_showAsianHL    = !g_showAsianHL;    GlobalVariableSet(pfx+"showAsianHL",    g_showAsianHL?1:0);  break;
        case 24: g_showIB         = !g_showIB;         GlobalVariableSet(pfx+"showIB",         g_showIB?1:0);       break;
        case 25: g_showEqLn       = !g_showEqLn;       GlobalVariableSet(pfx+"showEqLn",       g_showEqLn?1:0);     break;
        case 26: g_showFIB        = !g_showFIB;        GlobalVariableSet(pfx+"showFIB",        g_showFIB?1:0);      break;
        case 27: g_showH1         = !g_showH1;         GlobalVariableSet(pfx+"showH1",         g_showH1?1:0);       break;
        case 28: g_showH4         = !g_showH4;         GlobalVariableSet(pfx+"showH4",         g_showH4?1:0);       break;
        case 29: g_showW1         = !g_showW1;         GlobalVariableSet(pfx+"showW1",         g_showW1?1:0);       break;
        case 30: g_showMN         = !g_showMN;         GlobalVariableSet(pfx+"showMN",         g_showMN?1:0);       break;
        case 31: g_showBPR        = !g_showBPR;        GlobalVariableSet(pfx+"showBPR",        g_showBPR?1:0);      break;
        case 32: g_showBSZones   = !g_showBSZones;   GlobalVariableSet(pfx+"showBSZones",   g_showBSZones?1:0); break;
        case 33: g_showLondonHL  = !g_showLondonHL;  GlobalVariableSet(pfx+"showLondonHL",  g_showLondonHL?1:0); break;
    }
}

// Met à jour la couleur + le texte d'une ligne du panneau selon son type et son état.
// Header : fond bleu sombre, texte du chapitre, pas de ON/OFF.
// Toggle : vert sombre si ON, gris sombre si OFF, texte + "ON"/"OFF".
void UpdateButtonColor(int row) {
    string nm = PANEL_PREFIX + IntegerToString(row);
    if(PANEL_TYPES[row] == PANEL_TYPE_H) {
        ObjectSetInteger(0, nm, OBJPROP_BGCOLOR, clrDarkSlateBlue);
        ObjectSetInteger(0, nm, OBJPROP_COLOR, clrWhite);
        ObjectSetString (0, nm, OBJPROP_TEXT, PANEL_LABELS[row]);
    } else {
        bool isOn = GetShowVar(PANEL_VIDS[row]);
        ObjectSetInteger(0, nm, OBJPROP_BGCOLOR, isOn ? clrDarkGreen : clrDimGray);
        ObjectSetInteger(0, nm, OBJPROP_COLOR, clrWhite);
        ObjectSetString (0, nm, OBJPROP_TEXT, PANEL_LABELS[row] + (isOn ? "  ON" : "  OFF"));
    }
}

// Crée (ou recrée) le panneau : 26 lignes empilées (en-têtes + boutons), ancrées au coin
// haut-droit (CORNER_RIGHT_UPPER). Aucune dépendance au prix/temps : reste fixe au scroll.
// Appelée dans OnInit. Les couleurs sont mises à jour via UpdateButtonColor sur clic.
void CreatePanel() {
    ObjectsDeleteAll(0, PANEL_PREFIX);   // recréation propre (anti-doublons)
    int nbOk = 0;
    for(int row = 0; row < PANEL_ROWS; row++) {
        string nm = PANEL_PREFIX + IntegerToString(row);
        if(ObjectCreate(0, nm, OBJ_BUTTON, 0, 0, 0)) {
            ObjectSetInteger(0, nm, OBJPROP_CORNER, CORNER_RIGHT_UPPER);
            ObjectSetInteger(0, nm, OBJPROP_XDISTANCE, PANEL_X_OFFSET);  // dégage l'échelle de prix
            ObjectSetInteger(0, nm, OBJPROP_YDISTANCE, 5 + row * (PANEL_BTN_H + PANEL_GAP));
            ObjectSetInteger(0, nm, OBJPROP_XSIZE, PANEL_BTN_W);
            ObjectSetInteger(0, nm, OBJPROP_YSIZE, PANEL_BTN_H);
            ObjectSetInteger(0, nm, OBJPROP_FONTSIZE, 8);
            ObjectSetInteger(0, nm, OBJPROP_ALIGN, ALIGN_LEFT);   // texte justifié à gauche
            ObjectSetInteger(0, nm, OBJPROP_SELECTABLE, false);
            UpdateButtonColor(row);
            nbOk++;
        }
    }
    ChartRedraw(0);
}

// OnChartEvent : détecte les clics sur les boutons toggle du panneau et bascule la
// variable g_show* correspondante, puis met à jour la couleur. Les en-têtes (headers)
// sont ignorés (non cliquables). Le recalcul complet (rejeu bar-par-bar dans OnCalculate)
// n'est PAS déclenché ici : il se fera au prochain OnCalculate. Pour forcer un
// rafraîchissement immédiat, l'utilisateur peut changer de timeframe.
void OnChartEvent(const int id, const long &lparam, const double &dparam, const string &sparam) {
    // Redessiner les fonds bgcolor si le graphique est redimensionné/zoomé/déplacé.
    // Les rectangles utilisent CHART_PRICE_MAX/MIN (pleine hauteur) → leurs coordonnées
    // de prix doivent être recalculées quand la zone visible change, sinon ils restent
    // figés à l'ancienne échelle. On ne recalcule PAS les flags (g_bgVol etc. inchangés),
    // on met juste à jour PRICE/PRICE2 de tous les rectangles "BG_" existants.
    if(id == CHARTEVENT_CHART_CHANGE) {
        if(g_showBgTrend || g_showBgVol || g_showBgAtr || g_showBgPD) {
            UpdateBgRectPrices();
        }
        return;
    }
    if(id != CHARTEVENT_OBJECT_CLICK) return;
    if(StringFind(sparam, PANEL_PREFIX) == 0) {
        string idxStr = StringSubstr(sparam, StringLen(PANEL_PREFIX));
        int row = (int)StringToInteger(idxStr);
        if(row >= 0 && row < PANEL_ROWS && PANEL_TYPES[row] == PANEL_TYPE_T) {
            int vid = PANEL_VIDS[row];
            ToggleShowVar(vid);
            UpdateButtonColor(row);
            // Rejeu complet (ChartSetSymbolPeriod) : nécessaire car les fonctions draw lisent
            // les arrays qui ne sont remplis que pendant la boucle. ~1-2s sur 16000 bougies.
            RedrawForToggle(vid);
        }
    }
}

// Charge les toggles d'affichage depuis le disque (GlobalVariableGet).
// Si la variable n'existe pas (première utilisation), utilise le défaut codé en dur.
void LoadToggles() {
    string p = "SMC_" + _Symbol + "_";
    double v;
    // Défauts alignés sur Pine v11 (2026-07-26) : tous false sauf OB et Zones (true Pine).
    // Au démarrage : 2 toggles ON (OB + Zones), 29 OFF — comme le Pine v11.
    g_showStructure  = GlobalVariableGet(p+"showStructure",  v) ? (v>0) : false;
    g_showBOS        = GlobalVariableGet(p+"showBOS",        v) ? (v>0) : false;
    g_showMSS        = GlobalVariableGet(p+"showMSS",        v) ? (v>0) : false;
    g_showCHOCH      = GlobalVariableGet(p+"showCHOCH",      v) ? (v>0) : false;
    g_showLiq        = GlobalVariableGet(p+"showLiq",        v) ? (v>0) : false;
    g_showSweep      = GlobalVariableGet(p+"showSweep",      v) ? (v>0) : false;
    g_showFVG        = GlobalVariableGet(p+"showFVG",        v) ? (v>0) : false;
    g_showOB         = GlobalVariableGet(p+"showOB",         v) ? (v>0) : true;   // Pine true
    g_showSignals    = GlobalVariableGet(p+"showSignals",    v) ? (v>0) : false;
    g_showLevels     = GlobalVariableGet(p+"showLevels",     v) ? (v>0) : false;
    g_showAsianHL    = GlobalVariableGet(p+"showAsianHL",    v) ? (v>0) : false;
    // Phase 1.3/1.4 : IB, Equilibrium, OTE, OB HTF — défaut false (fidèle Pine).
    g_showIB         = GlobalVariableGet(p+"showIB",         v) ? (v>0) : false;
    g_showEqLn       = GlobalVariableGet(p+"showEqLn",       v) ? (v>0) : false;
    g_showFIB        = GlobalVariableGet(p+"showFIB",        v) ? (v>0) : false;
    g_showH1         = GlobalVariableGet(p+"showH1",         v) ? (v>0) : false;
    g_showH4         = GlobalVariableGet(p+"showH4",         v) ? (v>0) : false;
    g_showW1         = GlobalVariableGet(p+"showW1",         v) ? (v>0) : false;
    g_showMN         = GlobalVariableGet(p+"showMN",         v) ? (v>0) : false;
    g_showZones      = GlobalVariableGet(p+"showZones",      v) ? (v>0) : true;   // Pine true
    g_showAsian      = GlobalVariableGet(p+"showAsian",      v) ? (v>0) : false;
    g_showLondon     = GlobalVariableGet(p+"showLondon",     v) ? (v>0) : false;
    g_showNY         = GlobalVariableGet(p+"showNY",         v) ? (v>0) : false;
    g_showBgTrend    = GlobalVariableGet(p+"showBgTrend",    v) ? (v>0) : false;
    g_showBgVol      = GlobalVariableGet(p+"showBgVol",      v) ? (v>0) : false;
    g_showBgAtr      = GlobalVariableGet(p+"showBgAtr",      v) ? (v>0) : false;
    g_showBgPD       = GlobalVariableGet(p+"showBgPD",       v) ? (v>0) : false;
    g_showBB         = GlobalVariableGet(p+"showBB",         v) ? (v>0) : false;
    g_showPropulsion = GlobalVariableGet(p+"showPropulsion", v) ? (v>0) : false;
    g_showNDOG       = GlobalVariableGet(p+"showNDOG",       v) ? (v>0) : false;
    g_showNWOG       = GlobalVariableGet(p+"showNWOG",       v) ? (v>0) : false;
}

int OnInit() {
    InitAssetCalibration();
    // Solution A : charge les toggles depuis le disque (GlobalVariableGet).
    // Fallback sur valeurs par défaut si première utilisation.
    LoadToggles();
    CreatePanel();
    Print("SMC v11 MQL5 — asset=", _Symbol,
          " reconnu=", g_assetReconnu,
          " swingLen=", g_autoSwing,
          " slMode=", g_autoSlMode);
    return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason) {
    ObjectsDeleteAll(0, SMC_PREFIX);   // Nettoyage FIFO complet au detach
    ObjectsDeleteAll(0, PANEL_PREFIX); // Nettoyage du panneau de contrôle au detach
}

// === RedrawForToggle : déclenche un rejeu complet (ChartSetSymbolPeriod) ===
// Rejeu complet sur clic toggle (~1-2s sur 16000 bougies). Tous les toggles sont désormais
// dessinés APRÈS la boucle (BOS inclus depuis le fix 2026-07-27) → le rejeu est nécessaire
// uniquement parce qu'on n'a pas de cache des séries (tentative abandonnée : gel MT5).
// Robuste et éprouvé. La latence est acceptable pour un clic manuel.
void RedrawForToggle(int vid) {
    ChartSetSymbolPeriod(0, _Symbol, _Period);
}

int OnCalculate(const int rates_total,
                const int prev_calculated,
                const datetime &time[],
                const double &open[],
                const double &high[],
                const double &low[],
                const double &close[],
                const long &tick_volume[],
                const long &volume[],
                const int &spread[]) {
    // Anti-repaint : recalcul complet à l'ouverture d'un nouveau bar.
    // + recalcul complet si l'historique vient d'être chargé (prev_calculated == 0) ou
    //   étendu de façon significative (MT5 charge l'historique de façon asynchrone : au
    //   1er chargement, rates_total grossit sur plusieurs OnCalculate qui tombent sur le
    //   MÊME bar en cours → IsNewBar()=false → pas de recalcul → FVG/OB manquants).
    //   On détecte ça via prev_calculated == 0 ou un saut de rates_total > 1.
    bool newBar = IsNewBar();
    bool histReload = (prev_calculated == 0) || (rates_total > prev_calculated + 1);
    if(!newBar && !histReload) return(rates_total);

    // Garde-fou : données non chargées
    if(rates_total < 100) return(rates_total);

    // bar_index absolu Pine-équivalent : [i] = i (index 0 = 1er bar de la série).
    // Reconstruit à chaque OnCalculate ; pas de cast (int&)time qui ne compile pas en MQL5.
    // Servira aussi aux tâches J2-J5 (FVG/OB/signaux). Taille croissante avec l'historique.
    static int g_barIndex[];
    ArrayResize(g_barIndex, rates_total);
    for(int k = 0; k < rates_total; k++) g_barIndex[k] = k;

    // === Rejeu de l'historique bar-par-bar (comme le Pine) ===
    // Le Pine exécute son script séquentiellement sur chaque bar historique lors du
    // chargement : c'est ainsi qu'il accumule les pivots et détecte tous les BOS passés.
    // À chaque nouveau bar on recalcule TOUT depuis le début. Simple et fidèle.
    ResetSmcState();

    // Les 5 pools events structure (g_poolHH/BOS/MSS/CHOCH/SWP) sont des arrays statiques
    // de taille fixe — pas de ArrayResize nécessaire. Vidés par ResetSmcState ci-dessus.

    // Précalcul de l'ATR14 pour tout l'historique, en une seule passe O(n).
    // Lu ensuite en O(1) via g_atr14[i] dans la boucle (évite le timeout O(n²) observé).
    PrecalcAtr14(rates_total, high, low, close);

    // Précalcul des états f_htf HTF (J4e) pour les 4 TF supérieurs, en une passe O(n_htf) chacun.
    // Rejoue le moteur BOS/OB/mitigation Pine f_htf(3) sur les bougies HTF (y compris la bougie
    // en cours, aligné Pine LIVE) et stocke l'état dans g_htfStates_*. La boucle principale lit
    // en O(1) via iBarShift (HtfReadState → states[idxHtf] = bougie HTF en cours, repaint assumé,
    // décision Rono 2026-07-23 pour matcher le scoring TV).
    // Données HTF chargées de façon asynchrone : si non prêtes au 1er OnCalculate, les états
    // restent vides (confluence false) puis se rempliront au prochain recalcul (nouveau bar).
    PrecalcHtf(PERIOD_H1, g_htfStatesH1);
    PrecalcHtf(PERIOD_H4, g_htfStatesH4);
    PrecalcHtf(PERIOD_W1, g_htfStatesW1);
    PrecalcHtf(PERIOD_MN1, g_htfStatesMN);

    // g_znDaxHTF (Pine ligne 878) : DAX M15/M30 → filtres FVG/DoL neutralisés.
    // Calculé une fois (ne dépend que de l'asset/TF) avant la boucle bar-par-bar.
    // g_isDAX/g_tf sont positionnés par InitAssetCalibration (OnInit).
    g_znDaxHTF = g_isDAX && (g_tf >= 15);

    int startBar = g_autoSwing * 2 + 1;   // 1er bar où un pivot peut être confirmé
    for(int i = startBar; i <= rates_total - 2; i++) {
        // i = bar courant clôturé traité (= "bar_index" Pine sous barstate.isconfirmed).
        // Anti-repaint strict : on s'arrête à rates_total-2 (rates_total-1 = bar en cours,
        // interdit). Ordre obligatoire :
        //   structure → bos → choch → pdOte → prevLiq → asianHL → draw → liq → sweep
        //   → fvgCreate → fvgLifecycle → obCreate → obLifecycle → htfCompute → accumScores
        double atr14 = Atr14At(i);   // O(1), lu depuis le tableau précalculé

        f_structureCompute  (i, g_barIndex, time, high, low);
        f_bosCompute        (i, close, open);
        f_chochCompute      (i, g_barIndex, time);
        // Premium/Discount + OTE (J4d) APRÈS chochCompute (a besoin de g_bosHaussier/
        // g_bosBaissier figés au bar i) et AVANT accumScores (f_score y lit g_inPremium/
        // g_inDiscount/g_inOTE_*). 100% local (swings g_sh1/g_sl1), aucun effet visuel.
        f_pdOteCompute      (i, g_barIndex, time, close);
        // prevLiq PDH/PDL/PWH/PWL (J4b) APRÈS pdOteCompute et AVANT accumScores : f_score
        // consomme g_near*/g_sweep* (composantes +2/+4), et DoL (f_znQualBull/Bear, appelé
        // dans f_createBuy/SellSignals) lit g_pdh/g_pwh. Anti-repaint : iBarShift sur time[i]
        // + shift D1/W1 = idx+1 (jamais 0).
        f_prevLiqCompute    (i, time, high, low, close, atr14);
        f_megaVolCompute    (i, tick_volume);   // Module H — avant accumScores
        // Asian HL (J4b) APRÈS prevLiqCompute et AVANT accumScores : DoL (f_znQualBull/Bear)
        // lit g_ahHighDrawn/g_ahLowDrawn (figés à la fin de session Asie + invalidés si cassés).
        f_asianHlCompute    (i, time, high, low, close);
        f_londonHlCompute   (i, time, high, low);         // MODULE 14b (miroir v12)
        // BOS : plus de f_drawStructureBOS in-boucle (gélait MT5 sur rejeu complet).
        // Désormais capturé dans g_structEvents (kinds 11/12) et dessiné après la boucle
        // par f_drawStructEvents (même pattern que MSS/CHOCH/Sweep). Fix 2026-07-27.
        f_liqCompute        (i, close, time, atr14);
        f_sweepCompute      (i, high, low, close, g_barIndex, time);
        f_fvgCreate         (i, high, low, atr14);
        f_bprCreate         (i);                        // MODULE 6b — AVANT le lifecycle FVG (parité Pine : gap rempli appariantable)
        f_fvgLifecycle      (i, high, low, close);
        f_bprLifecycle      (i, high, low, close);      // MODULE 6b
        f_bsCompute         (i, time, open, high, low, close, tick_volume, atr14); // MODULE BSZones (miroir v12)
        f_obCreate          (i, time, open, high, low, close);
        f_obLifecycle       (i, time, high, low, close);
        // Phase 1.3 : Imbalance (Inner Bar) — détection + lifecycle (après obLifecycle).
        f_ibCompute         (i, time, open, close, atr14, g_barIndex[i]);
        f_ibLifecycle       (i, close);
        // Modules décoratifs étape 5 (DÉCORATIF, hors scoring) — APRÈS obLifecycle (qui capture
        // les Breakers à l'invalidation des OB) et APRÈS fvgCreate+obCreate (dont f_propCompute
        // lit les arrays) :
        //   - f_breakerLifecycle : consume les Breakers traversés (Pine f_bbLifecycle).
        //   - f_propCompute      : Propulsion = chevauchement FVG∩OB même sens (Pine MODULE 8c).
        //   - f_propLifecycle    : consume les Propulsion traversés (Pine f_propBull/BearLifecycle).
        //   - f_gapCompute       : NDOG/NWOG = gap open nouveau jour/semaine vs close veille.
        f_breakerLifecycle  (i, close);
        f_propCompute       (i, time, high, low);
        f_propLifecycle     (i, close);
        f_gapCompute        (i, time, open, close, atr14);
        f_gapLifecycle      (i, high, low);   // mitigation NDOG/NWOG (traversée → recoloration)
        // HTF confluence (J4e) APRÈS prevLiq/asianHL et AVANT accumScores : f_score consomme
        // g_confluence*/g_*Bull1Valid (composantes H4=4/H1=1/W1=5/MN=6 + garde anti-bruit hsHTF).
        // Anti-repaint : état HTF lu via iBarShift(time[i]) à la bougie HTF précédente clôturée.
        f_htfCompute        (i, time, close);
        // Accumulation scores OB APRÈS obLifecycle : on ne score QUE les OB survivants
        // (sinon on scorerait des OB que le lifecycle va supprimer juste après).
        f_accumScores       (i, time, open, high, low, close, atr14);

        // Stockage états bgcolor (SECTION 14ter, phase décorative étape 3). APRÈS accumScores :
        // les flags g_tendance*/g_inPremium/g_inDiscount sont figés au bar i par les modules
        // précédents. Ne stocke QUE les BGCOLOR_BARS dernières bougies (les plus récentes),
        // dessinées ensuite par f_drawBg après la boucle. Anti-repaint : i <= rates_total-2.
        f_storeBg(i, rates_total, tick_volume, high, low, close, open, atr14);

        // Génération signaux (Pine f_createBuySignals/Sell). Une fois les scores accumulés,
        // on teste le retour prix au bord des OB et on émet au plus 1 signal (BUY ou SELL).
        // f_tradeBloquant en tête de chaque fonction : si un trade est ouvert, on skip.
        f_createBuySignals (i, time, close, atr14);
        f_createSellSignals(i, time, close, atr14);
        f_createBsBuySignals (i, time, close, atr14);   // moteur B (miroir v12)
        f_createBsSellSignals(i, time, close, atr14);
        // Machine à états lifecycle trades (clone Pine) : fill, TP1/BE, TP2, clôtures, expire.
        // Recalcule aussi g_tradeOuvert (bloquant = filled && !t1Hit && !closed).
        f_updateTradeState(i, time, high, low, close);
    }

    // Affichage FVG APRÈS la boucle (Option B) : les arrays g_fvgBull/g_fvgBear ne
    // contiennent plus que les FVG actifs/mitigés (suppression faite en lifecycle),
    // ce qui reflète l'état courant du Pine à l'instant T. Appelée ici une seule fois,
    // jamais dans la boucle (sinon elle effacerait/recréerait tout à chaque bar).
    f_drawFVG(time);
    f_drawBPR(time);   // MODULE 6b (miroir v12)
    f_drawBsZones(time);   // moteur B (miroir v12)
    // Affichage OB APRÈS la boucle (Option B) : idem, les arrays ne contiennent plus que
    // les OB non invalidés (suppression faite en f_obLifecycle), avec leur state de
    // mitigation à jour. Comportement 1:1 vs Pine à l'instant T.
    f_drawOB(time);
    // Affichage signaux APRÈS la boucle (Option B) : tous les signaux accumulés dans
    // g_signals[] sont dessinés (flèches + lignes entry/SL/TP1). État final = Pine à l'instant T.
    f_drawSignals(time);
    // Affichage niveaux décoratifs APRÈS la boucle (Option B) : PDH/PDL/PWH/PWL + Asian HL.
    // Lignes horizontales du début du graphique au bar courant. État final = Pine à l'instant T.
    f_drawLevels(time);
    // Asian HL séparé de PDH/PDL/PWH/PWL (toggle indépendant g_showAsianHL, préfixe "AHL").
    f_drawAsianHL(time);
    // Phase 1.2 : Affichage des rectangles de Sessions (Asie, Londres, NY).
    f_drawSessions(time, high, low);
    // Affichage structure HH/HL/LH/LL + lignes MSS/CHOCH APRÈS la boucle (Option B, phase
    // décorative étape 2). Les événements ont été capturés dans g_structEvents[] pendant la
    // boucle. État final = Pine à l'instant T.
    f_drawStructEvents(time);
    // Affichage fonds bgcolor (Option B, phase décorative étape 3). Les états ont été stockés
    // dans g_bg* pendant la boucle par f_storeBg (uniquement les BGCOLOR_BARS dernières bougies).
    // Dessine 1 rectangle par bar active pour chaque type activé. État final = Pine à l'instant T.
    // NB : ne fait rien si aucun InpShowBg* n'est activé (sortie anticipée dans f_drawBg).
    f_drawBg(time, high, low);
    // Affichage Zones d'Achat/Vente (Option B, phase décorative étape 4). Dessine un rectangle
    // + label "ACHAT/VENTE force/10" pour chaque OB qualifié (force ≥ FORCE_MIN && f_znQual &&
    // assetReconnu && non signaled). État final = Pine à l'instant T.
    f_drawZones(time);
    // Affichage liquidité EQH/EQL (Option B, phase décorative étape 4). Lignes horizontales
    // + labels au niveau de la liquidité EQH/EQL archivée. État final = Pine à l'instant T.
    f_drawLiq(time);
    // Affichage Breaker / Propulsion / NDOG-NWOG (Option B, phase décorative étape 5).
    // DÉCORATIF (hors scoring). Les zones ont été accumulées dans g_bbBull/g_bbBear (Breakers)
    // et g_propBull/g_propBear (Propulsion) pendant la boucle (captures dans f_obLifecycle +
    // f_propCompute + f_gapCompute) et leur lifecycle géré par f_breakerLifecycle/f_propLifecycle.
    // État final = Pine à l'instant T. NB : ne font rien
    // si leur toggle (g_showBB/Propulsion/NDOG/NWOG) est à false (sortie anticipée dans chaque).
    f_drawBreakers(time);
    f_drawPropulsion(time);
    f_drawGaps(time);
    // Phase 1.3/1.4 : IB, Equilibrium, OTE, OB HTF (4 nouvelles fonctions draw).
    f_drawIB(time);
    f_drawEqLn(time);
    f_drawOTE(time);
    f_drawHtf(time, close);



    return(rates_total);
}
