# Screening TradingView — KDJ/Halftrend Intraday (phase A)

> Objectif : savoir si la stratégie est **porteuse** avant d'investir dans les
> conversions (Rust, MQ5) et l'intégration. Zéro code — tu fais tourner le
> strategy tester de TradingView avec un protocole honnête.
> La stratégie : `docs/reference/strategie_550_pourcent_v4.pine`.

## 1. Réglages du tester (Properties) — l'honnêteté avant tout

| Réglage | Valeur | Pourquoi |
|---|---|---|
| Initial capital | 10 000 | standardise la lecture |
| Base currency | USD | comparabilité entre actifs |
| Commission | **0,05 % par ordre** (FX/métaux/indices : ~1 point selon actif) | les défauts TV = 0 frais = mirage |
| Slippage | **2 ticks** | idem |
| Order size | 100 % of equity | simple |
| Recalculate | On bar close (défaut) | + noter : les sorties TP/SL sont intrabar → si tu as le Bar Magnifier (Premium), active-le pour un second passage de contrôle |

## 2. La grille : 8 actifs × 3 timeframes

H1 · H4 · D1 — **pas de M1/M5** (ce n'est pas du scalping). Teste chaque
actif SUR UN GRAPHIQUE SÉPARÉ, note les 6 chiffres du Strategy Tester :

| Actif | TF | Nb trades | Net profit | Profit factor | % gagnants | Max drawdown |
|---|---|---|---|---|---|---|
| NAS100 | H1 / H4 / D1 | | | | | |
| DAX | H1 / H4 / D1 | | | | | |
| SP500 | H1 / H4 / D1 | | | | | |
| EURUSD | H1 / H4 / D1 | | | | | |
| XAUUSD | H1 / H4 / D1 | | | | | |
| XAGUSD | H1 / H4 / D1 | | | | | |
| BTCUSD | H1 / H4 / D1 | | | | | |
| ETHUSD | H1 / H4 / D1 | | | | | |

Paramètres de la stratégie partout : **period 20, signal 7, Amplitude 2,
RatioRisk 2** (defaults — la calibration viendra en phase B, sur le rejeu).

## 3. Critères de lecture (go / no-go)

- **≥ 30 trades** sur la période, sinon la cellule est « non concluante »
  (laisse l'historique maximal disponible par TF)
- **Profit factor > 1,3** après frais sur les cellules concluantes
- **Positif sur au moins la moitié des actifs** (pas seulement 1-2)
- Max drawdown < 30 % du capital
- ⚠️ Règle d'or : l'actif qui a inspiré « 550 % » ne compte pas comme preuve

## 4. Ce que tu me rapportes

La grille remplie (ou des captures du Performance Summary par actif/TF).
Je fais la synthèse, je compare avec le benchmark du projet (SMC M15
+0,051 R/trade sur 24 mois), et on prend le go/no-go ensemble.
