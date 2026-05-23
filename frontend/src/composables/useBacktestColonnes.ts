import type { BacktestResult } from '@/services/api.backtest'

export type ColonneComp = {
  key: string
  label: string
  tooltip: string
  echelle?: string[]
  valeur: (r: BacktestResult) => string
  couleur: (r: BacktestResult) => string
}

export function echelleColor(ligne: string): string {
  if (ligne.startsWith('🟢')) return 'text-emerald-400'
  if (ligne.startsWith('🟡')) return 'text-yellow-400'
  if (ligne.startsWith('🔴')) return 'text-red-400'
  return 'text-gray-400'
}

const pct = (v: number) => (v * 100).toFixed(1) + '%'
const usd = (v: number) => '$' + v.toLocaleString('fr-FR', { maximumFractionDigits: 0 })
const c = (v: number, s1: number, s2: number) =>
  v >= s1 ? 'text-emerald-400 font-semibold' : v >= s2 ? 'text-yellow-400 font-semibold' : 'text-red-400 font-semibold'

export const colonnes: ColonneComp[] = [
  {
    key: 'nt', label: 'Nb. trades',
    tooltip: 'Nombre total de trades exécutés sur la période.',
    valeur: r => String(r.nb_trades), couleur: _r => 'text-gray-200',
  },
  {
    key: 'tg', label: 'Trades gagnants',
    tooltip: 'Nombre de trades clôturés en profit.',
    valeur: r => String(Math.round(r.nb_trades * r.win_rate)), couleur: _r => 'text-emerald-400',
  },
  {
    key: 'tp', label: 'Trades perdants',
    tooltip: 'Nombre de trades clôturés en perte.',
    valeur: r => String(r.nb_trades - Math.round(r.nb_trades * r.win_rate)), couleur: _r => 'text-red-400',
  },
  {
    key: 'wr', label: 'Win Rate',
    tooltip: '% de trades gagnants.',
    echelle: ['🟢 ≥55% (objectif projet)', '🟡 45–55%', '🔴 <45%'],
    valeur: r => pct(r.win_rate), couleur: r => c(r.win_rate, 0.55, 0.45),
  },
  {
    key: 'pf', label: 'Profit Factor',
    tooltip: 'Gains totaux ÷ Pertes totales. Sous 1 = stratégie perdante sur la période.',
    echelle: ['🟢 ≥1.5', '🟡 1.2–1.5', '🔴 <1.2'],
    valeur: r => r.profit_factor.toFixed(2), couleur: r => c(r.profit_factor, 1.5, 1.2),
  },
  {
    key: 'sh', label: 'Sharpe ratio',
    tooltip: 'Rendement ajusté au risque (annualisé). Un Sharpe >1.5 = excellent profil risque/gain.',
    echelle: ['🟢 ≥1.5', '🟡 1–1.5', '🔴 <1'],
    valeur: r => r.sharpe.toFixed(2), couleur: r => c(r.sharpe, 1.5, 1.0),
  },
  {
    key: 'dd', label: 'Drawdown max',
    tooltip: 'Perte maximale depuis un pic de capital. Limite projet : 20% (arrêt auto trading).',
    echelle: ['🟢 ≤10%', '🟡 ≤20%', '🔴 >20%'],
    valeur: r => pct(r.drawdown_max),
    couleur: r => r.drawdown_max <= 0.1 ? 'text-emerald-400 font-semibold' : r.drawdown_max <= 0.2 ? 'text-yellow-400 font-semibold' : 'text-red-400 font-semibold',
  },
  {

    key: 'cf', label: 'Capital final',
    tooltip: 'Capital à la clôture de la période.',
    echelle: ['🟢 supérieur au capital initial', '🔴 inférieur'],
    valeur: r => usd(r.capital_final),
    couleur: r => r.capital_final >= r.config.capital_initial ? 'text-emerald-400 font-semibold' : 'text-red-400 font-semibold',
  },
  {
    key: 'rm', label: 'R moyen / trade',
    tooltip: 'Espérance mathématique par trade en multiple du risque unitaire. Un R positif = stratégie profitable en moyenne.',
    echelle: ['🟢 >0', '🔴 ≤0'],
    valeur: r => r.pnl_r_moyen.toFixed(2) + 'R', couleur: r => r.pnl_r_moyen > 0 ? 'text-emerald-400' : 'text-red-400',
  },
  {
    key: 'pl', label: 'P&L total (R)',
    tooltip: 'Somme de tous les P&L en multiples de Risk sur la période.',
    echelle: ['🟢 positif', '🔴 négatif'],
    valeur: r => r.pnl_total_r.toFixed(1) + 'R', couleur: r => r.pnl_total_r > 0 ? 'text-emerald-400' : 'text-red-400',
  },
  {
    key: 'dsl', label: 'Double SL',
    tooltip: 'Straddle uniquement. % de trades où les deux jambes (Long + Short) sont stoppées. Chaque double SL coûte 2R — il efface l\'effet de 2 trades gagnants.',
    echelle: ['🟢 <20% : risque contrôlé', '🟡 20–40% : surveiller', '🔴 >40% : revoir les créneaux'],
    valeur: r => r.double_sl_rate !== null ? pct(r.double_sl_rate) : '—',
    couleur: r => r.double_sl_rate !== null ? (r.double_sl_rate <= 0.25 ? 'text-emerald-400' : 'text-red-400') : 'text-gray-600',
  },
  {
    key: 'sp', label: 'Pertes consécutives',
    tooltip: 'Plus longue série de pertes d\u2019affilée. Indicateur de résistance psychologique et de drawdown prolongé.',
    echelle: ['🟢 ≤4', '🟡 ≤7', '🔴 >7'],
    valeur: r => String(r.serie_pertes_max),
    couleur: r => r.serie_pertes_max <= 4 ? 'text-emerald-400' : r.serie_pertes_max <= 7 ? 'text-yellow-400' : 'text-red-400',
  },
  {
    key: 'sg', label: 'Gains consécutifs',
    tooltip: 'Plus longue série de gains d\u2019affilée. Indicateur de momentum favorable. Pas de seuil critique, information qualitative.',
    valeur: r => String(r.serie_gains_max), couleur: _r => 'text-blue-400',
  },
  {
    key: 'cm', label: 'Capital minimum',
    tooltip: 'Plus bas capital atteint pendant la période. Indique la pire phase de drawdown rencontrée.',
    echelle: ['🟢 ≥85% du capital initial', '🔴 <85%'],
    valeur: r => usd(r.capital_min),
    couleur: r => r.capital_min >= r.config.capital_initial * 0.85 ? 'text-emerald-400' : 'text-red-400',
  },
]
