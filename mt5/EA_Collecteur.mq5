//+------------------------------------------------------------------+
//| EA_Collecteur.mq5 — Native Trading AI                            |
//|                                                                  |
//| Collecteur MT5/Axi : pousse les bougies M1 du broker vers        |
//| l'application (même chemin que Bybit).                           |
//|                                                                  |
//| INSTALLATION :                                                   |
//|  1. Compiler ce fichier dans MetaEditor (F7).                    |
//|  2. MT5 → Outils → Options → Conseillers Experts →               |
//|     « Autoriser WebRequest pour les URL listées » → ajouter :    |
//|     http://localhost:8080                                        |
//|  3. Attacher l'EA à N'IMPORTE QUEL graphique (il collecte tous   |
//|     les symboles configurés dans l'application).                 |
//|                                                                  |
//| MÉCANIQUE :                                                      |
//|  - Toutes les 30 s : demande à l'app la liste des symboles à     |
//|    collecter ( cases cochées dans 📦 Données) + heartbeat.       |
//|  - Chaque seconde : pour chaque symbole, envoie la bougie M1 en  |
//|    formation ; au changement de minute, envoie la clôture        |
//|    officielle (conf=1) écrite en base par l'app.                 |
//+------------------------------------------------------------------+
#property copyright "Native Trading AI"
#property version   "1.00"
#property strict

input string ApiUrl = "http://localhost:8080"; // URL de l'application

// Abonnement : symbole broker ↔ nom actif dans l'app
string   symboles[];      // ex: "dax40.fs"
string   assets[];        // ex: "DAX"
datetime derniere_minute[];  // dernier début de M1 vu par symbole
double   dernier_prix[];     // dernier close envoyé (anti-spam)
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
   bool trentieme = (ticks_timer % 30 == 0);
   if(trentieme)
   {
      heartbeat();
      rafraichir_abonnements();
   }
   pousser_bougies();
}

//+------------------------------------------------------------------+
//| Liste des symboles : GET /api/mt5/symboles                       |
//| Réponse texte : « symbole|asset » par ligne.                     |
//+------------------------------------------------------------------+
void rafraichir_abonnements()
{
   char vide[];
   char resultat[];
   string reponse = "";
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
   for(int i = 0; i < ArraySize(resultat); i++)
      reponse += CharToString(resultat[i]);

   // Reconstruction de la liste (symbole|asset par ligne).
   string nouvelles_symboles[];
   string nouveaux_assets[];
   int nb = StringSplit(reponse, '\n', nouvelles_symboles);
   for(int i = 0; i < nb; i++)
   {
      string ligne = nouvelles_symboles[i];
      StringTrimLeft(ligne);
      StringTrimRight(ligne);
      if(StringLen(ligne) == 0) continue;
      int sep = StringFind(ligne, "|");
      if(sep <= 0) continue;
      string symbole = StringSubstr(ligne, 0, sep);
      string asset   = StringSubstr(ligne, sep + 1);
      int n = ArraySize(nouveaux_assets);
      ArrayResize(nouveaux_assets, n + 1);
      ArrayResize(nouvelles_symboles, n + 1);
      nouveaux_assets[n] = asset;
      nouvelles_symboles[n] = symbole;
      // Abonnement Market Watch (nécessaire pour iTime etc.).
      SymbolSelect(symbole, true);
   }

   // Diff : log des ajouts/retraits.
   int ancien = ArraySize(symboles);
   if(ancien != ArraySize(nouvelles_symboles))
      Print("EA_Collecteur: ", ArraySize(nouvelles_symboles), " symbole(s) à collecter (avant : ", ancien, ")");

   ArrayCopy(symboles, nouvelles_symboles);
   ArrayCopy(assets, nouveaux_assets);
   ArrayResize(derniere_minute, ArraySize(symboles));
   ArrayInitialize(derniere_minute, 0);
   ArrayResize(dernier_prix, ArraySize(symboles));
   ArrayInitialize(dernier_prix, 0.0);
}

//+------------------------------------------------------------------+
//| Bougies M1 : formation (conf=0) et clôture officielle (conf=1)   |
//+------------------------------------------------------------------+
void pousser_bougies()
{
   for(int i = 0; i < ArraySize(symboles); i++)
   {
      string symbole = symboles[i];
      datetime t0 = iTime(symbole, PERIOD_M1, 0);
      if(t0 <= 0) continue; // symbole sans données (marché fermé ?)

      // Changement de minute → envoyer la clôture officielle de la M1
      // précédente, puis le snapshot de la nouvelle.
      if(derniere_minute[i] > 0 && t0 > derniere_minute[i])
      {
         envoyer_kline(i, 1, true);
      }
      double close_courant = iClose(symbole, PERIOD_M1, 0);
      if(close_courant > 0 && (close_courant != dernier_prix[i] || t0 != derniere_minute[i]))
      {
         envoyer_kline(i, 0, false);
      }
   }
}

//+------------------------------------------------------------------+
//| Envoie la bougie M1 au shift donné.                              |
//+------------------------------------------------------------------+
void envoyer_kline(const int indice, const int shift, const bool confirmee)
{
   string symbole = symboles[indice];
   string asset   = assets[indice];
   datetime debut = iTime(symbole, PERIOD_M1, shift);
   if(debut <= 0) return;

   double o = iOpen(symbole, PERIOD_M1, shift);
   double h = iHigh(symbole, PERIOD_M1, shift);
   double b = iLow(symbole, PERIOD_M1, shift);
   double c = iClose(symbole, PERIOD_M1, shift);
   long   v = iTickVolume(symbole, PERIOD_M1, shift);

   string corps = StringFormat(
      "asset=%s&debut=%I64d&o=%.10f&h=%.10f&l=%.10f&c=%.10f&v=%I64d&conf=%d",
      asset, (long)debut, o, h, b, c, v, confirmee ? 1 : 0);

   char donnees[];
   StringToCharArray(corps, donnees, 0, StringLen(corps));

   char resultat[];
   string en_tetes_reponse = "";
   ResetLastError();
   int statut = WebRequest("POST", ApiUrl + "/api/mt5/kline",
                           "Content-Type: application/x-www-form-urlencoded\r\n",
                           5000, donnees, resultat, en_tetes_reponse);
   if(statut == -1)
   {
      return; // erreur réseau : la prochaine seconde réessaiera
   }

   derniere_minute[indice] = iTime(symbole, PERIOD_M1, 0);
   dernier_prix[indice] = iClose(symbole, PERIOD_M1, 0);
}

//+------------------------------------------------------------------+
//| Heartbeat : POST /api/mt5/heartbeat                              |
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
