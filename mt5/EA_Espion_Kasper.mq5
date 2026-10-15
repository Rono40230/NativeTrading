//+------------------------------------------------------------------+
//| EA_Espion_Kasper.mq5 — Native Trading AI                         |
//|                                                                  |
//| Espion de « Tendance Kasper Bootcamp » (étude 15/10).            |
//| LECTURE SEULE : aucun ordre, aucun objet créé/modifié/supprimé.  |
//|                                                                  |
//| IMPORTANT — l'outil Kasper est un EA (1 seul EA par graphique) : |
//| l'espion se pose sur un AUTRE graphique, idéalement XAU M1       |
//| (n'importe quel TF convient), et lit les objets de TOUS les      |
//| graphiques ouverts via leur identifiant (ChartFirst/ChartNext +  |
//| ObjectsTotal(chart_id, …)) — les objets appartiennent au         |
//| graphique, pas à l'EA qui les dessine. Le tableau Kasper sera    |
//| repéré dans l'analyse par son graphique porteur (le journal      |
//| embarque la liste des graphiques : id, symbole, TF).             |
//|                                                                  |
//| MÉCANIQUE :                                                      |
//|  - À chaque clôture M1 : snapshot COMPLET des objets de tous les |
//    graphiques (nom, type, couleur, texte, coin, x/y) + sondes     |
//    EMA 9/21 des 7 TF (M1..D1) du symbole porteur de l'espion :    |
//    valeur VIVANTE (bougie en cours) et CLÔTURÉE (dernière finie). |
//|  - Sinon toutes les PeriodeSec : DIFF seules (objets créés,      |
//    modifiés, supprimés, tous graphiques) + sondes si changées.    |
//|  - Journal en FICHIER (l'endpoint /api/mt5/espion a été purgé) : |
//|    MQL5/Files/espion_kasper_journal.jsonl, une ligne JSON par    |
//|    événement, rotation à 20 Mo.                                  |
//|                                                                  |
//| POURQUOI PAS iCustom : les outils Kasper vivent dans             |
//| Experts/Advisors/ — iCustom ne charge que depuis Indicators/     |
//| (cause de l'échec erreur_handle de l'ancien Espion_OB_IA) et ne  |
//| sait de toute façon pas charger un EA. Les OBJETS se lisent      |
//| partout.                                                         |
//|                                                                  |
//| INSTALLATION :                                                   |
//|  1. Compiler dans MetaEditor (F7) — fichier déjà placé dans      |
//|     MQL5/Experts/Advisors/.                                      |
//|  2. Attacher sur un graphique XAU SANS la Tendance Kasper        |
//|     (elle reste sur son graphique actuel — ne rien changer).     |
//|  3. Le journal s'accumule tout seul — l'agent le lit sur disque. |
//+------------------------------------------------------------------+
#property copyright "Native Trading AI"
#property version   "1.02"
#property strict

input int    PeriodeSec = 5;                              // période des diffs (secondes)
input string Journal    = "espion_kasper_journal_u.jsonl"; // sous MQL5/Files/ — UTF-16 (v1.02 : l'ANSI transformait les flèches d'état en '?')

// Les 7 TF suivis par le tableau Kasper (captures owner 15/10).
const ENUM_TIMEFRAMES TFS[7]    = {PERIOD_M1, PERIOD_M5, PERIOD_M15, PERIOD_M30,
                                   PERIOD_H1, PERIOD_H4, PERIOD_D1};
const string          TFNOMS[7] = {"M1","M5","M15","M30","H1","H4","D1"};

int      g_ema9[7];    // handles iMA EMA 9  par TF (symbole de l'espion)
int      g_ema21[7];   // handles iMA EMA 21 par TF
int      g_seq         = 0;
datetime g_barre_vue   = 0;
string   g_cles[];     // état précédent : clés "chartId|nom" des objets
string   g_sers[];     // état précédent : forme sérialisée
string   g_ema_pre     = "";

string TfNom(ENUM_TIMEFRAMES tf)
{
   switch(tf)
   {
      case PERIOD_M1:  return "M1";
      case PERIOD_M5:  return "M5";
      case PERIOD_M15: return "M15";
      case PERIOD_M30: return "M30";
      case PERIOD_H1:  return "H1";
      case PERIOD_H4:  return "H4";
      case PERIOD_D1:  return "D1";
      default:         return "TF" + IntegerToString((int)tf);
   }
}

string JsonEchapper(string v)
{
   StringReplace(v, "\\", "\\\\");
   StringReplace(v, "\"", "\\\"");
   StringReplace(v, "\n", " ");
   StringReplace(v, "\r", " ");
   StringReplace(v, "\t", " ");
   return v;
}

/// Forme sérialisée d'un objet — tout ce qui bouge doit changer la chaîne.
string SerObjet(const long id, const string nom)
{
   return IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_TYPE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_COLOR))
      + "|" + ObjectGetString(id, nom, OBJPROP_TEXT)
      + "|" + ObjectGetString(id, nom, OBJPROP_TOOLTIP)
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_CORNER))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XDISTANCE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YDISTANCE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XSIZE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YSIZE))
      + "|" + StringFormat("%I64d:%.5f", (long)(datetime)ObjectGetInteger(id, nom, OBJPROP_TIME, 0),
                                         ObjectGetDouble(id, nom, OBJPROP_PRICE, 0));
}

/// Un objet en JSON (propriétés relues, jamais réutilisées du diff).
string JsonObjet(const long id, const string nom)
{
   return "{\"chart\":" + IntegerToString((int)id)
      + ",\"nom\":\"" + JsonEchapper(nom) + "\",\"type\":"
      + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_TYPE))
      + ",\"couleur\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_COLOR))
      + ",\"texte\":\"" + JsonEchapper(ObjectGetString(id, nom, OBJPROP_TEXT)) + "\""
      + ",\"tooltip\":\"" + JsonEchapper(ObjectGetString(id, nom, OBJPROP_TOOLTIP)) + "\""
      + ",\"coin\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_CORNER))
      + ",\"x\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XDISTANCE))
      + ",\"y\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YDISTANCE))
      + ",\"l\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XSIZE))
      + ",\"h\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YSIZE))
      + StringFormat(",\"ts0\":%I64d,\"p0\":%.5f}",
                     (long)(datetime)ObjectGetInteger(id, nom, OBJPROP_TIME, 0),
                     ObjectGetDouble(id, nom, OBJPROP_PRICE, 0));
}

/// Cartographie des graphiques ouverts (pour identifier le porteur Kasper).
string JsonGraphiques()
{
   string j = "";
   for(long id = ChartFirst(); id >= 0; id = ChartNext(id))
   {
      if(j != "") j += ",";
      j += StringFormat("{\"id\":%d,\"symbole\":\"%s\",\"tf\":\"%s\"}",
                        (int)id, ChartSymbol(id), TfNom(ChartPeriod(id)));
   }
   return j;
}

/// Sondes EMA 9/21 des 7 TF : vivante (bougie 0) + clôturée (bougie 1).
string SondesEma()
{
   string j = "";
   for(int t = 0; t < 7; t++)
   {
      double a9[], a21[];
      bool ok9  = (g_ema9[t]  != INVALID_HANDLE && CopyBuffer(g_ema9[t],  0, 0, 2, a9)  == 2);
      bool ok21 = (g_ema21[t] != INVALID_HANDLE && CopyBuffer(g_ema21[t], 0, 0, 2, a21) == 2);
      if(!ok9 && !ok21) continue;
      if(j != "") j += ",";
      j += "\"" + TFNOMS[t] + "\":{";
      j += ok9  ? StringFormat("\"e9v\":%.5f,\"e9c\":%.5f", a9[0],  a9[1])  : "\"e9v\":null,\"e9c\":null";
      j += ",";
      j += ok21 ? StringFormat("\"e21v\":%.5f,\"e21c\":%.5f", a21[0], a21[1]) : "\"e21v\":null,\"e21c\":null";
      j += "}";
   }
   return j;
}

/// Écrit une ligne JSON dans le journal (rotation à 20 Mo).
void Journaliser(const string json)
{
   int mode = FILE_READ|FILE_WRITE|FILE_TXT|FILE_UNICODE|FILE_SHARE_READ|FILE_SHARE_WRITE;
   int f = FileOpen(Journal, mode);
   if(f == INVALID_HANDLE) { Print("EA_Espion_Kasper: journal illisible err ", GetLastError()); return; }
   FileSeek(f, 0, SEEK_END);
   if(FileTell(f) > 20000000)
   {
      FileClose(f);
      string archive = "espion_kasper_" + TimeToString(TimeCurrent(), TIME_DATE) + ".jsonl";
      StringReplace(archive, "2026.10.15", "2026_10_15"); // les points de TimeToString
      FileMove(Journal, 0, archive, FILE_REWRITE);
      Print("EA_Espion_Kasper: rotation → ", archive);
      f = FileOpen(Journal, mode);
      if(f == INVALID_HANDLE) return;
   }
   FileWriteString(f, json + "\n");
   FileFlush(f);
   FileClose(f);
}

int Chercher(const string cle)
{
   for(int i = 0; i < ArraySize(g_cles); i++) if(g_cles[i] == cle) return i;
   return -1;
}

void RemplacerEtat(const string &cles[], const string &sers[])
{
   ArrayResize(g_cles, ArraySize(cles));
   ArrayResize(g_sers, ArraySize(sers));
   for(int i = 0; i < ArraySize(g_cles); i++) { g_cles[i] = cles[i]; g_sers[i] = sers[i]; }
}

/// Un cycle d'observation, sur TOUS les graphiques ouverts.
/// plein=true → tous les objets ; sinon le diff (clé = chartId|nom).
void Cycle(const bool plein)
{
   g_seq++;
   string cles_act[]; string sers_act[];

   string objets_json = "";
   int    emis = 0, suppr = 0;
   for(long id = ChartFirst(); id >= 0; id = ChartNext(id))
   {
      int total = ObjectsTotal(id, -1, -1);
      for(int i = 0; i < total; i++)
      {
         string nom = ObjectName(id, i, -1, -1);
         if(nom == "") continue;
         string cle = IntegerToString((int)id) + "|" + nom;
         string ser = SerObjet(id, nom);
         int n = ArraySize(cles_act);
         ArrayResize(cles_act, n + 1); ArrayResize(sers_act, n + 1);
         cles_act[n] = cle; sers_act[n] = ser;

         int avant = Chercher(cle);
         if(plein || avant < 0 || g_sers[avant] != ser)
         {
            if(emis + suppr > 0) objets_json += ",";
            objets_json += JsonObjet(id, nom);
            emis++;
         }
      }
   }
   if(!plein)
      for(int i = 0; i < ArraySize(g_cles); i++)
      {
         bool encore = false;
         for(int k = 0; k < ArraySize(cles_act); k++) if(cles_act[k] == g_cles[i]) { encore = true; break; }
         if(!encore)
         {
            if(emis + suppr > 0) objets_json += ",";
            objets_json += "{\"chart\":" + StringSubstr(g_cles[i], 0, StringFind(g_cles[i], "|"))
                         + ",\"nom\":\"" + JsonEchapper(StringSubstr(g_cles[i], StringFind(g_cles[i], "|") + 1))
                         + "\",\"supprime\":true}";
            suppr++;
         }
      }

   string ema = SondesEma();
   bool ema_changee = (ema != g_ema_pre);

   // Rien de neuf en diff → silence (le silence est une information).
   if(!plein && emis == 0 && suppr == 0 && !ema_changee)
      { RemplacerEtat(cles_act, sers_act); return; }

   string json = StringFormat(
      "{\"ts\":%I64d,\"heure\":\"%s\",\"symbole\":\"%s\",\"tf\":\"%s\",\"seq\":%d,\"plein\":%s",
      (long)TimeCurrent(), TimeToString(TimeCurrent(), TIME_DATE|TIME_MINUTES|TIME_SECONDS),
      _Symbol, TfNom(_Period), g_seq, plein ? "true" : "false");
   if(plein) json += ",\"graphiques\":[" + JsonGraphiques() + "]";
   json += ",\"objets\":[" + objets_json + "]";
   json += ",\"ema\":{" + ema + "}}";

   Journaliser(json);
   g_ema_pre = ema;
   RemplacerEtat(cles_act, sers_act);
}

int OnInit()
{
   for(int t = 0; t < 7; t++)
   {
      g_ema9[t]  = iMA(_Symbol, TFS[t], 9,  0, MODE_EMA, PRICE_CLOSE);
      g_ema21[t] = iMA(_Symbol, TFS[t], 21, 0, MODE_EMA, PRICE_CLOSE);
   }
   g_barre_vue = 0;
   g_ema_pre   = "";
   EventSetTimer(1); // premier cycle rapide puis bascule sur PeriodeSec
   Print("EA_Espion_Kasper v1.01 posé sur ", _Symbol, " ", TfNom(_Period),
         " — espionne les objets de TOUS les graphiques, journal MQL5/Files/", Journal);
   return INIT_SUCCEEDED;
}

void OnTimer()
{
   if(g_seq == 0)
   {
      Cycle(true);
      EventSetTimer(PeriodeSec);
      return;
   }
   // Clôture M1 → snapshot complet (état propre par minute).
   datetime b = iTime(_Symbol, PERIOD_M1, 0);
   if(b != g_barre_vue) { g_barre_vue = b; Cycle(true); return; }
   Cycle(false);
}

void OnDeinit(const int raison)
{
   EventKillTimer();
   for(int t = 0; t < 7; t++)
   {
      if(g_ema9[t]  != INVALID_HANDLE) IndicatorRelease(g_ema9[t]);
      if(g_ema21[t] != INVALID_HANDLE) IndicatorRelease(g_ema21[t]);
   }
   Journaliser(StringFormat("{\"ts\":%I64d,\"fin\":true,\"seq\":%d}", (long)TimeCurrent(), g_seq));
}
//+------------------------------------------------------------------+
