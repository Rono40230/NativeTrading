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
#property version   "1.32"
#property strict

input string ApiUrl = "http://127.0.0.1:8080"; // URL de l'application (localhost refusé par MT5 — bug connu)

// Abonnements
string   symboles[];       // nom broker réel (résolu, casse exacte)
string   assets[];         // nom dans l'app
bool     historique_fait[];// historique poussé pour ce symbole ?
bool     rattrapage_fait[];// trous RÉCENTS rattrapés (v1.31 : nuit PC éteint)

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
//| Conversion heure SERVEUR broker → UTC.                          |
//| Axi : GMT+2 (heure US d'hiver) / GMT+3 (heure US d'été) — règle    |
//| approximative aux semaines frontières près (±1 h), exacte         |
//| aujourd'hui. Sans ça, toutes les bougies partent 2-3 h dans le    |
//| futur et les annonces UTC (straddle) se désynchronisent.          |
//+------------------------------------------------------------------+
bool heure_ete_us(const datetime t_srv)
{
   MqlDateTime d;
   TimeToStruct(t_srv, d);
   int m = d.mon, j = d.day;
   if(m > 3 && m < 11) return true;   // avril-octobre
   if(m < 3 || m > 11) return false;  // novembre-février
   if(m == 3) return j >= 8;          // ~2e dimanche de mars
   return j <= 7;                     // ~1er dimanche de novembre
}

datetime vers_utc(const datetime t_srv)
{
   int offset_heures = heure_ete_us(t_srv) ? 3 : 2;
   // Exact à l'instant présent si l'horloge courante est dans la même
   // saison (elle l'est quasi toujours) :
   int offset_vif = (int)(TimeTradeServer() - TimeGMT()) / 3600;
   if(offset_vif == 2 || offset_vif == 3)
      offset_heures = offset_vif;
   return t_srv - offset_heures * 3600;
}

//+------------------------------------------------------------------+
int idx(const int s, const int t) { return s * NB_TF + t; }

// Durée d'une bougie du TF t (secondes) — borne de recherche CopyRates.
int tf_secs(const int t)
{
   return PeriodSeconds(tf_periodes[t]);
}

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

   // Réinitialiser les suiveurs UNIQUEMENT si la liste a changé — sinon
   // l'historique serait repoussé en boucle à chaque cycle de 30 s
   // (vu en prod : complet à 12h36, repoussé à 12h38).
   if(!identique || ArraySize(symboles) != ArraySize(nouvelles_symboles))
   {
      ArrayCopy(symboles, nouvelles_symboles);
      ArrayCopy(assets, nouveaux_assets);
      ArrayResize(historique_fait, ArraySize(symboles));
      ArrayInitialize(historique_fait, false);
      ArrayResize(rattrapage_fait, ArraySize(symboles));
      ArrayInitialize(rattrapage_fait, false);

      ArrayResize(dernier_debut, ArraySize(symboles) * NB_TF);
      ArrayInitialize(dernier_debut, 0);
      ArrayResize(dernier_close, ArraySize(symboles) * NB_TF);
      ArrayInitialize(dernier_close, 0.0);
      ArrayResize(etat_tf, ArraySize(symboles) * NB_TF);
      ArrayInitialize(etat_tf, 0);
      ArrayResize(etat_rattrapage, ArraySize(symboles) * NB_TF);
      ArrayInitialize(etat_rattrapage, 0);
   }
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
//| Historique d'un symbole : batch JSON par TF, MORCEAUX de 2 000    |
//| bougies (~130 Ko par POST — les gros corps font planter WebRequest)|
//| et UN SEUL TF par tick d'horloge (1 s) pour étaler la charge.     |
//| État de progression dans etat_tf[] : 0 = à faire, 1 = fait.       |
//+------------------------------------------------------------------+
int etat_tf[]; // [symbole × tf] : historique poussé ?
int etat_rattrapage[]; // [symbole × tf] : trou récent rattrapé (v1.31)

void pousser_historique(const int s)
{
   string asset = assets[s];
   for(int t = 0; t < NB_TF; t++)
   {
      if(etat_tf[idx(s, t)] == 1) continue;

      int prof = tf_profondeurs[t];

      // ── Que manque-t-il déjà ? L'app connaît son historique : on lui
      // demande count + min_ts. Base pleine → TF sauté instantanément ;
      // sinon on ne pousse que les bougies PLUS ANCIENNES que son min.
      long min_ts_db = 0;
      int  count_db = 0;
      if(etat_historique(asset, tf_noms[t], count_db, min_ts_db) && count_db > 0)
      {
         datetime limite_utc = (datetime)min_ts_db;
         datetime limite_srv = vers_serveur(limite_utc);
         if(count_db >= prof)
         {
            etat_tf[idx(s, t)] = 1;
            dernier_debut[idx(s, t)] = 0;
            return; // déjà complet — TF sauté
         }
         MqlRates probe[];
         int np = CopyRates(symboles[s], tf_periodes[t], 0, 2, probe);
         if(np > 0 && probe[np - 1].time >= limite_srv)
         {
            etat_tf[idx(s, t)] = 1;
            dernier_debut[idx(s, t)] = 0;
            return; // la base remonte aussi loin que le broker — rien à ajouter
         }
      }

      MqlRates barres[];
      // CopyRates est borné par « Max bars in chart » du terminal : on
      // demande le max voulu, MT5 rend ce qu'il a.
      int n = CopyRates(symboles[s], tf_periodes[t], 0, prof + 1, barres);
      if(n <= 1)
      {
         Print("EA_Collecteur: historique ", symboles[s], " ", tf_noms[t],
               " indisponible (", n, ") — réessai au prochain cycle");
         return; // ce symbole réessayera
      }
      int TAILLE_MORCEAU = 2000;
      int envoyees = 0;
      int debut_envoi = n - 1; // plus ancienne bougie à envoyer
      if(count_db > 0)
      {
         datetime limite_srv = vers_serveur((datetime)min_ts_db);
         for(int i = 0; i < n; i++)
         {
            if(barres[i].time < limite_srv) { debut_envoi = i; break; }
            debut_envoi = i + 1; // toutes plus récentes que le min → rien
         }
         if(debut_envoi >= n)
         {
            etat_tf[idx(s, t)] = 1;
            dernier_debut[idx(s, t)] = 0;
            return; // rien de plus ancien à ajouter
         }
      }
      for(int fin = n - 1; fin > debut_envoi; fin -= TAILLE_MORCEAU)
      {
         int debut_m = MathMax(debut_envoi, fin - TAILLE_MORCEAU);
         string json = "{\"asset\":\"" + asset + "\",\"tf\":\"" + tf_noms[t] + "\",\"b\":[";
         for(int i = fin - 1; i >= debut_m; i--)
         {
            json += StringFormat("[%I64d,%.10f,%.10f,%.10f,%.10f,%I64d]",
                                 (long)vers_utc(barres[i].time), barres[i].open, barres[i].high,
                                 barres[i].low, barres[i].close, (long)barres[i].tick_volume);
            if(i > debut_m) json += ",";
         }
         json += "]}";

         char donnees[];
         StringToCharArray(json, donnees, 0, StringLen(json));
         int statut = -1;
         for(int essai = 0; essai < 3 && statut != 200; essai++)
         {
            char resultat[];
            string en_tetes_reponse = "";
            ResetLastError();
            statut = WebRequest("POST", ApiUrl + "/api/mt5/historique",
                                "Content-Type: application/json\r\n",
                                30000, donnees, resultat, en_tetes_reponse);
            if(statut != 200) Sleep(1000);
         }
         if(statut != 200)
         {
            Print("EA_Collecteur: historique ", asset, " ", tf_noms[t],
                  " morceau → HTTP ", statut, " err ", GetLastError(),
                  " — reprise au prochain cycle");
            return;
         }
         envoyees += fin - debut_m;
         Sleep(120);
      }
      etat_tf[idx(s, t)] = 1;
      dernier_debut[idx(s, t)] = 0;
      Print("EA_Collecteur: historique ", asset, " ", tf_noms[t],
            pousser_note(count_db, envoyees));
      return; // UN TF par tick : le suivant dans une seconde
   }
   historique_fait[s] = true;
   Print("EA_Collecteur: historique Axi de ", asset, " complet (", NB_TF, " TF)");
}

//+------------------------------------------------------------------+
//| RATTRAPAGE DES 48 DERNIÈRES HEURES (v1.32).                      |
//| v1.31 remplissait depuis max_ts (la FIN) — aveugle aux trous AU   |
//| MILIEU : ce matin-là, l'EA avait déjà collecté après le trou.     |
//| v1.32 : re-push IDempotent des 48h (INSERT OR IGNORE côté app) —  |
//| les trous guérissent quelle que soit leur position, marché fermé  |
//| → rien à pousser, doublons ignorés. UN TF par tick, une passe.    |
//+------------------------------------------------------------------+
void rattraper_trou(const int s)
{
   for(int t = 0; t < NB_TF; t++)
   {
      if(etat_rattrapage[idx(s, t)] == 1) continue;

      string asset = assets[s];
      etat_rattrapage[idx(s, t)] = 1;

      datetime debut_srv = TimeCurrent() - 48 * 3600;
      MqlRates barres[];
      int n = CopyRates(symboles[s], tf_periodes[t], debut_srv, TimeCurrent(), barres);
      if(n <= 0) continue; // rien chez le broker (marché fermé / pas encore téléchargé)

      int TAILLE_MORCEAU = 2000;
      int envoyees = 0;
      for(int fin = n; fin > 0; fin -= TAILLE_MORCEAU)
      {
         int debut_m = MathMax(0, fin - TAILLE_MORCEAU);
         string json = "{\"asset\":\"" + asset + "\",\"tf\":\"" + tf_noms[t] + "\",\"b\":[";
         for(int i = fin - 1; i >= debut_m; i--)
         {
            json += StringFormat("[%I64d,%.10f,%.10f,%.10f,%.10f,%I64d]",
                                 (long)vers_utc(barres[i].time), barres[i].open, barres[i].high,
                                 barres[i].low, barres[i].close, (long)barres[i].tick_volume);
            if(i > debut_m) json += ",";
         }
         json += "]}";

         char donnees[];
         StringToCharArray(json, donnees, 0, StringLen(json));
         int statut = -1;
         for(int essai = 0; essai < 3 && statut != 200; essai++)
         {
            char resultat[];
            string en_tetes_reponse = "";
            ResetLastError();
            statut = WebRequest("POST", ApiUrl + "/api/mt5/historique",
                                "Content-Type: application/json", 10000,
                                donnees, resultat, en_tetes_reponse);
            if(statut == -1) Sleep(500);
         }
         if(statut != 200)
         {
            Print("EA_Collecteur: rattrapage ", asset, " ", tf_noms[t],
                  " échec HTTP ", statut);
            return;
         }
         envoyees += MathMin(fin, TAILLE_MORCEAU) - debut_m;
      }
      if(envoyees > 0)
         Print("EA_Collecteur: rattrapage ", asset, " ", tf_noms[t],
               " — ", envoyees, " bougies comblées");
      return; // UN TF par tick
   }
   rattrapage_fait[s] = true; // tous les TF traités
}

//+------------------------------------------------------------------+
//| Bougies en formation + clôtures officielles, chaque (s, tf).     |
//+------------------------------------------------------------------+
void pousser_bougies()
{
   for(int s = 0; s < ArraySize(symboles); s++)
   {
      if(!historique_fait[s])
         pousser_historique(s); // UN TF par tick — puis le live continue
      if(historique_fait[s] && !rattrapage_fait[s])
         rattraper_trou(s); // v1.31 : combler les trous récents (une passe)
      for(int t = 0; t < NB_TF; t++)
      {
         if(!historique_fait[s] && etat_tf[idx(s, t)] != 1)
            continue; // TF pas encore poussé : pas de bougie à envoyer
         string symbole = symboles[s];
         ENUM_TIMEFRAMES periode = tf_periodes[t];
         datetime t0 = vers_utc(iTime(symbole, periode, 0));
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
   datetime debut = vers_utc(iTime(symbole, periode, shift));
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

   dernier_debut[idx(s, t)] = vers_utc(iTime(symbole, periode, 0));
   dernier_close[idx(s, t)] = iClose(symbole, periode, 0);
}

//+------------------------------------------------------------------+
//| Demande à l'app ce qu'elle a déjà : GET /api/mt5/historique/etat |
//| → { "count": N, "min_ts": T } (tiny JSON parsé à la main).       |
//+------------------------------------------------------------------+
bool etat_historique(const string asset, const string tf, int &count_db, long &min_ts_db)
{
   long max_ignored = 0;
   return etat_historique_ex(asset, tf, count_db, min_ts_db, max_ignored);
}

/// Variante v1.31 : expose AUSSI max_ts (dernière bougie en base) pour le
/// rattrapage des trous récents. Champ absent (ancien backend) → max_ts = 0.
bool etat_historique_ex(const string asset, const string tf,
                        int &count_db, long &min_ts_db, long &max_ts_db)
{
   char vide[];
   char resultat[];
   string en_tetes_reponse = "";
   string url = ApiUrl + "/api/mt5/historique/etat?asset=" + asset + "&tf=" + tf;
   if(WebRequest("GET", url, "", 5000, vide, resultat, en_tetes_reponse) != 200)
      return false;
   string reponse = "";
   for(int i = 0; i < ArraySize(resultat); i++)
      reponse += CharToString(resultat[i]);
   int pc = StringFind(reponse, "\"count\":");
   int pm = StringFind(reponse, "\"min_ts\":");
   if(pc < 0 || pm < 0) return false;
   int fc = StringFind(reponse, ",", pc);
   int fm = StringFind(reponse, "}", pm);
   if(fc < 0 || fm < 0) return false;
   count_db = (int)StringToInteger(StringSubstr(reponse, pc + 8, fc - pc - 8));
   min_ts_db = StringToInteger(StringSubstr(reponse, pm + 9, fm - pm - 9));
   max_ts_db = 0;
   int px = StringFind(reponse, "\"max_ts\":");
   if(px >= 0)
   {
      int fx = StringFind(reponse, "}", px);
      if(fx > px)
         max_ts_db = StringToInteger(StringSubstr(reponse, px + 9, fx - px - 9));
   }
   return true;
}

/// Inverse de vers_utc (pour comparer au temps serveur de CopyRates).
datetime vers_serveur(const datetime t_utc)
{
   int offset_heures = heure_ete_us(t_utc) ? 3 : 2;
   int offset_vif = (int)(TimeTradeServer() - TimeGMT()) / 3600;
   if(offset_vif == 2 || offset_vif == 3)
      offset_heures = offset_vif;
   return t_utc + offset_heures * 3600;
}

string pousser_note(const int count_db, const int envoyees)
{
   if(count_db > 0 && envoyees == 0) return " déjà complet — rien à ajouter";
   if(count_db > 0) return " delta poussé — " + IntegerToString(envoyees) + " bougies ajoutées";
   return " poussé — " + IntegerToString(envoyees) + " bougies";
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
