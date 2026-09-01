import { dateHeureParis } from '@/utils/date'

export function formatDate(ts: number): string {
  if (!ts) return '—'
  return dateHeureParis(ts)
}

export function formatNombre(v: number | undefined): string {
  if (v === undefined || v === null) return '—'
  if (v >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(v)
  if (v >= 1) return v.toFixed(4)
  return v.toFixed(6)
}

export function classeVerdictSignal(verdict: string | null): string {
  const v = verdict?.toLowerCase() ?? ''
  if (v === 'tp3' || v === 'tp2') return 'badge-green'
  if (v === 'tp1') return 'badge-blue'
  if (v === 'be') return 'badge-gray'
  if (v === 'sl') return 'badge-red'
  if (v === 'invalide') return 'badge-orange'
  if (v === 'expire') return 'badge-gray'
  return 'badge-yellow'
}

export function labelVerdictSignal(verdict: string | null): string {
  const v = verdict?.toLowerCase() ?? ''
  if (v === 'tp3') return '✅ TP3'
  if (v === 'tp2+be') return '✅ TP2+BE (+2R acquis)'
  if (v === 'tp1+be') return '✅ TP1+BE (+1R acquis)'
  if (v === 'tp2') return '✅ TP2 (SL→TP1)'
  if (v === 'tp1') return '✅ TP1 (SL→BE)'
  if (v === 'be')  return '⚪ BE (dégradation zone) — 0R'
  if (v === 'sl')  return '❌ SL'
  if (v === 'invalide') return '↩️ Entrée non atteinte'
  if (v === 'expire') return '⏰ Expiré'
  return '⏳ En cours'
}

// ── État vivant d'un signal (non clôturé) ────────────────────────────────────
// Distingue « En attente » (annoncé, entrée pas encore touchée/planifiée) de
// « En cours » (trade ouvert). Sémantique de `heure_entree` par stratégie :
// - SMC    : marqué par le moteur quand le prix revient toucher l'entrée
//            (NULL = ordre en limite en attente de remplissage).
// - Straddle : heure d'entrée PLANIFIÉE (annonce HIGH impact) — compte à rebours.
// - Rockets  : position ouverte dès le signal (pas d'attente de remplissage).

interface EtatSignal {
  statut?: string | null
  verdict: string | null
  heure_entree?: number | null
  strategie?: string | null
}

function estVivant(s: EtatSignal): boolean {
  return (s.statut ?? '') !== 'Fermé' && s.verdict === null
}

export function labelEtatSignal(s: EtatSignal): string {
  if (!estVivant(s)) {
    // Straddle « be » : jambe gagnante revenue à E après TP1 (+1R acquis)
    // nettée contre le SL de la perdante (-1R) — PAS un BE forcé BOS.
    if (s.verdict?.toLowerCase() === 'be' && (s.strategie ?? '').toLowerCase() === 'straddle') {
      return '⚖️ TP1 gagnante − SL perdante (0R net)'
    }
    return labelVerdictSignal(s.verdict)
  }
  const strat = (s.strategie ?? '').toLowerCase()
  if (strat === 'straddle' && s.heure_entree) {
    const reste = s.heure_entree - Math.floor(Date.now() / 1000)
    return reste > 0 ? `⏳ Entrée dans ${Math.ceil(reste / 60)} min` : '🟢 En cours'
  }
  if (strat.includes('rocket')) return '⏳ En cours'
  return s.heure_entree ? '🟢 En cours' : '⏳ En attente'
}

export function classeEtatSignal(s: EtatSignal): string {
  if (!estVivant(s)) return classeVerdictSignal(s.verdict)
  return labelEtatSignal(s) === '🟢 En cours' ? 'badge-green' : 'badge-yellow'
}

export function titreEtatSignal(s: EtatSignal): string {
  if (!estVivant(s)) return ''
  const strat = (s.strategie ?? '').toLowerCase()
  if (strat === 'straddle') {
    if (!s.heure_entree) return 'En attente de l\u2019heure d\u2019entrée (annonce HIGH impact)'
    const reste = s.heure_entree - Math.floor(Date.now() / 1000)
    return reste > 0
      ? 'Entrée planifiée : les 2 jambes seront posées à l\u2019heure E (T-10 s)'
      : 'Passes straddle actives : les 2 jambes sont en place'
  }
  if (strat.includes('rocket')) return 'Position ouverte dès le signal — gestion par le moteur Rockets'
  return s.heure_entree
    ? 'Entrée touchée : trade ouvert — ses boxes sont visibles sur TOUS les timeframes'
    : 'Annoncé : ordre en limite — se remplit quand le prix revient toucher l\u2019entrée (pointillés sur le graphique de son TF)'
}

export function calculerR(signal: { direction: string; prix_entree: number; stop_loss: number; prix_verdict: number | null; r_realise?: number | null }): number | null {
  // Vérité du moteur en priorité (TP2 encaissé puis sortie à BE = +2R alors
  // que le prix de sortie vaut l'entrée) ; repli prix pour les lignes
  // antérieures au 24/08.
  if (signal.r_realise !== null && signal.r_realise !== undefined) return signal.r_realise;
  if (!signal.prix_verdict || !signal.prix_entree || !signal.stop_loss) return null;
  const diffPx = signal.prix_entree - signal.stop_loss;
  if (Math.abs(diffPx) < 0.000001) return null; // Évite la division par zéro

  const risk = Math.abs(diffPx);
  const isLong = signal.direction.toUpperCase() === 'LONG';
  const pnl = isLong
    ? (signal.prix_verdict - signal.prix_entree)
    : (signal.prix_entree - signal.prix_verdict);

  return pnl / risk;
}

export function formatR(r: number | null): string {
  if (r === null) return '';
  const sign = r > 0 ? '+' : '';
  return `${sign}${r.toFixed(2)}R`;
}

export function classeR(r: number | null): string {
  if (r === null) return ''
  if (r > 0) return 'text-emerald-400 font-bold'
  if (r < 0) return 'text-red-400 font-bold'
  return 'text-white font-bold'
}

// ── Palier max atteint & R de référence (spéc propriétaire 31/08) ────────────
// La vérité qui juge l'entrée est l'EXTRÊME atteint (SL ou TP max touché),
// pas la sortie. Le R de référence se déduit des niveaux stockés :
//   SMC      : dist(tp_n)/risque (TP1 ≈ 0.6R post-étape 4, TP2 = 2R…)
//   Straddle : idem MOINS 1R de la jambe perdante (TP2 touché = +1R net)

interface SignalPalier {
  strategie?: string | null
  direction: string
  prix_entree: number
  stop_loss: number
  take_profit: number[]
  verdict?: string | null
}

export interface PalierMax {
  palier: 'SL' | 'TP1' | 'TP2' | 'TP3' | 'BE' | 'Expiré' | 'Non rempli' | null
  rReference: number | null
}

function rNiveau(niveau: number, entree: number, sl: number): number | null {
  const risque = Math.abs(entree - sl)
  if (risque < 1e-9) return null
  return Math.abs(niveau - entree) / risque
}

export function palierMax(s: SignalPalier): PalierMax {
  const v = s.verdict?.toLowerCase() ?? ''
  const straddle = (s.strategie ?? '').toLowerCase() === 'straddle'
  const penalite = straddle ? 1 : 0 // la jambe perdante a payé 1R
  if (v === 'sl' || v === 'sl+be') return { palier: 'SL', rReference: -1 }
  if (v === 'tp1' || v === 'tp1+be')
    return { palier: 'TP1', rReference: (rNiveau(s.take_profit[0], s.prix_entree, s.stop_loss) ?? 0) - penalite }
  if (v === 'tp2' || v === 'tp2+be')
    return { palier: 'TP2', rReference: (rNiveau(s.take_profit[1] ?? s.take_profit[0], s.prix_entree, s.stop_loss) ?? 0) - penalite }
  if (v === 'tp3')
    return { palier: 'TP3', rReference: (rNiveau(s.take_profit[2] ?? s.take_profit[0], s.prix_entree, s.stop_loss) ?? 0) - penalite }
  if (v === 'be') return { palier: 'BE', rReference: 0 }
  if (v === 'expire') return { palier: 'Expiré', rReference: null }
  if (v === 'invalide') return { palier: 'Non rempli', rReference: null }
  return { palier: null, rReference: null }
}

export function labelPalierMax(p: PalierMax['palier']): string {
  switch (p) {
    case 'SL': return '❌ SL'
    case 'TP1': return '🎯 TP1'
    case 'TP2': return '✅ TP2'
    case 'TP3': return '🏆 TP3'
    case 'BE': return '⚪ BE'
    case 'Expiré': return '⏰ Expiré'
    case 'Non rempli': return '↩️ Non rempli'
    default: return '—'
  }
}

export function classePalierMax(p: PalierMax['palier']): string {
  switch (p) {
    case 'TP3': case 'TP2': return 'badge-green'
    case 'TP1': return 'badge-blue'
    case 'SL': return 'badge-red'
    default: return 'badge-gray'
  }
}

/** Palier atteint À CET INSTANT par le prix courant (trades ouverts). */
export function palierActuel(prix: number | null, s: SignalPalier): PalierMax['palier'] {
  if (prix === null) return null
  const long = s.direction.toUpperCase() === 'LONG'
  if (long ? prix <= s.stop_loss : prix >= s.stop_loss) return 'SL'
  const [tp1, tp2, tp3] = s.take_profit
  if (tp3 !== undefined && (long ? prix >= tp3 : prix <= tp3)) return 'TP3'
  if (tp2 !== undefined && tp2 !== null && (long ? prix >= tp2 : prix <= tp2)) return 'TP2'
  if (long ? prix >= tp1 : prix <= tp1) return 'TP1'
  return null
}

/** Libellé MFE : excursion favorable avant le SL (« +0.85R avant SL »). */
export function formatMfe(mfeR: number | null): string {
  if (mfeR === null || mfeR === undefined) return ''
  const sign = mfeR > 0 ? '+' : ''
  return `${sign}${mfeR.toFixed(2)}R avant SL`
}
