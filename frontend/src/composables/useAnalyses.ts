/**
 * Types + client du centre d'analyse « Rapport d'activité »
 * (backend analyses.rs — GET /api/analyses[/{strategie}]).
 */
import { http } from '@/services/http.client'

export interface PeriodeAnalyse {
  cle: string
  label: string
  dollars: number
  r: number
  trades: number
  gagnants: number
}

export interface CategorieAnalyse {
  label: string
  n: number
  dollars: number
  r: number
  /** 0-1 */
  wr: number
}

export interface ResumeJour {
  date: string
  dollars: number
  r: number
  trades: number
}

/// Croisé asset × TF (bloc Timeframes du rapport).
export interface ParAssetTf {
  asset: string
  dollars: number
  n: number
  tfs: CategorieAnalyse[]
}

export interface AnalyseStrategie {
  strategie: string
  etat: string
  source: 'rejeu' | 'base'
  nb_trades: number
  fenetre_debut: number
  fenetre_fin: number
  capital_depart: number
  capital_actuel: number
  fraction_risque: number
  r_total: number
  taux_reussite: number
  hier: ResumeJour | null
  journalier: PeriodeAnalyse[]
  hebdomadaire: PeriodeAnalyse[]
  mensuel: PeriodeAnalyse[]
  verdicts: CategorieAnalyse[]
  assets: CategorieAnalyse[]
  tfs: CategorieAnalyse[]
  par_asset_tf: ParAssetTf[]
}

export interface ResumeStrategie {
  strategie: string
  etat: string
  source: 'rejeu' | 'base'
  nb_trades: number
  capital_depart: number
  capital_actuel: number
  r_total: number
  taux_reussite: number
  hier: ResumeJour | null
}

export async function chargerAnalyse(id: string): Promise<AnalyseStrategie | null> {
  try {
    const res = await http.get<AnalyseStrategie>(`/api/analyses/${id}`)
    return res.data
  } catch {
    return null
  }
}

export async function chargerAnalyses(): Promise<ResumeStrategie[]> {
  try {
    const res = await http.get<{ strategies: ResumeStrategie[] }>('/api/analyses')
    return res.data.strategies ?? []
  } catch {
    return []
  }
}

/// Format $ : 2 093 $ — signe − typographique devant la somme négative.
export function fmtDollars(v: number): string {
  const n = Math.round(Math.abs(v)).toLocaleString('fr-FR')
  return `${v < 0 ? '−' : ''}${n} $`
}

/// Snapshot quotidien persisté (§14 — évolution jour après jour).
export interface SnapshotAnalyse {
  strategie: string
  jour: string
  capital_depart: number
  capital_actuel: number
  r_total: number
  taux_reussite: number
  nb_trades: number
  hier_dollars: number | null
  calcule_le: number
  avis_ia: string | null
  avis_le: number | null
}

/// Historique des snapshots d'une stratégie (du plus récent au plus ancien).
export async function chargerHistoriqueAnalyses(id: string): Promise<SnapshotAnalyse[]> {
  try {
    const res = await http.get<{ snapshots: SnapshotAnalyse[] }>(`/api/analyses/${id}/historique`)
    return res.data.snapshots ?? []
  } catch {
    return []
  }
}

/// Avis structuré de l'analyste IA (POST /api/analyses/{id}/ia).
export interface AnalyseIa {
  etat: string
  points_forts: string[]
  points_faibles: string[]
  corrections: string[]
  /** 0-100 */
  confiance: number
  nb_trades: number
  generee_le: number
}

/// Génère (ou sert le cache du jour) l'analyse IA d'une stratégie.
export async function genererAnalyseIa(id: string): Promise<{ en_cache: boolean; analyse: AnalyseIa } | null> {
  try {
    const res = await http.post<{ en_cache: boolean; analyse: AnalyseIa }>(`/api/analyses/${id}/ia`, null, { timeout: 180_000 })
    return res.data
  } catch {
    return null
  }
}

/// R signé à 1 décimale : +4.7 R / −1.0 R.
export function fmtR(v: number): string {
  const r = Math.round(v * 10) / 10
  return `${r > 0 ? '+' : r < 0 ? '−' : ''}${Math.abs(r).toFixed(1)} R`
}

/// Couleur de verdit canonique (badge/point).
export function couleurVerdict(v: string): string {
  if (v === 'TP3' || v === 'TS') return '#22d3ee'
  if (v === 'TP2+BE') return '#60a5fa'
  if (v === 'TP1+BE') return '#34d399'
  if (v === 'SL') return '#f87171'
  if (v === 'BE') return '#e5e7eb'
  return '#94a3b8'
}
