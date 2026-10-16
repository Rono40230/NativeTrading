//+------------------------------------------------------------------+
//| EA_Espion_OBIA.mq5 — Native Trading AI                           |
//|                                                                  |
//| Espion de « OrderBlock IA v6 » (étude 16/10) — LECTURE SEULE :   |
//| aucun ordre, aucun objet créé/modifié/supprimé.                  |
//|                                                                  |
//| L'OBIA est un EA (1 seul EA par graphique) : l'espion vit sur un |
//| AUTRE graphique et lit les objets de TOUS les graphiques ouverts |
//| via leur identifiant (ChartFirst/ChartNext + ObjectsTotal(id)).  |
//| Les objets de l'OBIA sont des RECTANGLES ancrés temps × prix —   |
//| contrairement à l'espion Tendance, on capture ici LES DEUX       |
//| ancres + remplissage + arrière-plan + épaisseur/style, matière   |
//| première de la reproduction exacte des zones FVG/OB.             |
//|                                                                  |
//| MÉCANIQUE :                                                      |
//|  - À chaque clôture M1 : snapshot COMPLET des objets de tous les |
//|    graphiques ; sinon toutes les PeriodeSec : DIFF seules        |
//|    (créés/modifiés/supprimés) — la vie des zones : naissance,    |
//|    redimensionnement, invalidation.                              |
//|  - Journal FICHIER UTF-16 : MQL5/Files/espion_obia_journal.jsonl |
//|    (une ligne JSON par événement, rotation à 20 Mo).             |
//|                                                                  |
//| INSTALLATION :                                                   |
//|  1. Compiler dans MetaEditor (F7) — déjà déposé dans Advisors/.  |
//|  2. Attacher OBIA v6 sur un graphique XAU (le TF à étudier, ex.  |
//|     M15) — voir consignes anti-trades de l'agent.                |
//|  3. Attacher CET espion sur un AUTRE graphique XAU. Laisser      |
//|     courir : les zones vivent ≤ 50 barres (12h30 en M15).        |
//+------------------------------------------------------------------+
#property copyright "Native Trading AI"
#property version   "1.00"
#property strict

input int    PeriodeSec = 5;                            // période des diffs (secondes)
input string Journal    = "espion_obia_journal.jsonl";  // sous MQL5/Files/

int      g_seq       = 0;
datetime g_barre_vue = 0;
string   g_cles[];     // état précédent : clés "chartId|nom"
string   g_sers[];     // état précédent : forme sérialisée

string JsonEchapper(string v)
{
   StringReplace(v, "\\", "\\\\");
   StringReplace(v, "\"", "\\\"");
   StringReplace(v, "\n", " ");
   StringReplace(v, "\r", " ");
   StringReplace(v, "\t", " ");
   return v;
}

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

/// Ancres temps × prix d'un objet (0..3) — les rectangles portent leurs
/// bornes exactes dans les ancres 0 et 1.
string SerAncres(const long id, const string nom)
{
   string s = "";
   for(int k = 0; k < 4; k++)
   {
      datetime t = (datetime)ObjectGetInteger(id, nom, OBJPROP_TIME, k);
      double   p = ObjectGetDouble(id, nom, OBJPROP_PRICE, k);
      if(t > 0) s += StringFormat("%I64d:%.5f;", (long)t, p);
   }
   return s;
}

string JsonAncres(const long id, const string nom)
{
   string j = "";
   for(int k = 0; k < 4; k++)
   {
      datetime t = (datetime)ObjectGetInteger(id, nom, OBJPROP_TIME, k);
      double   p = ObjectGetDouble(id, nom, OBJPROP_PRICE, k);
      if(t > 0)
      {
         if(j != "") j += ",";
         j += StringFormat("{\"ts\":%I64d,\"prix\":%.5f}", (long)t, p);
      }
   }
   return j;
}

/// Forme sérialisée — TOUT ce qui bouge doit changer la chaîne (ancres,
/// couleur, remplissage, épaisseur, style, textes, position écran).
string SerObjet(const long id, const string nom)
{
   return IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_TYPE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_COLOR))
      + "|" + SerAncres(id, nom)
      + "|" + ObjectGetString(id, nom, OBJPROP_TEXT)
      + "|" + ObjectGetString(id, nom, OBJPROP_TOOLTIP)
      + "|" + ObjectGetString(id, nom, OBJPROP_FONT)
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_FONTSIZE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_WIDTH))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_STYLE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_FILL))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_BACK))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_CORNER))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XDISTANCE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YDISTANCE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XSIZE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YSIZE))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_RAY_RIGHT))
      + "|" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_HIDDEN));
}

string JsonObjet(const long id, const string nom)
{
   string j = "{\"chart\":" + IntegerToString((int)id)
      + ",\"nom\":\"" + JsonEchapper(nom) + "\",\"type\":"
      + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_TYPE))
      + ",\"couleur\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_COLOR))
      + ",\"ancres\":[" + JsonAncres(id, nom) + "]"
      + ",\"texte\":\"" + JsonEchapper(ObjectGetString(id, nom, OBJPROP_TEXT)) + "\""
      + ",\"tooltip\":\"" + JsonEchapper(ObjectGetString(id, nom, OBJPROP_TOOLTIP)) + "\""
      + ",\"police\":\"" + JsonEchapper(ObjectGetString(id, nom, OBJPROP_FONT)) + "\""
      + ",\"taille_police\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_FONTSIZE))
      + ",\"epaisseur\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_WIDTH))
      + ",\"style\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_STYLE))
      + ",\"rempli\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_FILL))
      + ",\"arriere_plan\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_BACK))
      + ",\"coin\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_CORNER))
      + ",\"x\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XDISTANCE))
      + ",\"y\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YDISTANCE))
      + ",\"l\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_XSIZE))
      + ",\"h\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_YSIZE))
      + ",\"rayon_droite\":" + IntegerToString((int)ObjectGetInteger(id, nom, OBJPROP_RAY_RIGHT))
      + "}";
   return j;
}

/// Cartographie des graphiques ouverts (identifier le porteur OBIA).
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

void Journaliser(const string json)
{
   int mode = FILE_READ|FILE_WRITE|FILE_TXT|FILE_UNICODE|FILE_SHARE_READ|FILE_SHARE_WRITE;
   int f = FileOpen(Journal, mode);
   if(f == INVALID_HANDLE) { Print("EA_Espion_OBIA: journal illisible err ", GetLastError()); return; }
   FileSeek(f, 0, SEEK_END);
   if(FileTell(f) > 20000000)
   {
      FileClose(f);
      string archive = "espion_obia_" + TimeToString(TimeCurrent(), TIME_DATE) + ".jsonl";
      StringReplace(archive, ".", "_");
      StringReplace(archive, "_jsonl", ".jsonl");
      FileMove(Journal, 0, archive, FILE_REWRITE);
      Print("EA_Espion_OBIA: rotation → ", archive);
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

/// Un cycle d'observation sur TOUS les graphiques ouverts.
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

   if(!plein && emis == 0 && suppr == 0)
      { RemplacerEtat(cles_act, sers_act); return; }

   string json = StringFormat(
      "{\"ts\":%I64d,\"heure\":\"%s\",\"symbole\":\"%s\",\"tf\":\"%s\",\"seq\":%d,\"plein\":%s",
      (long)TimeCurrent(), TimeToString(TimeCurrent(), TIME_DATE|TIME_MINUTES|TIME_SECONDS),
      _Symbol, TfNom(_Period), g_seq, plein ? "true" : "false");
   if(plein) json += ",\"graphiques\":[" + JsonGraphiques() + "]";
   json += ",\"objets\":[" + objets_json + "]}";

   Journaliser(json);
   RemplacerEtat(cles_act, sers_act);
}

int OnInit()
{
   g_barre_vue = 0;
   EventSetTimer(1);
   Print("EA_Espion_OBIA v1.00 posé sur ", _Symbol, " ", TfNom(_Period),
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
   datetime b = iTime(_Symbol, PERIOD_M1, 0);
   if(b != g_barre_vue) { g_barre_vue = b; Cycle(true); return; }
   Cycle(false);
}

void OnDeinit(const int raison)
{
   EventKillTimer();
   Journaliser(StringFormat("{\"ts\":%I64d,\"fin\":true,\"seq\":%d}", (long)TimeCurrent(), g_seq));
}
//+------------------------------------------------------------------+
