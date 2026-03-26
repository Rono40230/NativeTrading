import { computed, type Ref } from 'vue'
import type { BacktestResults } from '@/services/api.service'

export function usePnLNiveaux(resultats: Ref<BacktestResults | null>) {
  const niveaux = computed(() => {
    if (!resultats.value) return null
    const r = resultats.value
    return {
      roi:          [{ label: '≥ 15% — Production', couleur: 'emerald', actif: r.roi_pct >= 15 }, { label: '0–15% — Positif', couleur: 'yellow', actif: r.roi_pct >= 0 && r.roi_pct < 15 }, { label: '< 0% — Perdant', couleur: 'red', actif: r.roi_pct < 0 }],
      sharpe:       [{ label: '≥ 1.5 — Excellent', couleur: 'emerald', actif: r.sharpe_ratio >= 1.5 }, { label: '1–1.5 — Correct', couleur: 'yellow', actif: r.sharpe_ratio >= 1.0 && r.sharpe_ratio < 1.5 }, { label: '< 1.0 — Faible', couleur: 'red', actif: r.sharpe_ratio < 1.0 }],
      winRate:      [{ label: '≥ 55% — Atteint', couleur: 'emerald', actif: r.win_rate >= 55 }, { label: '45–55% — Marginal', couleur: 'yellow', actif: r.win_rate >= 45 && r.win_rate < 55 }, { label: '< 45% — Faible', couleur: 'red', actif: r.win_rate < 45 }],
      drawdown:     [{ label: '≤ 10% — Excellent', couleur: 'emerald', actif: r.max_drawdown_pct <= 10 }, { label: '10–20% — Acceptable', couleur: 'yellow', actif: r.max_drawdown_pct > 10 && r.max_drawdown_pct <= 20 }, { label: '> 20% — Arrêt auto', couleur: 'red', actif: r.max_drawdown_pct > 20 }],
      profitFactor: [{ label: '≥ 1.5 — Performant', couleur: 'emerald', actif: r.profit_factor >= 1.5 }, { label: '1–1.5 — Neutre', couleur: 'yellow', actif: r.profit_factor >= 1.0 && r.profit_factor < 1.5 }, { label: '< 1.0 — Perdant', couleur: 'red', actif: r.profit_factor < 1.0 }],
      profitNet:    [{ label: 'Positif — Rentable', couleur: 'emerald', actif: r.profit_net >= 0 }, { label: 'Négatif — En perte', couleur: 'red', actif: r.profit_net < 0 }],
      capitalFinal: [{ label: 'Capital en hausse', couleur: 'emerald', actif: r.capital_final >= r.capital_initial }, { label: 'Capital en baisse', couleur: 'red', actif: r.capital_final < r.capital_initial }],
    }
  })

  const pyramidalisation = computed(() => {
    if (!resultats.value) return []
    const r = resultats.value
    return [
      { label: 'TP3 complet', n: r.nb_tp3, color: 'text-emerald-400', classes: 'bg-emerald-900/30 border-emerald-500/20' },
      { label: 'TP2 (⅔)',     n: r.nb_tp2, color: 'text-blue-400',    classes: 'bg-blue-900/30 border-blue-500/20' },
      { label: 'TP1 (⅓)',     n: r.nb_tp1, color: 'text-yellow-400',  classes: 'bg-yellow-900/30 border-yellow-500/20' },
      { label: 'SL / BE',     n: r.nb_sl,  color: 'text-red-400',     classes: 'bg-red-900/30 border-red-500/20' },
    ]
  })

  return { niveaux, pyramidalisation }
}
