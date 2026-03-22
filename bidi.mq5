#property copyright "Copyright 2025 nonconnu"
#define BOT_VERSION "2.025"
#define BOT_NAME "Bidi"
#define BOT_SERIAL 1

#include <Trade\Trade.mqh>
#include <Indicators\Indicators.mqh>

#include <nonconnu\Event.mqh>
#include <nonconnu\AccountData.mqh>
#include <nonconnu\MarketHelper.mqh>
#include <nonconnu\Logger.mqh>
#include <nonconnu\PriceSqueeze.mqh>
#include <nonconnu\TradeInfo.mqh>
#include <nonconnu\Straddler.mqh>
#include <nonconnu\Utils.mqh>

static ulong MagicNumber;

input int Bot_Instance = 1; // Pour différencier les instances du bot sur une meme paire

input double RiskPercent = 2.0; // Risque total (%) (1/2 par jambe)
input double StopLoss = 0; // Valeur du SL en points (0=valeur paire)
input double TakeProfitRatio = 0; // Niveau TP par rapport au SL (0=TP ouvert, 2=2xSL, 3=3xSL)
input int ExpirationMinutes = 0; // Durée de vie max. en minutes (0=illimité, 300=5h 3000=50h)

input bool BreakEvenEnabled = false; // Activer le Break Even (false=Non, true=Oui)
input bool TrailingStopEnabled = false; // Activer le trailing stop (false=Non, true=Oui)
input double PartialTPLevel = 0; // Niveau TP pour fermeture partielle (0=inactif, 2=2xSL, 3=3xSL)
input double PartialProfitFraction = 0.5; // Fraction fermeture partielle (0.5=50%, 1=100%)

input string ATRTimeframe = "M5"; // Echelle de temps pour l'ATR (M1, M5, M15, M30)
input int ATRPeriod = 7;          // Période de l'ATR (>=5 et <=30)
input double ATRMultiplier = 2.0; // Trailing stop multiplicateur de l'ATR (Min:.5, Max5.0, 0=valeur paire)

input string EventTime = ""; // Heure événement HH:MM:SS (ex: 09:0:10), vide pour désactiver
input string EventDays = "Semaine"; // Jours (Lun,Mar,Mer,Jeu,Ven,Sam,Dim,Semaine,Tous)
input string EventMonths ="Tous"; // Mois (Jan,Fev,Mar,Avr,Mai,Juin,Juil,Aou,...,Tous,SansJuilAou)
input string EventWeeks = "Toutes"; // Semaines (1,2,3,4,5,6,Toutes)

input string LogLevel = "Info"; // Niveau de journalisation (Info, Warn, Debug)
input bool IsFileLogEnabled = false; // Journalisation dans un fichier (false=Non, true=Oui)

// Button parameters
string buttonCloseAll = "CloseAllButton";
string buttonStraddle = "StartStopButton";
int buttonX = 5;
int buttonY = 40;
int buttonWidth = 90;
int buttonHeight = 30;

// Instances
Straddler *straddler;
Event *event;

int OnInit()
{
    MagicNumber = BOT_SERIAL * 10 + Bot_Instance;
    EventSetTimer(1);

    Log.SetBotName(BOT_NAME);
    Log.EnableLogLevel(StringToLogFlag(LogLevel));
    if (IsFileLogEnabled) Log.EnableFileLog();
    
    Log.Info("OnInit", StringFormat("%s v%s : Account: %s, AccountType: %s, Symbole: %s, MagicNumber: %d",
                                    BOT_NAME, BOT_VERSION, AccountName, AccountType(), _Symbol, MagicNumber));

    Log.Info("OnInit", StringFormat("Effective: Risk=%.2f%%, SL=%.0f, TP=%.0f, BE=%s, partialTPLevel=%.1f, TrailingSL=%s, ATR(%s, Period=%d, Multiplier=%.1f) Expire=%dmin",
                                    RiskPercent, StopLoss, TakeProfitRatio, BreakEvenEnabled ? "Yes" : "No", PartialTPLevel,
                                    TrailingStopEnabled ? "Yes" : "No", ATRTimeframe, ATRPeriod, ATRMultiplier, ExpirationMinutes));

    Comment(StringFormat("V%s Risk=%.1f%% SL=%.0f TP=%.0f, BE=%s, partialTPLevel=%.1f, partialProfitFraction=%.1f, TrailingSL=%s, ATR(%s, Period=%d, Multiplier=%.1f) Exp=%dh",
                         BOT_VERSION, RiskPercent, StopLoss, TakeProfitRatio, BreakEvenEnabled ? "Yes" : "No", PartialTPLevel, PartialProfitFraction, 
                         TrailingStopEnabled ? "Yes" : "No", ATRTimeframe, ATRPeriod, ATRMultiplier, ExpirationMinutes / 60));

    PrintSymbolSpecifications();
    
    straddler = Straddler::Create(
        RiskPercent,
        StopLoss,
        TakeProfitRatio,
        BreakEvenEnabled,
        PartialTPLevel,
        PartialProfitFraction,
        TrailingStopEnabled,
        ATRTimeframe,
        ATRPeriod,
        ATRMultiplier,
        ExpirationMinutes
    );

    if (straddler == NULL ||!straddler.IsValid())
    {
        delete straddler;
        return INIT_FAILED;
    }
    straddler.PrintInfo();

    // Restore state from open positions after timeframe change or EA restart
    straddler.RestoreFromOpenPositions();

    CreateButton(buttonStraddle, "Trade", buttonX, buttonY, buttonWidth, buttonHeight);
    ObjectSetInteger(0, buttonStraddle, OBJPROP_BGCOLOR, clrGreen);

    if (EventTime != "") {       
        event = new Event(EventTime, EventDays, EventWeeks, EventMonths);
        if (!event.IsValid())
        {
            delete event;
            return INIT_FAILED;
        }
        event.PrintInfo();
    }
    return (INIT_SUCCEEDED);
}

void OnDeinit(const int reason)
{
    Log.Info("OnDeinit", "Fin du bot Bidi - Raison: " + IntegerToString(reason) + " (" + GetDeinitReasonText(reason) + ")");
    ObjectDelete(0, buttonCloseAll);
    ObjectDelete(0, buttonStraddle);
    ChartRedraw();

    delete event;
    delete straddler;
    EventKillTimer();
}

void OnTimer()
{  
    // straddler.PositionInfo("OnTimer");
    // MarketInfo();
    // SymbolPositionInfo();
    
    if (HasOpenPositions())
    {
        if (ObjectFind(0, buttonCloseAll) < 0)
        {
            ObjectDelete(0, buttonStraddle);
            CreateButton(buttonCloseAll, "Fermer", buttonX, buttonY, buttonWidth, buttonHeight);
            ObjectSetInteger(0, buttonCloseAll, OBJPROP_BGCOLOR, clrRed);
            ChartRedraw();
        }
    }
    else    
    {
        if (ObjectFind(0, buttonCloseAll) >= 0)
        {
            ObjectDelete(0, buttonCloseAll);
            CreateButton(buttonStraddle, "Trade", buttonX, buttonY, buttonWidth, buttonHeight);
            ObjectSetInteger(0, buttonStraddle, OBJPROP_BGCOLOR, clrGreen);
            ChartRedraw();
        }
    }

    if (straddler.IsStraddle() || straddler.IsOneLeg())
    {
        Log.Debug("OnTimer", "Fermeture des trades expirés");
        straddler.CloseExpiredTrades();
    }
    else
    {
        if (!HasOpenPositions() && event != NULL && event.IsReady())
        {
            if (!straddler.Straddle()) Log.Error("OnTimer", "Echec du straddle");
        }
    }
}

void OnTick()
{
    // straddler.PositionInfo("OnTick");

    if (straddler.IsOneLeg())
    {
        straddler.PartialTP();
        straddler.TrailingSL();
    }
}

void OnTradeTransaction(const MqlTradeTransaction &trans,
    const MqlTradeRequest &request,
    const MqlTradeResult &result)
{
    // Only process deal additions
    if (trans.type != TRADE_TRANSACTION_DEAL_ADD) return;

    ulong dealTicket = trans.deal;
    if (!HistoryDealSelect(dealTicket)) return;

    long dealPositionId = HistoryDealGetInteger(dealTicket, DEAL_POSITION_ID);
    ENUM_DEAL_ENTRY dealEntryType = (ENUM_DEAL_ENTRY)HistoryDealGetInteger(dealTicket, DEAL_ENTRY);
    ENUM_DEAL_REASON dealReason = (ENUM_DEAL_REASON)HistoryDealGetInteger(dealTicket, DEAL_REASON);
    double dealProfit = HistoryDealGetDouble(dealTicket, DEAL_PROFIT);
    string dealSymbol = HistoryDealGetString(dealTicket, DEAL_SYMBOL);
    double dealVolume = HistoryDealGetDouble(dealTicket, DEAL_VOLUME);
    double dealPrice = HistoryDealGetDouble(dealTicket, DEAL_PRICE);
    ENUM_DEAL_TYPE dealType = (ENUM_DEAL_TYPE)HistoryDealGetInteger(dealTicket, DEAL_TYPE);
 
    // Only process our deals
    if (dealSymbol != _Symbol)
    { 
        return;
    }

    if (dealEntryType == DEAL_ENTRY_IN)
    {
        Log.Debug("OnTradeTransaction", StringFormat("Deal #%d is an entry deal, ignoring", dealTicket));
        return;
    }

    string dealt = StringFormat("Profit: %.2f, Deal# %d, Entry: %s, Volume: %.2f, Price: %.5f, %s",
                                dealProfit, dealPositionId, EnumToString(dealEntryType),
                                dealVolume, dealPrice, EnumToString(dealReason));

    if (dealReason == DEAL_REASON_SL)
    {
        if (straddler.IsStraddle())
        {
            if (straddler.GetBuyTicket() == dealPositionId)
            {
                Log.Info("Stop Loss Leg BUY ", dealt);
                straddler.SetStraddleOnLegSell();
            }
            else if (straddler.GetSellTicket() == dealPositionId)
            {
                Log.Info("Stop Loss Leg SELL ", dealt);
                straddler.SetStraddleOnLegBuy();
            }
        }
        else if (straddler.IsOneLeg())
        {
            Log.Info("Stop Loss ", dealt);
            straddler.ResetStraddle();
        }
    }
    else if (dealReason == DEAL_REASON_TP)
    {
        if (straddler.IsStraddle())
        {
            Log.Error("Take Profit 1er jambe avant SL: Situation anormale (ajustement manuel ?) pour le straddle si SL < TP ", dealt);
            straddler.ResetStraddle();
        }
        else if (straddler.IsOneLeg())
        {
            Log.Info("Take Profit ", dealt);
            straddler.ResetStraddle();
        }
    }
    else if (dealReason == DEAL_REASON_EXPERT && straddler.IsOneLeg() && straddler.IsPartialProfit()) {
        Log.Info("Take Profit Partial Close", dealt);
    }
    else {
      string reasonText;
      switch (dealReason) {
      case DEAL_REASON_CLIENT:
        reasonText = "Client";
        break;
      case DEAL_REASON_MOBILE:
        reasonText = "Mobile";
        break;
      case DEAL_REASON_WEB:
        reasonText = "Web";
        break;
      case DEAL_REASON_SO:
        reasonText = "Stop Out";
        break;
      case DEAL_REASON_ROLLOVER:
        reasonText = "Rollover";
        break;
      default:
        reasonText = "Unknown";
        break;
      }
      string details = StringFormat(
          "#%d fermee: %d (%s), Profit %.2f, volume %.2f, Price %.5f",
          dealPositionId, dealReason, reasonText, dealProfit, dealVolume,
          dealPrice);
      Log.Warn("OnTradeTransaction", details);
      straddler.ResetStraddle();
    }
}

void OnChartEvent(const int id,
                  const long &lparam,
                  const double &dparam,
                  const string &sparam)
{
    if (id == CHARTEVENT_OBJECT_CLICK)
    {
        if (sparam == buttonStraddle)
        {
            if (!HasOpenPositions())
            {
                if (!straddler.Straddle())
                    Log.Error("OnTimer", "Echec du straddle");
            }
        }
        else if (sparam == buttonCloseAll)
        {
            ObjectSetInteger(0, buttonCloseAll, OBJPROP_STATE, false);
            ChartRedraw();
            if (straddler.CloseAllPositions())
            {
                Log.Info("OnChartEvent", "Fermeture manuelle de toutes les positions réussie");
            }
            else
            {
                Log.Error("OnChartEvent", "Echec de la fermeture manuelle de toutes les positions");
            }
        }
    }
}