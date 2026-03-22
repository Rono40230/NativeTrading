#property copyright "Copyright 2026 nonconnu"
#define BOT_VERSION "1.002"
#define BOT_NAME "Uni"
#define BOT_SERIAL 2

#include <Trade\Trade.mqh>
#include <Indicators\Indicators.mqh>

#include <nonconnu\AccountData.mqh>
#include <nonconnu\Event.mqh>
#include <nonconnu\MarketHelper.mqh>
#include <nonconnu\Logger.mqh>
#include <nonconnu\PriceSqueeze.mqh>
#include <nonconnu\TradeInfo.mqh>
#include <nonconnu\UniTrader.mqh>
#include <nonconnu\Utils.mqh>

static ulong MagicNumber;

input int Bot_Instance = 1; // Pour différencier les instances du bot sur une meme paire

input double RiskPercent = 2.0; // Risque (%)
input double StopLoss = 0; // Valeur du SL en points (0=valeur paire)
input double TakeProfitRatio = 0; // Niveau TP max. par rapport au SL (0=TP ouvert, 2=2xSL, 3=3xSL)
input int ExpirationMinutes = 0; // Durée de vie max. du tradde en minutes (0=illimité, 300=5h 3000=50h)

input bool BreakEvenEnabled = false; // Activer le Break Even (false=Non, true=Oui)
input bool TrailingStopEnabled = false; // Activer le trailing stop (false=Non, true=Oui)
input bool PartialEnabled = false; // Activer la fermeture partielle (false=Non, true=Oui)

input double PartialTPLevel1 = 1;  // Niveau TP pour fermeture partielle tier 1 (2=2xSL, 3=3xSL)
input double PartialSize1 = 0.3333; // Fraction fermeture partielle tier 1 (0.3333=33,33%, 0,5=50%)
input double PartialTPLevel2 = 2; // Niveau TP2 pour fermeture partielle (2=2xSL, 3=3xSL)
input double PartialSize2 = 0.5; // Fraction fermeture partielle tier 2 (0.5=50%, 1=100%)

input string ATRTimeframe = "M5"; // Echelle de temps pour l'ATR (M1, M5, M15, M30)
input int ATRPeriod = 14;          // Période de l'ATR (>=5 et <=30)
input double ATRMultiplier = 2.0; // Trailing stop multiplicateur de l'ATR (Min:.5, Max5.0, 0=valeur paire)

input string EventTime = "";   // Heure événement HH:MM:SS (ex: 09:0:10), vide pour désactiver
input string EventDays = "Semaine"; // Jours (Lun,Mar,Mer,Jeu,Ven,Sam,Dim,Semaine,Tous)
input string EventMonths = "Tous";  // Mois (Jan,Fev,Mar,Avr,Mai,Juin,Juil,Aou,...,Tous,SansJuilAou)
input string EventWeeks = "Toutes"; // Semaines (1,2,3,4,5,6,Toutes)
input ENUM_POSITION_TYPE EventTradeType = POSITION_TYPE_BUY; // Type de trade à ouvrir lors de l'événement (BUY ou SELL)

input string LogLevel = "Info";  // Niveau de journalisation (Info, Warn, Debug)
input bool IsFileLogEnabled = false; // Journalisation dans un fichier (false=Non, true=Oui)

// Button parameters
string buttonCloseAll = "CloseAllButton";
string buttonBuy = "BuyButton";
string buttonSell = "SellButton";
int buttonX = 5;
int buttonY = 40;
int buttonWidth = 90;
int buttonHeight = 30;

// Instances
UniTrader *unitrader;
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

    string message = StringFormat("V%s Risk=%.1f%%, SL=%.0f points, TP Ratio=%.1f",
        BOT_VERSION, RiskPercent, StopLoss, TakeProfitRatio);

    if (TrailingStopEnabled)
        message += StringFormat(", Trailing Stop: ATR(%s, Period=%d, Mult=%.1f)", ATRTimeframe, ATRPeriod, ATRMultiplier);

    if (BreakEvenEnabled)
        message += ", Break Even";

    if (PartialEnabled)
        message += StringFormat(", Partial: tier1(TP=%.1f, %.2f%%), tier2(TP=%.1f, %.2f%%)",
            PartialTPLevel1, PartialSize1 * 100, PartialTPLevel2, PartialSize2 * 100);

    if (ExpirationMinutes > 0)
        message += StringFormat(", Exp=%d h", ExpirationMinutes / 60);
    
    Comment(message);
    Log.Info("OnInit", message);

    PrintSymbolSpecifications();

    unitrader = UniTrader::Create(
        RiskPercent,
        StopLoss,
        TakeProfitRatio,
        TrailingStopEnabled,
        BreakEvenEnabled,
        PartialEnabled,
        PartialTPLevel1,
        PartialSize1,
        PartialTPLevel2,
        PartialSize2,
        ATRTimeframe,
        ATRPeriod,
        ATRMultiplier,
        ExpirationMinutes);

    if (unitrader == NULL ||!unitrader.IsValid())
    {
        delete unitrader;
        return INIT_FAILED;
    }
    unitrader.PrintInfo();

    // Restore state from open positions after timeframe change or EA restart
    unitrader.RestoreFromOpenPositions();

    CreateButton(buttonBuy, "BUY", buttonX, buttonY, buttonWidth, buttonHeight);
    ObjectSetInteger(0, buttonBuy, OBJPROP_BGCOLOR, clrGreen);
    CreateButton(buttonSell, "SELL", buttonX + buttonWidth + 5, buttonY, buttonWidth, buttonHeight);
    ObjectSetInteger(0, buttonSell, OBJPROP_BGCOLOR, clrOrangeRed);
    
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
    Log.Info("OnDeinit", "Fin du bot Mono - Raison: " + IntegerToString(reason) + " (" + GetDeinitReasonText(reason) + ")");
    ObjectDelete(0, buttonCloseAll);
    ObjectDelete(0, buttonBuy);
    ObjectDelete(0, buttonSell);
    ChartRedraw();
    
    delete event;
    delete unitrader;
    EventKillTimer();
}

void OnTimer()
{  
    // PositionInfoLog("OnTimer");
    // MarketInfo();
    // SymbolPositionInfo();
    
    if (HasOpenPositions())
    {
        if (ObjectFind(0, buttonCloseAll) < 0)
        {
            ObjectDelete(0, buttonBuy);
            ObjectDelete(0, buttonSell);
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
            CreateButton(buttonBuy, "BUY", buttonX, buttonY, buttonWidth, buttonHeight);
            ObjectSetInteger(0, buttonBuy, OBJPROP_BGCOLOR, clrGreen);
            CreateButton(buttonSell, "SELL", buttonX + buttonWidth + 5, buttonY, buttonWidth, buttonHeight);
            ObjectSetInteger(0, buttonSell, OBJPROP_BGCOLOR, clrOrangeRed);
            ChartRedraw();
        }
    }

    if (unitrader.IsTradeOpen())
    {
        Log.Debug("OnTimer", "Fermeture des trades expirés");
        unitrader.CloseExpiredTrades();
    }
    else
    {
        if (!HasOpenPositions() && event != NULL && event.IsReady())
        {

            if (!unitrader.Trade(EventTradeType))
                Log.Error("OnTimer", "Echec du Trade UniTrader");
        }
    }
}

void OnTick()
{
    // PositionInfoLog("OnTick");

    if (unitrader.IsTradeOpen())
    {
        if (!unitrader.IsBreakEven())
        {
            unitrader.BreakEven();
        }
        
        if (!unitrader.IsPartialProfit1())
        {
            unitrader.PartialProfit1();
        }
        else if (!unitrader.IsPartialProfit2())
        {
            unitrader.PartialProfit2();
        }

        unitrader.TrailingSL();
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
        Log.Info("Stop Loss", dealt);
        unitrader.ResetTrade();
    }
    else if (dealReason == DEAL_REASON_TP)
    {
        Log.Info("Take Profit", dealt);
        unitrader.ResetTrade();
    }
    else if (dealReason == DEAL_REASON_EXPERT && unitrader.IsTradeOpen() && (unitrader.IsPartialProfit1() || unitrader.IsPartialProfit2())) 
    {
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
      unitrader.ResetTrade();
    }
}

void OnChartEvent(const int id,
                  const long &lparam,
                  const double &dparam,
                  const string &sparam)
{
    if (id == CHARTEVENT_OBJECT_CLICK)
    {
        if (sparam == buttonBuy)
        {
            ObjectSetInteger(0, buttonBuy, OBJPROP_STATE, false);
            if (!HasOpenPositions())
            {
                if (!unitrader.Trade(POSITION_TYPE_BUY))
                    Log.Error("OnChartEvent", "Echec Trade BUY");
            }
        }
        else if (sparam == buttonSell)
        {
            ObjectSetInteger(0, buttonSell, OBJPROP_STATE, false);
            if (!HasOpenPositions())
            {
                if (!unitrader.Trade(POSITION_TYPE_SELL))
                    Log.Error("OnChartEvent", "Echec Trade SELL");
            }
        }
        else if (sparam == buttonCloseAll)
        {
            ObjectSetInteger(0, buttonCloseAll, OBJPROP_STATE, false);
            ChartRedraw();
            if (unitrader.CloseAllPositions())
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