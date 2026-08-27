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
  if (r === null) return '';
  if (r > 0) return 'text-emerald-400 font-bold';
  if (r < 0) return 'text-red-400 font-bold';
  return 'text-gray-400 font-bold';
}
