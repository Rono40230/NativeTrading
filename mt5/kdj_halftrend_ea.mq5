// ════════════════════════════════════════════════════════════════════
//  KDJ/HALTREND « 550 % » — EA miroir Pine étalon + Rust
//  Référence : docs/reference/definition_kdj_halftrend.md (md5 étalon
//  be3343edd854d0759c6ac23bb2ec2b69) — toute divergence est un bug.
//
//  Modèle d'exécution (piège 1 de la définition) : AUCUN ordre limit/stop.
//  Signaux et crosses TP/SL détectés à la CLÔTURE de barre, ordre marché
//  passé à l'open de la barre suivante (= ici : immédiatement, au premier
//  tick de la nouvelle barre). TP/SL = seuils de détection uniquement.
//
//  Fige (piège 3) : E et EMA200_E viennent de la BARRE D'ENTRÉE clôturée
//  (valuewhen) — l'EA fige les niveaux à la clôture de la barre d'entrée.
//  Crosses (piège 4) : crossover/crossunder, jamais de test de niveau nu.
// ════════════════════════════════════════════════════════════════════
#property copyright "Rono"
#property version   "1.00"
#property strict
#include <Trade\Trade.mqh>

// --- Inputs (les 4 de l'étalon + exécution) ---
input int    InpPeriod    = 20;      // KDJ : period (fenêtre high/low)
input int    InpSignal    = 7;       // KDJ : signal (lissage K et D)
input int    InpAmplitude = 2;       // HalfTrend : amplitude
input double InpRatioRisk = 2.0;     // TP = RatioRisk × distance entrée→EMA200
input double InpLot       = 0.01;    // lot (indifférent : on compare les signaux)
input int    InpFenetre   = 3000;    // barres clôturées chargées (0 = tout)
input long   InpMagic     = 20260913;
input bool   InpDiagCsv   = true;    // dump CSV MQL5/Files/ pour la diff 3 voies

CTrade trade;

// --- Position en cours (état EA) ---
struct PositionKdj {
   bool     ouverte;
   int      direction;     // 1 long, −1 short
   datetime tsEntree;      // open-time de la barre d'entrée
   double   openEntree;    // open THÉORIQUE de la barre d'entrée (convention diff)
   double   fillEntree;    // fill réel
   bool     niveauxFiges;  // figés à la clôture de la barre d'entrée
   double   slNiveau;      // = EMA200_E (détection)
   double   tpNiveau;      // = E ± RatioRisk × (E − EMA200_E)
   double   risque;        // |E − EMA200_E|
};
PositionKdj g_pos;

datetime g_derniereBarre = 0;
int      g_fhDiag = INVALID_HANDLE;

// Valeur « non calculé » (MQL5 n'a pas de NaN commode).
#define KDJ_NA  (EMPTY_VALUE)

// ════════════════════════════════════════════════════════════════════
//  Indicateurs — transpositions exactes de backend/crates/indicators
//  (même seeds, mêmes NaN) : la diff au centime en dépend.
//  Tous les tableaux sont CHRONOLOGIQUES (index 0 = plus ancien).
// ════════════════════════════════════════════════════════════════════

// SMA sur close — KDJ_NA avant periode−1 (calculer_sma).
void CalcSma(const double &close[], int n, int periode, double &sma[]) {
   for(int i = 0; i < n; i++) sma[i] = KDJ_NA;
   if(periode <= 0 || n < periode) return;
   double somme = 0.0;
   for(int i = 0; i < n; i++) {
      somme += close[i];
      if(i >= periode) somme -= close[i - periode];
      if(i >= periode - 1) sma[i] = somme / periode;
   }
}

// EMA sur close — seed SMA des `periode` premières (calculer_ema).
void CalcEma(const double &close[], int n, int periode, double &ema[]) {
   for(int i = 0; i < n; i++) ema[i] = KDJ_NA;
   if(periode <= 0 || n < periode) return;
   double seed = 0.0;
   for(int i = 0; i < periode; i++) seed += close[i];
   seed /= periode;
   ema[periode - 1] = seed;
   double k = 2.0 / (periode + 1.0);
   for(int i = periode; i < n; i++) ema[i] = close[i] * k + ema[i - 1] * (1.0 - k);
}

// ATR Wilder — init = moyenne des `periode` premiers TR (calculer_atr).
// tr(i) = max(h−l, |h−c[i−1]|, |l−c[i−1]|) ; atr[periode] défini.
void CalcAtr(const double &high[], const double &low[], const double &close[],
             int n, int periode, double &atr[]) {
   for(int i = 0; i < n; i++) atr[i] = KDJ_NA;
   if(periode <= 0 || n <= periode) return;
   double somme = 0.0;
   for(int i = 1; i <= periode; i++) {
      double tr = MathMax(high[i] - low[i],
                  MathMax(MathAbs(high[i] - close[i - 1]), MathAbs(low[i] - close[i - 1])));
      somme += tr;
   }
   atr[periode] = somme / periode;
   for(int i = periode + 1; i < n; i++) {
      double tr = MathMax(high[i] - low[i],
                  MathMax(MathAbs(high[i] - close[i - 1]), MathAbs(low[i] - close[i - 1])));
      atr[i] = (atr[i - 1] * (periode - 1.0) + tr) / periode;
   }
}

// KDJ — RSV fenêtré + double lissage seed 0 (nz de l'étalon).
void CalcKdj(const double &high[], const double &low[], const double &close[],
             int n, int period, int sig, double &k[], double &d[], double &j[]) {
   for(int i = 0; i < n; i++) { k[i] = KDJ_NA; d[i] = KDJ_NA; j[i] = KDJ_NA; }
   if(period <= 0 || sig <= 0 || n < period) return;
   double m = 1.0, s = sig;
   // Fenêtre glissante high/low (identique au Rust, O(n×period) — fenêtre bornée).
   for(int i = period - 1; i < n; i++) {
      double hh = -DBL_MAX, ll = DBL_MAX;
      for(int w = i + 1 - period; w <= i; w++) {
         if(high[w] > hh) hh = high[w];
         if(low[w]  < ll) ll = low[w];
      }
      double rsv = (hh > ll) ? 100.0 * (close[i] - ll) / (hh - ll) : KDJ_NA;
      double kPrec = (i == 0 || k[i - 1] == KDJ_NA) ? 0.0 : k[i - 1];
      double dPrec = (i == 0 || d[i - 1] == KDJ_NA) ? 0.0 : d[i - 1];
      k[i] = (rsv == KDJ_NA) ? KDJ_NA : (m * rsv + (s - m) * kPrec) / s;
      d[i] = (k[i] == KDJ_NA) ? KDJ_NA : (m * k[i] + (s - m) * dPrec) / s;
      j[i] = (k[i] == KDJ_NA || d[i] == KDJ_NA) ? KDJ_NA : 3.0 * k[i] - 2.0 * d[i];
   }
}

// HalfTrend — machine d'état de l'étalon (channelDeviation=2 codé dur,
// atr2 = atr(100)/2). Flèches uniquement au retournement, ATR prêt.
void CalcHalftrend(const double &high[], const double &low[], const double &close[],
                   int n, int amplitude, const double &atr[],
                   int &trend[], bool &arrowUp[], bool &arrowDown[]) {
   for(int i = 0; i < n; i++) { trend[i] = 0; arrowUp[i] = false; arrowDown[i] = false; }
   if(n == 0) return;
   int nextTrend = 0;
   double maxLowPrice = low[0], minHighPrice = high[0];
   double up = KDJ_NA, down = KDJ_NA;

   for(int i = 0; i < n; i++) {
      // Extrêmes des `amplitude` dernières barres (bornés).
      int debut = i - (amplitude - 1); if(debut < 0) debut = 0;
      double hh = -DBL_MAX, ll = DBL_MAX;
      for(int w = debut; w <= i; w++) {
         if(high[w] > hh) hh = high[w];
         if(low[w]  < ll) ll = low[w];
      }
      // sma(high, amplitude) / sma(low, amplitude) — NA si fenêtre incomplète.
      double highma = KDJ_NA, lowma = KDJ_NA;
      if(amplitude > 0 && i + 1 >= amplitude) {
         double sh = 0.0, sl = 0.0;
         for(int w = i + 1 - amplitude; w <= i; w++) { sh += high[w]; sl += low[w]; }
         highma = sh / amplitude; lowma = sl / amplitude;
      }
      bool atr2Pret = (atr[i] != KDJ_NA);

      trend[i] = (i == 0) ? 0 : trend[i - 1];
      if(nextTrend == 1) {
         if(ll > maxLowPrice) maxLowPrice = ll;
         if(i >= 1 && highma != KDJ_NA && highma < maxLowPrice && close[i] < low[i - 1]) {
            trend[i] = 1; nextTrend = 0; minHighPrice = hh;
         }
      } else {
         if(hh < minHighPrice) minHighPrice = hh;
         if(i >= 1 && lowma != KDJ_NA && lowma > minHighPrice && close[i] > high[i - 1]) {
            trend[i] = 0; nextTrend = 1; maxLowPrice = ll;
         }
      }

      if(trend[i] == 0) {
         if(i >= 1 && trend[i - 1] != 0) {
            up = down; // down[1] ou down (var identique dans l'étalon)
            arrowUp[i] = atr2Pret && (up != KDJ_NA);
         } else {
            up = (i == 0 || up == KDJ_NA) ? maxLowPrice : MathMax(maxLowPrice, up);
         }
      } else {
         if(i >= 1 && trend[i - 1] != 1) {
            down = up;
            arrowDown[i] = atr2Pret && (down != KDJ_NA);
         } else {
            down = (i == 0 || down == KDJ_NA) ? minHighPrice : MathMin(minHighPrice, down);
         }
      }
   }
}

// ════════════════════════════════════════════════════════════════════
//  Exécution + dump
// ════════════════════════════════════════════════════════════════════

void DumpTrade(string verdict, datetime tsSortie, double openSortie, double fillSortie) {
   // Champs séparés (FILE_CSV) — DoubleToString force le point décimal,
   // insensible aux paramètres régionaux. Flush immédiat : le CSV doit être
   // lisible au fil de l'eau, même si le test est arrêté brutalement.
   if(g_fhDiag != INVALID_HANDLE) {
      FileWrite(g_fhDiag,
         (string)(long)g_pos.tsEntree, (string)g_pos.direction,
         DoubleToString(g_pos.openEntree, 5), DoubleToString(g_pos.fillEntree, 5),
         DoubleToString(g_pos.slNiveau, 5), DoubleToString(g_pos.tpNiveau, 5),
         (string)(long)tsSortie, DoubleToString(openSortie, 5),
         DoubleToString(fillSortie, 5), verdict);
      FileFlush(g_fhDiag);
   }
   PrintFormat("[KDJ diag TRADE] %I64d,%d,%.5f,%.5f,%.5f,%.5f,%I64d,%.5f,%.5f,%s",
      (long)g_pos.tsEntree, g_pos.direction, g_pos.openEntree, g_pos.fillEntree,
      g_pos.slNiveau, g_pos.tpNiveau, (long)tsSortie, openSortie, fillSortie, verdict);
}

bool OuvrirPosition(int direction) {
   // Entrée marché = open de la barre qui vient de s'ouvrir (modèle TV).
   double fill;
   bool ok;
   if(direction == 1) ok = trade.Buy(InpLot, _Symbol, 0.0, 0.0, 0.0, "kdj long");
   else               ok = trade.Sell(InpLot, _Symbol, 0.0, 0.0, 0.0, "kdj short");
   if(!ok) { Print("[KDJ diag] ordre refusé ", trade.ResultRetcode()); return false; }
   fill = trade.ResultPrice();
   if(fill <= 0.0) fill = SymbolInfoDouble(_Symbol, SYMBOL_BID);
   g_pos.ouverte = true;
   g_pos.direction = direction;
   g_pos.tsEntree = iTime(_Symbol, _Period, 0);   // barre en cours = barre d'entrée
   g_pos.openEntree = iOpen(_Symbol, _Period, 0); // open théorique (convention diff)
   g_pos.fillEntree = fill;
   g_pos.niveauxFiges = false;
   g_pos.slNiveau = 0.0; g_pos.tpNiveau = 0.0; g_pos.risque = 0.0;
   return true;
}

bool FermerPosition() {
   if(!trade.PositionClose(_Symbol)) {
      Print("[KDJ diag] clôture refusée ", trade.ResultRetcode());
      return false;
   }
   return true;
}

// ════════════════════════════════════════════════════════════════════
//  Cycle de vie
// ════════════════════════════════════════════════════════════════════

bool IsNewBar() {
   datetime t = iTime(_Symbol, _Period, 0);
   if(t == 0) return false;
   if(t != g_derniereBarre) { g_derniereBarre = t; return true; }
   return false;
}

int OnInit() {
   trade.SetExpertMagicNumber((ulong)InpMagic);
   trade.SetDeviationInPoints(20);
   g_pos.ouverte = false;
   if(InpDiagCsv) {
      string nom = StringFormat("kdj_diag_%s_%s.csv", _Symbol, EnumToString(_Period));
      g_fhDiag = FileOpen(nom, FILE_WRITE | FILE_CSV | FILE_ANSI, ',');
      if(g_fhDiag != INVALID_HANDLE) {
         FileWrite(g_fhDiag, "ts_entree,dir,open_entree,fill_entree,sl_niveau,tp_niveau,ts_sortie,open_sortie,fill_sortie,verdict");
         FileFlush(g_fhDiag);
      }
   }
   return INIT_SUCCEEDED;
}

void OnDeinit(const int reason) {
   // Position résiduelle : dump « Ouvert » (exclue des stats, comme le rejeu).
   if(g_pos.ouverte) DumpTrade("Ouvert", 0, 0.0, 0.0);
   if(g_fhDiag != INVALID_HANDLE) { FileClose(g_fhDiag); g_fhDiag = INVALID_HANDLE; }

   // Dump des bougies de la fenêtre (heure serveur) : le rejeu Rust rejoue
   // CES bougies pour la parité au centime (l'historique du tester peut
   // différer de la base du collecteur sur l'ancien).
   int total = Bars(_Symbol, _Period);
   int nb = InpFenetre > 0 ? (int)MathMin(InpFenetre, total - 1) : total - 1;
   if(nb > 0) {
      int fb = FileOpen(StringFormat("kdj_bougies_%s_%s.csv", _Symbol, EnumToString(_Period)),
                        FILE_WRITE | FILE_CSV | FILE_ANSI, ',');
      if(fb != INVALID_HANDLE) {
         FileWrite(fb, "ts,open,high,low,close");
         for(int k = nb; k >= 1; k--)
            FileWrite(fb, (string)(long)iTime(_Symbol, _Period, k),
                      DoubleToString(iOpen (_Symbol, _Period, k), 5),
                      DoubleToString(iHigh (_Symbol, _Period, k), 5),
                      DoubleToString(iLow  (_Symbol, _Period, k), 5),
                      DoubleToString(iClose(_Symbol, _Period, k), 5));
         FileClose(fb);
      }
   }
}

void OnTick() {
   if(!IsNewBar()) return;
   int total = Bars(_Symbol, _Period);
   if(total < 260) return;

   int nb = InpFenetre > 0 ? (int)MathMin(InpFenetre, total - 1) : total - 1;
   if(nb < 260) return;

   // Séries chronologiques : shift 1..nb → index 0 = plus ancien, nb−1 = dernière clôturée.
   static datetime time[]; static double open[], high[], low[], close[];
   ArraySetAsSeries(time, false); ArraySetAsSeries(open, false);
   ArraySetAsSeries(high, false); ArraySetAsSeries(low, false); ArraySetAsSeries(close, false);
   if(CopyTime (_Symbol, _Period, 1, nb, time)  != nb) return;
   if(CopyOpen (_Symbol, _Period, 1, nb, open)  != nb) return;
   if(CopyHigh (_Symbol, _Period, 1, nb, high)  != nb) return;
   if(CopyLow  (_Symbol, _Period, 1, nb, low)   != nb) return;
   if(CopyClose(_Symbol, _Period, 1, nb, close) != nb) return;

   static double k[], d[], j[], sma[], ema[], atr[];
   static int trend[]; static bool aUp[], aDn[];
   ArrayResize(k, nb); ArrayResize(d, nb); ArrayResize(j, nb);
   ArrayResize(sma, nb); ArrayResize(ema, nb); ArrayResize(atr, nb);
   ArrayResize(trend, nb); ArrayResize(aUp, nb); ArrayResize(aDn, nb);

   CalcSma(close, nb, 100, sma);
   CalcEma(close, nb, 200, ema);
   CalcAtr(high, low, close, nb, 100, atr);
   CalcKdj(high, low, close, nb, InpPeriod, InpSignal, k, d, j);
   CalcHalftrend(high, low, close, nb, InpAmplitude, atr, trend, aUp, aDn);

   int i = nb - 1; // dernière barre CLÔTURÉE — toute l'évaluation porte sur elle.
   if(i < 210) return;

   // Conditions (NaN-safe) et signaux à la clôture de i.
   bool condOk = (sma[i] != KDJ_NA && ema[i] != KDJ_NA && j[i] != KDJ_NA && d[i] != KDJ_NA);
   bool longOk  = condOk && sma[i] > ema[i] && j[i] > d[i] && close[i] > ema[i];
   bool shortOk = condOk && sma[i] < ema[i] && j[i] < d[i] && close[i] < ema[i];
   bool buy  = aUp[i] && longOk;
   bool sell = aDn[i] && shortOk;

   // ── 1) Fige à la clôture de la barre d'entrée (piège 3 : valuewhen) ──
   if(g_pos.ouverte && !g_pos.niveauxFiges) {
      if(time[i] == g_pos.tsEntree && ema[i] != KDJ_NA) {
         double e = g_pos.openEntree, emaE = ema[i];
         double distance = e - emaE;
         if(g_pos.direction == 1) {
            g_pos.slNiveau = emaE;
            g_pos.tpNiveau = e + InpRatioRisk * distance;
            g_pos.risque = MathAbs(distance);
         } else {
            g_pos.slNiveau = emaE;
            g_pos.tpNiveau = e - InpRatioRisk * (emaE - e);
            g_pos.risque = MathAbs(emaE - e);
         }
         g_pos.niveauxFiges = true;
         Print("[KDJ diag FIGE] ", TimeToString(g_pos.tsEntree), " dir=", g_pos.direction,
               " E=", DoubleToString(e, 5), " emaE=", DoubleToString(emaE, 5),
               " tp=", DoubleToString(g_pos.tpNiveau, 5), " sl=", DoubleToString(g_pos.slNiveau, 5));
      } else if(time[i] > g_pos.tsEntree) {
         // La barre d'entrée n'est plus dans la fenêtre (fenêtre glissante courte) :
         // on ne peut plus figer fidèlement — signalé, trade fermé au plus vite.
         Print("[KDJ diag ERREUR] barre d'entrée hors fenêtre — clôture forcée");
         if(FermerPosition()) DumpTrade("ErreurFenetre", time[i], open[i], close[i]);
         g_pos.ouverte = false;
      }
   }

   // ── 2) Crosses TP/SL sur la barre clôturée i (piège 4 : croisements) ──
   int verdictSortie = 0; // 1 TP, 2 SL, 3 retournement
   if(g_pos.ouverte && g_pos.niveauxFiges) {
      bool tp, sl;
      if(g_pos.direction == 1) {
         tp = high[i - 1] <= g_pos.tpNiveau && high[i] > g_pos.tpNiveau;
         sl = low[i - 1]  >= g_pos.slNiveau && low[i]  < g_pos.slNiveau;
      } else {
         tp = low[i - 1]  >= g_pos.tpNiveau && low[i]  < g_pos.tpNiveau;
         sl = high[i - 1] <= g_pos.slNiveau && high[i] > g_pos.slNiveau;
      }
      if(tp) verdictSortie = 1; else if(sl) verdictSortie = 2;
   }
   bool retournement = g_pos.ouverte &&
      ((g_pos.direction == 1 && sell) || (g_pos.direction == -1 && buy));
   if(retournement && verdictSortie == 0) verdictSortie = 3;

   // ── 3) Exécutions (open de la barre qui vient de s'ouvrir = maintenant) ──
   if(verdictSortie > 0 && FermerPosition()) {
      string v = (verdictSortie == 1) ? "TP" : (verdictSortie == 2) ? "SL" : "Retournement";
      DumpTrade(v, iTime(_Symbol, _Period, 0), iOpen(_Symbol, _Period, 0),
                SymbolInfoDouble(_Symbol, SYMBOL_BID));
      g_pos.ouverte = false;
   }
   if(!g_pos.ouverte) {
      // Entrée (ou retournement déjà fermé ci-dessus) — au même open (modèle TV).
      if(buy) OuvrirPosition(1);
      else if(sell) OuvrirPosition(-1);
   }
}
