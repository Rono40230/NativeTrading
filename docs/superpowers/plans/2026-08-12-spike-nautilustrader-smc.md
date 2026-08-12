# Spike NautilusTrader — Valider SMC sur XAUUSD via OANDA

> **Type** : Preuve de concept (POC) bornée — 2-3 jours max.
> **Objectif** : Valider que NautilusTrader peut (a) s'installer, (b) recevoir des données
> XAUUSD M15 d'OANDA (démo gratuit), (c) faire tourner un backtest, et (d) porter un
> concept SMC (Order Blocks) en Python. Si ça marche → la migration NautilusTrader est
> viable. Si ça coince → on revient à 1.7A sans regret (le spike est jetable).
>
> **Répertoire** : `/mnt/IA/nautilus-smc-spike/` (NOUVEAU projet, indépendant de native-trading-ai).
>
> Date : 2026-08-12 · Statut : à exécuter par l'assistant avec accord du propriétaire.

---

## Prérequis (par le propriétaire, ~10 min)

1. **Compte OANDA practice** : https://www.oanda.com/account-register/ → créer un compte démo (Practice).
2. **Token API** : dans le dashboard OANDA → "Manage API Access" → "Generate API Token". **Copier le token**.
3. **Account ID** : visible dans le dashboard OANDA (format `001-00X-000000-001`). **Le noter**.
4. **Python 3.12+** : vérifier `python3 --version` sur le système. Si < 3.12 → installer via `dnf install python3.12` (Fedora) ou pyenv.

> ⚠ **Le token OANDA est un secret** : le stocker dans un `.env` (jamais en clair dans le code ni git).

---

## Étape 1 : Installer NautilusTrader (~15 min)

**But** : valider que le package s'installe et s'importe.

```bash
mkdir -p /mnt/IA/nautilus-smc-spike && cd /mnt/IA/nautilus-smc-spike
python3.12 -m venv .venv
source .venv/bin/activate
pip install nautilus_trader
```

**Vérification** :
```bash
python -c "import nautilus_trader; print(nautilus_trader.__version__)"
```
Expected : un numéro de version (ex: `1.200.0` ou similaire).

**Si échec** : NautilusTrader fournit des **wheels pré-compilés** (pas besoin de Rust). Si pas de wheel pour ta plateforme → installer Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`) puis `pip install nautilus_trader` (compilation). Sur Fedora + Python 3.12 + CPU x86_64, le wheel devrait exister.

---

## Étape 2 : Connecter OANDA + récupérer XAUUSD M15 (~30 min)

**But** : valider que NautilusTrader peut pull des données OANDA.

Créer `spike_data.py` :
```python
import asyncio
import os
from nautilus_trader.adapters.oanda.config import OandaDataClientConfig
from nautilus_trader.adapters.oanda.factories import OandaLiveDataClientFactory
from nautilus_trader.adapters.oanda.config import OandaVenueConfig
from nautilus_trader.common.clock import LiveClock
from nautilus_trader.live.node import TradingNode

# Config OANDA (token + account depuis .env)
config = OandaDataClientConfig(
    api_key=os.getenv("OANDA_API_KEY"),
    account_id=os.getenv("OANDA_ACCOUNT_ID"),
    environment="practice",  # ou "demo"
)

# Pull des bougies XAUUSD M15 (historique)
async def main():
    # Utiliser le OANDA data client pour récupérer l'historique
    # (voir docs: nautilustrader.io/docs/latest/integrations/oanda/)
    # But : obtenir ~500 bougies XAUUSD M15
    ...

asyncio.run(main())
```

> ⚠ **L'API exacte** peut varier selon la version (breaking changes v2 août 2026). L'implementer doit suivre la doc officielle : https://nautilustrader.io/docs/latest/integrations/oanda/. Ne pas copier-coller le code ci-dessus aveuglément — c'est un squelette. Lire la doc de la version installée.

**Vérification** : ~500 bougies XAUUSD M15 (Open/High/Low/Close/Volume) sont téléchargées et affichables (ex: 5 dernières bougies en print).

**Si l'adapter OANDA ne fonctionne pas** pour le backtest → alternative : télécharger l'historique via l'API REST OANDA directement (`curl https://api-fxpractice.oanda.com/v3/instruments/XAU_USD/candles?granularity=M15&count=500` avec le token Bearer) → convertir en CSV → utiliser `GenericCSVDataProvider` dans NautilusTrader. C'est un fallback acceptable pour le spike.

---

## Étape 3 : Backtest trivial (~30 min)

**But** : valider que le BacktestEngine tourne sur XAUUSD M15.

Créer `spike_backtest.py` avec :
- Un **BacktestNode** (ou `BacktestEngine`) configuré avec les données XAUUSD M15.
- Une **stratégie triviale** (ex: Moving Average crossover) — NautilusTrader en fournit dans `examples/`.
- Lancer le backtest → obtenir des métriques (trades, win rate, P&L).

```python
from nautilus_trader.backtest.engine import BacktestEngine
from nautilus_trader.model.identifiers import Venue
# ... configurer engine + data + stratégie triviale ...
# Lancer → print du rapport
```

**Vérification** : le backtest tourne sans erreur et produit un rapport (nombre de trades, P&L).

---

## Étape 4 : Porter l'indicateur Order Blocks (Python, ~2-4h)

**But** : valider qu'un concept SMC est portable.

Source : `/home/rono/Applis Nono/Indicateur Scalp à Nono/BSZones.pine` (Order Blocks) + `PseudoCode.md`.

Créer `smc_order_blocks.py` — un indicateur custom NautilusTrader (ou une fonction pure) qui :
1. Détecte les **Order Blocks** : la bougie précédant une impulsion (corps ≥ 1.5×ATR), avec 3 états (vierge/partiel/profond).
2. Détecte le **BOS** (Break of Structure) : clôture au-dessus du dernier swing high.
3. Expose les OB actifs + le sens (bull/bear).

Référence Pine (BSZones.pine MODULE 7) :
- OB BULL = `close>open AND close[1]<open[1] AND roc>seuil` → l'OB = la bougie `[1]` (top=high[1], bot=low[1]).
- État : 0=Vierge, 1=Partiel (close>mid), 2=Profond (close≤mid).
- ATR14 (Wilder) pour le seuil.

L'indicateur utilise ATR14 (disponible dans `nautilus_indicators` ou à calculer).

**Vérification** : l'indicateur produit des Order Blocks sur les 500 bougies XAUUSD M15, comparables (visuellement ou par count) à ce que tu vois sur ton indicateur Pine/MQL5 sur TradingView.

---

## Étape 5 : Stratégie SMC minimale + backtest comparatif (~2-4h)

**But** : valider que la stratégie SMC produit des signaux cohérents avec ton EA MQL5.

Créer `smc_strategy.py` — une `Strategy` NautilusTrader qui :
1. Sur chaque `on_bar` (clôture M15) : appelle l'indicateur Order Blocks.
2. Si prix revient dans un OB vierge (BOS confirmé) → émet un signal (order).
3. SL = bas de l'OB, TP = 1.5R / 2R / 3R.
4. Journalise chaque signal (date, prix, sens, OB utilisé).

```python
from nautilus_trader.trading.strategy import Strategy

class SmcObStrategy(Strategy):
    def on_bar(self, bar):
        # Calculer OB + BOS
        # Si retest d'OB → submit_order
        ...
```

**Backtest** sur XAUUSD M15 (même période que ton EA MQL5).

**Comparaison** : le nombre de signaux + les dates + le win rate doivent être **proches** de ton EA MQL5 (aux différences de source de données / spread près).

---

## Critères de décision (à la fin du spike)

| Résultat | Décision |
|----------|----------|
| ✅ Étapes 1-5 réussies + signaux proches du MQL5 | **NautilusTrader validé** → continuer le portage SMC complet (BSZones), brancher en live OANDA, abandonner le moteur Rust. |
| ⚠ Étapes 1-3 OK mais portage OB (4-5) trop dur | **Partiel** → évaluer : est-ce Python/NautilusTrader qui coince, ou juste la complexité SMC ? Si NautilusTrader fonctionne mais le portage SMC est long → c'est OK, c'est prévu (la SMC est riche). |
| ❌ Étape 1 ou 2 bloque (install/data impossible) | **NautilusTrader écarté** → revenir à 1.7A (ingestion persistante dans l'app Rust actuelle). |

---

## Notes

- Ce spike est **jetable** : il vit dans `/mnt/IA/nautilus-smc-spike/`, indépendant de `native-trading-ai`. Aucun impact sur l'app actuelle.
- L'assistant exécute le spike (le propriétaire ne code pas). Mais le propriétaire doit fournir : (a) le token OANDA, (b) l'accès au projet Pine/MQL5 de référence (déjà disponible).
- Le spike peut être assisté par l'IA (ChatGPT/Claude sont bons en Python + NautilusTrader).
- Si la version NautilusTrader installée a des **breaking changes v2** (août 2026), l'assistant adapte le code selon la doc officielle (ne pas copier-coller ce plan aveuglément — c'est un squelette, la doc est la source de vérité).
