// ── Poste d'observation rockets — état et dérivés partagés ─────────────────
// (décision 06/09 : lecture seule, le moteur décide). Le chargement et le
// live Binance vivent ici ; les sections À risque / Neutralisées de la page
// consomment l'état réactif. Layout 06/09 : les 3 blocs (risque,
// neutralisées, historique) partagent le même design, empilés pleine largeur,
// la colonne Setups occupant toute la hauteur.

import { computed, onMounted, onUnmounted, ref } from 'vue'
import { http } from '@/services/http.client'

export interface Position {
  cle: string; symbole: string; univers: 'crypto' | 'action'
  ouvert_le: number; entree: number; stop: number; r1: number
  qty: number; capital_epoque: number; risque_pct: number; montant: number
  dernier_close: number | null; close_veille: number | null
  trailing?: number | null; prix_r1?: number | null
  qty_restante?: number; montant_restant?: number
}

/// Cours/tendance : crypto → Binance live (CSP autorisé) ; action → dernier
/// close D1 Tiingo fourni par l'endpoint.
export function coursDe(live: Record<string, { prix: number; open: number }>, p: Position): number | null {
  if (p.univers === 'crypto') return live[p.symbole]?.prix ?? p.dernier_close
  return p.dernier_close
}
export function tendanceDe(live: Record<string, { prix: number; open: number }>, p: Position): number | null {
  if (p.univers === 'crypto') {
    const l = live[p.symbole]
    return l && l.open > 0 ? (l.prix / l.open - 1) * 100 : null
  }
  return p.dernier_close && p.close_veille && p.close_veille > 0
    ? (p.dernier_close / p.close_veille - 1) * 100
    : null
}
export function plLatent(live: Record<string, { prix: number; open: number }>, p: Position): number | null {
  const c = coursDe(live, p)
  return c === null ? null : (c - p.entree) * p.qty
}
export function rLatent(live: Record<string, { prix: number; open: number }>, p: Position): number | null {
  const c = coursDe(live, p)
  const dist = p.entree - p.stop
  return c === null || dist <= 0 ? null : (c - p.entree) / dist
}
export function plNeutralisee(live: Record<string, { prix: number; open: number }>, p: Position): number | null {
  const c = coursDe(live, p)
  if (c === null) return null
  const encaisse = (p.prix_r1 ?? p.r1) > 0 ? ((p.prix_r1 ?? p.r1) - p.entree) * (p.qty_restante ?? p.qty / 2) : 0
  return encaisse + (c - p.entree) * (p.qty_restante ?? p.qty / 2)
}
/// Évolution R du journal : (cours − trailing)/risque + 1 — la marge
/// au-dessus du plancher de sortie.
export function evolutionR(live: Record<string, { prix: number; open: number }>, p: Position): number | null {
  const c = coursDe(live, p)
  const dist = p.entree - p.stop
  if (c === null || !p.trailing || dist <= 0) return null
  return (c - p.trailing) / dist + 1
}
/// Alertes visuelles (lecture) : R1 atteint → pulse verte, invalidation →
/// pulse rouge ; neutralisée : trailing touché → pulse rouge.
export function classeLigne(live: Record<string, { prix: number; open: number }>, p: Position, neutralisee = false): string {
  const c = coursDe(live, p)
  if (neutralisee) {
    return c !== null && p.trailing && c <= p.trailing ? 'animate-pulse-rouge' : ''
  }
  if (c !== null && c <= p.stop) return 'animate-pulse-rouge'
  if (c !== null && c >= p.r1) return 'animate-pulse-vert'
  return ''
}

export function usePositionsRockets() {
  const positions = ref<{ risque: Position[]; neutralisees: Position[]; trailing_pct: number } | null>(null)
  const chargement = ref(true)
  /** Cours live Binance : { SYM: { prix, open } } — poll 5 s. */
  const live = ref<Record<string, { prix: number; open: number }>>({})

  const risque = computed(() => positions.value?.risque ?? [])
  const neutralisees = computed(() => positions.value?.neutralisees ?? [])
  const nb = computed(() => risque.value.length + neutralisees.value.length)

  async function charger() {
    try {
      const res = await http.get('/api/rockets/positions')
      positions.value = res.data
    } catch { /* silencieux */ }
    chargement.value = false
  }

  async function rafraichirLive() {
    const cryptos = [...risque.value, ...neutralisees.value]
      .filter(p => p.univers === 'crypto')
      .map(p => p.symbole)
    if (!cryptos.length) return
    try {
      const q = encodeURIComponent(JSON.stringify(cryptos))
      const res = await fetch(`https://api.binance.com/api/v3/ticker/24hr?symbols=${q}`)
      if (!res.ok) return
      const data: { symbol: string; lastPrice: string; openPrice: string }[] = await res.json()
      const map: Record<string, { prix: number; open: number }> = {}
      for (const t of data) map[t.symbol] = { prix: parseFloat(t.lastPrice), open: parseFloat(t.openPrice) }
      live.value = map
    } catch { /* silencieux */ }
  }

  let pollPositions: ReturnType<typeof setInterval> | null = null
  let pollLive: ReturnType<typeof setInterval> | null = null
  onMounted(async () => {
    await charger()
    void rafraichirLive()
    pollPositions = setInterval(charger, 30_000)
    pollLive = setInterval(rafraichirLive, 5_000)
  })
  onUnmounted(() => {
    if (pollPositions !== null) clearInterval(pollPositions)
    if (pollLive !== null) clearInterval(pollLive)
  })

  return { positions, chargement, live, risque, neutralisees, nb, charger }
}
