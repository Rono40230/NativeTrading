//+------------------------------------------------------------------+
//| EA_Collecteur.mq5 — Native Trading AI                            |
//|                                                                  |
//| Collecteur MT5/Axi : pousse les bougies du broker vers           |
//| l'application (même chemin que Bybit).                           |
//|                                                                  |
//| INSTALLATION :                                                   |
//|  1. Compiler ce fichier dans MetaEditor (F7).                    |
//|  2. MT5 → Outils → Options → Conseillers Experts →               |
//|     « Autoriser WebRequest pour les URL listées » → ajouter :    |
//|     http://127.0.0.1:8080  (localhost refusé par MT5 — bug connu)|
//|  3. Attacher l'EA à N'IMPORTE QUEL graphique (il collecte tous   |
//|     les symboles configurés dans l'application).                 |
//|                                                                  |
//| MÉCANIQUE :                                                      |
//|  - Toutes les 30 s : liste des symboles + TF à collecter +       |
//|    heartbeat. En-tête « #TF M1:10080,... » = profondeurs.        |
//|  - Nouveau symbole : pousser l'HISTORIQUE Axi de chaque TF      |
//|    (batch JSON) — les moteurs SMC s'arment dessus ensuite.       |
//|  - Chaque seconde : bougie en formation de chaque (symbole, TF) ;|
//|    au changement de période : clôture officielle (conf=1).       |
//+------------------------------------------------------------------+
#property copyright "Native Trading AI"
#property version   "1.30"
#property strict

input string ApiUrl = "http://127.0.0.1:8080"; // URL de l'application (localhost refusé par MT5 — bug connu)

// Abonnements
string   symboles[];       // nom broker réel (résolu, casse exacte)
string   assets[];         // nom dans l'app
bool     historique_fait[];// historique poussé pour ce symbole ?

// Timeframes demandés par l'app
string   tf_noms[];
ENUM_TIMEFRAMES tf_periodes[];
int      tf_profondeurs[];
int      NB_TF = 0;

// Suivi live par (symbole × tf) — tableaux aplatis
datetime dernier_debut[];
double   dernier_close[];

int      ticks_timer = 0;

//+------------------------------------------------------------------+
int OnInit()
{
   EventSetTimer(1);
   rafraichir_abonnements();
   Print("EA_Collecteur: démarré — cible ", ApiUrl);
   return(INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
   EventKillTimer();
   Print("EA_Collecteur: arrêt (raison ", reason, ")");
}

//+------------------------------------------------------------------+
void OnTimer()
{
   ticks_timer++;
   if(ticks_timer % 30 == 0)
   {
      heartbeat();
      rafraichir_abonnements();
   }
   pousser_bougies();
}

//+------------------------------------------------------------------+
int idx(const int s, const int t) { return s * NB_TF + t; }

//+------------------------------------------------------------------+
//| Liste des symboles : GET /api/mt5/symboles                       |
//| En-tête « #TF M1:10080,M5:2016,... » puis « symbole|asset ».     |
//+------------------------------------------------------------------+
void rafraichir_abonnements()
{
   char vide[];
   char resultat[];
   string en_tetes_reponse = "";
   ResetLastError();
   int statut = WebRequest("GET", ApiUrl + "/api/mt5/symboles", "", 5000,
                           vide, resultat, en_tetes_reponse);
   if(statut == -1)
   {
      int err = GetLastError();
      if(err == 4014)
         Print("EA_Collecteur: ERREUR — URL non autorisée. Outils > Options > Conseillers Experts > ajouter ", ApiUrl);
      return;
   }
   string reponse = "";
   for(int i = 0; i < ArraySize(resultat); i++)
      reponse += CharToString(resultat[i]);

   string lignes[];
   string nouvelles_symboles[];
   string nouveaux_assets[];
   StringSplit(reponse, '\n', lignes);
   for(int i = 0; i < ArraySize(lignes); i++)
   {
      string ligne = lignes[i];
      StringTrimLeft(ligne);
      StringTrimRight(ligne);
      if(StringLen(ligne) == 0) continue;

      if(StringFind(ligne, "#TF") == 0)
      {
         lire_tfs(StringSubstr(ligne, 4));
         continue;
      }
      int sep = StringFind(ligne, "|");
      if(sep <= 0) continue;
      string symbole = StringSubstr(ligne, 0, sep);
      string asset   = StringSubstr(ligne, sep + 1);

      string reel = resoudre_symbole(symbole);
      if(reel == "")
      {
         Print("EA_Collecteur: symbole introuvable chez le broker : ", symbole);
         continue;
      }
      int n = ArraySize(nouveaux_assets);
      ArrayResize(nouveaux_assets, n + 1);
      ArrayResize(nouvelles_symboles, n + 1);
      nouveaux_assets[n] = asset;
      nouvelles_symboles[n] = reel;
      SymbolSelect(reel, true);
   }

   int ancien = ArraySize(symboles);
   bool identique = (ancien == ArraySize(nouvelles_symboles));
   if(identique)
      for(int i = 0; i < ancien; i++)
         if(symboles[i] != nouvelles_symboles[i]) { identique = false; break; }
   if(!identique)
      Print("EA_Collecteur: ", ArraySize(nouvelles_symboles), " symbole(s) à collecter (avant : ", ancien, ")");

   ArrayCopy(symboles, nouvelles_symboles);
   ArrayCopy(assets, nouveaux_assets);
   ArrayResize(historique_fait, ArraySize(symboles));
   ArrayInitialize(historique_fait, false);

   ArrayResize(dernier_debut, ArraySize(symboles) * NB_TF);
   ArrayInitialize(dernier_debut, 0);
   ArrayResize(dernier_close, ArraySize(symboles) * NB_TF);
   ArrayInitialize(dernier_close, 0.0);
}

//+------------------------------------------------------------------+
//| « M1:10080,M5:2016,D1:2000 » → tableaux TF + profondeurs.        |
//+------------------------------------------------------------------+
void lire_tfs(const string entete)
{
   string morceaux[];
   StringSplit(entete, ',', morceaux);
   string noms[];
   ENUM_TIMEFRAMES periodes[];
   int profondeurs[];
   for(int i = 0; i < ArraySize(morceaux); i++)
   {
      string m = morceaux[i];
      int sep = StringFind(m, ":");
      if(sep <= 0) continue;
      string nom = StringSubstr(m, 0, sep);
      int prof = (int)StringToInteger(StringSubstr(m, sep + 1));
      ENUM_TIMEFRAMES p = periode_depuis_nom(nom);
      if(p == PERIOD_CURRENT) continue;
      int n = ArraySize(noms);
      ArrayResize(noms, n + 1);
      ArrayResize(periodes, n + 1);
      ArrayResize(profondeurs, n + 1);
      noms[n] = nom;
      periodes[n] = p;
      profondeurs[n] = prof;
   }
   ArrayCopy(tf_noms, noms);
   ArrayCopy(tf_periodes, periodes);
   ArrayCopy(tf_profondeurs, profondeurs);
   NB_TF = ArraySize(tf_noms);
}

//+------------------------------------------------------------------+
ENUM_TIMEFRAMES periode_depuis_nom(const string nom)
{
   if(nom == "M1")  return PERIOD_M1;
   if(nom == "M5")  return PERIOD_M5;
   if(nom == "M15") return PERIOD_M15;
   if(nom == "M30") return PERIOD_M30;
   if(nom == "H1")  return PERIOD_H1;
   if(nom == "H4")  return PERIOD_H4;
   if(nom == "D1")  return PERIOD_D1;
   if(nom == "W1")  return PERIOD_W1;
   return PERIOD_CURRENT;
}

//+------------------------------------------------------------------+
//| Résout le nom broker réel : exact, puis insensible à la casse.   |
//+------------------------------------------------------------------+
string resoudre_symbole(const string demande)
{
   if(SymbolSelect(demande, true) && SymbolInfoDouble(demande, SYMBOL_BID) != 0.0)
      return demande;
   int total = SymbolsTotal(false);
   for(int i = 0; i < total; i++)
   {
      string candidat = SymbolName(i, false);
      if(StringCompare(candidat, demande, false) == 0)
         return candidat;
   }
   return "";
}

//+------------------------------------------------------------------+
//| Historique d'un symbole : batch JSON par TF.                     |
//+------------------------------------------------------------------+
void pousser_historique(const int s)
{
   string asset = assets[s];
   for(int t = 0; t < NB_TF; t++)
   {
      int prof = tf_profondeurs[t];
      MqlRates barres[];
      int n = CopyRates(symboles[s], tf_periodes[t], 0, prof + 1, barres);
      if(n <= 1)
      {
         Print("EA_Collecteur: historique ", symboles[s], " ", tf_noms[t],
               " indisponible (", n, ") — réessai au prochain cycle");
         return;
      }
      string json = "{\"asset\":\"" + asset + "\",\"tf\":\"" + tf_noms[t] + "\",\"b\":[";
      for(int i = 0; i < n - 1; i++) // exclure la bougie en formation
      {
         if(i > 0) json += ",";
         json += StringFormat("[%I64d,%.10f,%.10f,%.10f,%.10f,%I64d]",
                              (long)barres[i].time, barres[i].open, barres[i].high,
                              barres[i].low, barres[i].close, (long)barres[i].tick_volume);
      }
      json += "]}";

      char donnees[];
      StringToCharArray(json, donnees, 0, StringLen(json));
      char resultat[];
      string en_tetes_reponse = "";
      int statut = WebRequest("POST", ApiUrl + "/api/mt5/historique",
                              "Content-Type: application/json\r\n",
                              30000, donnees, resultat, en_tetes_reponse);
      if(statut == 200)
      {
         dernier_debut[idx(s, t)] = 0;
      }
      else
      {
         Print("EA_Collecteur: historique ", asset, " ", tf_noms[t], " → HTTP ", statut);
      }
      Sleep(150);
   }
   historique_fait[s] = true;
   Print("EA_Collecteur: historique Axi de ", asset, " poussé (", NB_TF, " TF)");
}

//+------------------------------------------------------------------+
//| Bougies en formation + clôtures officielles, chaque (s, tf).     |
//+------------------------------------------------------------------+
void pousser_bougies()
{
   for(int s = 0; s < ArraySize(symboles); s++)
   {
      if(!historique_fait[s])
      {
         pousser_historique(s);
         continue;
      }
      for(int t = 0; t < NB_TF; t++)
      {
         string symbole = symboles[s];
         ENUM_TIMEFRAMES periode = tf_periodes[t];
         datetime t0 = iTime(symbole, periode, 0);
         if(t0 <= 0) continue;

         if(dernier_debut[idx(s, t)] > 0 && t0 > dernier_debut[idx(s, t)])
            envoyer_kline(s, t, 1, true);

         double c = iClose(symbole, periode, 0);
         if(c > 0 && (c != dernier_close[idx(s, t)] || t0 != dernier_debut[idx(s, t)]))
            envoyer_kline(s, t, 0, false);
      }
   }
}

//+------------------------------------------------------------------+
//| Envoie la bougie au shift donné pour (symbole s, tf t).          |
//+------------------------------------------------------------------+
void envoyer_kline(const int s, const int t, const int shift, const bool confirmee)
{
   string symbole = symboles[s];
   ENUM_TIMEFRAMES periode = tf_periodes[t];
   datetime debut = iTime(symbole, periode, shift);
   if(debut <= 0) return;

   string corps = StringFormat(
      "asset=%s&tf=%s&debut=%I64d&o=%.10f&h=%.10f&l=%.10f&c=%.10f&v=%I64d&conf=%d",
      assets[s], tf_noms[t], (long)debut,
      iOpen(symbole, periode, shift), iHigh(symbole, periode, shift),
      iLow(symbole, periode, shift), iClose(symbole, periode, shift),
      (long)iVolume(symbole, periode, shift), confirmee ? 1 : 0);

   char donnees[];
   StringToCharArray(corps, donnees, 0, StringLen(corps));

   char resultat[];
   string en_tetes_reponse = "";
   ResetLastError();
   int statut = WebRequest("POST", ApiUrl + "/api/mt5/kline",
                           "Content-Type: application/x-www-form-urlencoded\r\n",
                           5000, donnees, resultat, en_tetes_reponse);
   if(statut == -1)
      return;

   dernier_debut[idx(s, t)] = iTime(symbole, periode, 0);
   dernier_close[idx(s, t)] = iClose(symbole, periode, 0);
}

//+------------------------------------------------------------------+
void heartbeat()
{
   char vide[];
   char resultat[];
   string en_tetes_reponse = "";
   WebRequest("POST", ApiUrl + "/api/mt5/heartbeat",
              "Content-Type: application/x-www-form-urlencoded\r\n",
              5000, vide, resultat, en_tetes_reponse);
}
//+------------------------------------------------------------------+
