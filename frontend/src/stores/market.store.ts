import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService, type Candle } from '@/services/api.service'

const WS_URL = 'ws://localhost:8080'

export const useMarketStore = defineStore('market', () => {
  const bougies = ref<Record<string, Candle[]>>({})
  const chargement = ref(false)
  const erreur = ref<string | null>(null)
  const wsMiseAJour = ref<{ asset: string; timeframe: string; bougie: Candle } | null>(null)
  let ws: WebSocket | null = null
  const dernierPrix = computed(() => {
    return (asset: string) => {
      const data = bougies.value[asset]
      return data && data.length > 0 ? data[data.length - 1].close : null
    }
  })

  async function chargerBougies(asset: string, timeframe = 'M15', limit = 200) {
    chargement.value = true
    erreur.value = null
    try {
      const data = await apiService.getCandles(asset, timeframe, limit)
      bougies.value[`${asset}_${timeframe}`] = data
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur réseau'
      erreur.value = msg
    } finally {
      chargement.value = false
    }
  }

  function getBougies(asset: string, timeframe = 'M15'): Candle[] {
    return bougies.value[`${asset}_${timeframe}`] ?? []
  }

  function connecterStream(asset: string, timeframe = 'M5') {
    deconnecterStream()
    ws = new WebSocket(`${WS_URL}/api/stream?asset=${asset}&timeframe=${timeframe}`)
    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string)
        if (msg.type !== 'candle' || !msg.data) return
        const key = `${asset}_${timeframe}`
        const liste = bougies.value[key]
        if (!liste || liste.length === 0) return
        // DateTime<Utc> Rust sérialise en ISO string, pas en timestamp numérique
        const rawTs = msg.data.timestamp as string | number
        const timestamp = typeof rawTs === 'string' ? rawTs : new Date(rawTs * 1000).toISOString()
        const nouvelleBougie: Candle = { timestamp, open: msg.data.open, high: msg.data.high, low: msg.data.low, close: msg.data.close, volume: msg.data.volume }
        const tsDerniere = new Date(liste[liste.length - 1].timestamp).getTime()
        const tsNouvelle = new Date(timestamp).getTime()
        if (Math.abs(tsNouvelle - tsDerniere) < 60_000) {
          bougies.value[key] = [...liste.slice(0, -1), nouvelleBougie]
        } else {
          bougies.value[key] = [...liste, nouvelleBougie]
        }
        wsMiseAJour.value = { asset, timeframe, bougie: nouvelleBougie }
      } catch { /* message invalide ignoré */ }
    }
    ws.onerror = () => { erreur.value = 'WebSocket déconnecté' }
  }

  function deconnecterStream() {
    if (ws) { ws.close(); ws = null }
  }

  return { bougies, chargement, erreur, wsMiseAJour, dernierPrix, chargerBougies, getBougies, connecterStream, deconnecterStream }
})
