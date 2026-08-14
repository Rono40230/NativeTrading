import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import { WS_BASE_URL } from '@/services/http.client'

const NON_CRYPTO_POLL_MS  = 15_000

// Assets non-crypto (métaux, forex, indices) — prix servis par le backend
const NON_CRYPTO_ASSETS = [
  'XAUUSD', 'XAGUSD', 'XPTUSD', 'XPDUSD',
  'EURUSD', 'GBPUSD', 'USDJPY', 'USDCHF', 'AUDUSD', 'USDCAD', 'NZDUSD',
  'GBPJPY', 'CADJPY', 'NZDJPY', 'EURJPY', 'EURGBP',
  'DAX', 'NAS100', 'SP500', 'US30', 'FTSE100', 'CAC40', 'JP225',
]

export interface TickerData {
  prix: number
}

export const usePrixStore = defineStore('prix', () => {
  const tickers       = ref<Record<string, TickerData>>({})
  const variationLive = ref<Record<string, number>>({})   // variation tick-à-tick (flash dashboard)
  const chargement    = ref(false)
  const erreur        = ref(false)

  let actif = false
  let wsCrypto: WebSocket | null = null
  const assetsAbonnes = new Set<string>()
  let intervalNonCrypto: ReturnType<typeof setInterval> | null = null

  // ── WebSocket — prix live pour tous les assets abonnés (via backend) ──
  function _connecterWs() {
    if (wsCrypto) { wsCrypto.close(); wsCrypto = null }
    const assets = [...assetsAbonnes]
    if (assets.length === 0) return
    const url = `${WS_BASE_URL}/api/prix/stream?assets=${assets.join(',')}`
    const ws = new WebSocket(url)

    ws.onmessage = (evt) => {
      try {
        const data = JSON.parse(evt.data) as Record<string, number>
        for (const [ticker, prix] of Object.entries(data)) {
          if (typeof prix === 'number' && prix > 0) {
            const prev = tickers.value[ticker]?.prix
            if (prev && prev > 0) variationLive.value[ticker] = ((prix - prev) / prev) * 100
            tickers.value[ticker] = { prix }
          }
        }
        erreur.value    = false
        chargement.value = false
      } catch { /* message invalide ignoré */ }
    }
    ws.onerror = () => { erreur.value = true }
    ws.onclose = () => {
      setTimeout(() => { if (actif && wsCrypto === ws) _connecterWs() }, 3000)
    }
    wsCrypto = ws
  }

  // ── Fallback REST (15s) — backup si WS inaccessible ──
  async function chargerNonCrypto() {
    try {
      const prixIg = await apiService.getPrixAssets(NON_CRYPTO_ASSETS)
      for (const [ticker, prix] of Object.entries(prixIg)) {
        tickers.value[ticker] = { prix }
      }
    } catch { /* données précédentes conservées */ }
  }

  /** Ajoute des assets à surveiller — démarre le store si pas encore actif, reconnecte le WS */
  function abonner(assets: string[]) {
    let changed = false
    for (const a of assets) { if (!assetsAbonnes.has(a)) { assetsAbonnes.add(a); changed = true } }
    if (!actif) { demarrer(); return }  // demarrer() inclut _connecterWs()
    if (changed) _connecterWs()
  }

  function getPrix(ticker: string): number | null {
    return tickers.value[ticker]?.prix ?? null
  }

  function demarrer(assets: string[] = []) {
    for (const a of assets) assetsAbonnes.add(a)
    if (actif) { if (assets.length > 0) _connecterWs(); return }
    actif = true
    chargement.value = true
    chargerNonCrypto()
    _connecterWs()
    intervalNonCrypto = setInterval(chargerNonCrypto, NON_CRYPTO_POLL_MS)
  }

  function arreter() {
    actif = false
    if (wsCrypto)          { wsCrypto.close(); wsCrypto = null }
    if (intervalNonCrypto) { clearInterval(intervalNonCrypto); intervalNonCrypto = null }
  }

  return { tickers, variationLive, chargement, erreur, getPrix, demarrer, abonner, arreter }
})

