import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService, type Candle } from '@/services/api.service'

const WS_URL = 'ws://localhost:8080'

export const useMarketStore = defineStore('market', () => {
  const bougies = ref<Record<string, Candle[]>>({})
  const chargement = ref(false)
  const erreur = ref<string | null>(null)
  const erreurWs = ref<string | null>(null)
  const wsMiseAJour = ref<{ asset: string; timeframe: string; bougie: Candle } | null>(null)
  const wsConnecte = ref(false)
  let ws: WebSocket | null = null
  const dernierPrix = computed(() => {
    return (asset: string) => {
      const data = bougies.value[asset]
      return data && data.length > 0 ? data[data.length - 1].close : null
    }
  })

  async function chargerBougies(asset: string, timeframe = 'M15', limit = 200, force = false) {
    chargement.value = true
    erreur.value = null
    try {
      const data = await apiService.getCandles(asset, timeframe, limit, force)
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

  /** Durée d'une bougie en ms selon le timeframe */
  function dureeMs(tf: string): number {
    const map: Record<string, number> = {
      M1: 60_000, M5: 300_000, M15: 900_000, M30: 1_800_000,
      H1: 3_600_000, H4: 14_400_000, D1: 86_400_000, W1: 604_800_000,
    }
    return map[tf] ?? 60_000
  }

  async function connecterStream(asset: string, timeframe = 'M5') {
    deconnecterStream()

    // WebSocket pour tous les assets : crypto (Binance) et métaux (IB Gateway historical_data_streaming)
    ws = new WebSocket(`${WS_URL}/api/stream?asset=${asset}&timeframe=${timeframe}`)
    ws.onopen = () => { wsConnecte.value = true }
    ws.onclose = () => { wsConnecte.value = false }

    // Buffer pour les bougies historiques IB reçues en batch avant les updates temps réel
    let historiqueBuffer: Candle[] = []
    let enModeHistorique = false

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string)
        const key = `${asset}_${timeframe}`

        // Début du batch historique IB : basculer en mode buffer
        if (msg.type === 'historical_start') {
          enModeHistorique = true
          historiqueBuffer = []
          return
        }

        // Fin du batch : trier et remplacer les données du store
        if (msg.type === 'historical_end') {
          enModeHistorique = false
          const triees = historiqueBuffer.sort((a, b) =>
            new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
          )
          if (triees.length > 0) bougies.value[key] = triees
          historiqueBuffer = []
          return
        }

        if (msg.type === 'error') {
          erreurWs.value = msg.message ?? 'Erreur flux IB Gateway'
          return
        }

        // Prix live 5s (IB realtime_bars) — met à jour la dernière bougie sans modifier le chart
        if (msg.type === 'price' && msg.price != null) {
          const liste = bougies.value[key]
          if (liste && liste.length > 0) {
            const derniere = liste[liste.length - 1]
            liste[liste.length - 1] = { ...derniere, close: msg.price, high: Math.max(derniere.high, msg.price), low: Math.min(derniere.low, msg.price) }
            wsMiseAJour.value = { asset, timeframe, bougie: liste[liste.length - 1] }
          }
          return
        }

        if (msg.type !== 'candle' || !msg.data) return

        // DateTime<Utc> Rust sérialise en ISO string, pas en timestamp numérique
        const rawTs = msg.data.timestamp as string | number
        const timestamp = typeof rawTs === 'string' ? rawTs : new Date(rawTs * 1000).toISOString()
        const nouvelleBougie: Candle = { timestamp, open: msg.data.open, high: msg.data.high, low: msg.data.low, close: msg.data.close, volume: msg.data.volume }

        // Phase historique : accumuler dans le buffer
        if (enModeHistorique) {
          historiqueBuffer.push(nouvelleBougie)
          return
        }

        // Phase temps réel : mise à jour incrémentale
        const liste = bougies.value[key]
        if (!liste || liste.length === 0) return
        const tsDerniere = new Date(liste[liste.length - 1].timestamp).getTime()
        const tsNouvelle = new Date(timestamp).getTime()
        const duree = dureeMs(timeframe)
        if (tsNouvelle - tsDerniere < duree) {
          // Même bougie — mutation in-place (ne déclenche pas le watcher shallow)
          liste[liste.length - 1] = nouvelleBougie
        } else {
          // Nouvelle bougie — push in-place
          liste.push(nouvelleBougie)
        }
        wsMiseAJour.value = { asset, timeframe, bougie: nouvelleBougie }
      } catch { /* message invalide ignoré */ }
    }
    ws.onerror = () => { erreurWs.value = 'WebSocket déconnecté'; wsConnecte.value = false }
  }

  function deconnecterStream() {
    if (ws) { ws.close(); ws = null }
    wsConnecte.value = false
    erreurWs.value = null
  }

  // ─── Prix live ticker dashboard (WS par asset : Binance pour crypto, IB pour le reste) ──────────
  const CRYPTO_ASSETS = new Set(['BTC', 'ETH'])
  const prixLive = ref<Record<string, number>>({})
  const variationLive = ref<Record<string, number>>({})
  const wsLiveMap: Record<string, WebSocket> = {}

  function connecterPrixLiveAssets(assets: string[]) {
    assets.forEach(asset => {
      if (wsLiveMap[asset]) return
      // M1 pour crypto (Binance, 24/7), M1 pour IB (métaux/forex/indices — markets hours)
      const tf = CRYPTO_ASSETS.has(asset) ? 'M1' : 'M1'
      const liveWs = new WebSocket(`${WS_URL}/api/stream?asset=${asset}&timeframe=${tf}`)
      wsLiveMap[asset] = liveWs

      liveWs.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data as string)
          // Binance envoie type:"candle", IB envoie type:"price" (tick bid/ask) ou type:"candle"
          if (msg.type === 'price' && msg.price != null) {
            prixLive.value[asset] = msg.price
          } else if (msg.type === 'candle' && msg.data) {
            prixLive.value[asset] = msg.data.close
            if (msg.data.open > 0) {
              variationLive.value[asset] = ((msg.data.close - msg.data.open) / msg.data.open) * 100
            }
          }
        } catch { /* message invalide ignoré */ }
      }
      liveWs.onclose = () => { delete wsLiveMap[asset] }
      // Si IB offline → fermeture silencieuse, fallback REST géré par le dashboard
      liveWs.onerror = () => { delete wsLiveMap[asset] }
    })
  }

  function deconnecterPrixLiveAssets() {
    Object.values(wsLiveMap).forEach(liveWs => liveWs.close())
    Object.keys(wsLiveMap).forEach(k => delete wsLiveMap[k])
  }

  return { bougies, chargement, erreur, erreurWs, wsMiseAJour, wsConnecte, dernierPrix, prixLive, variationLive, chargerBougies, getBougies, connecterStream, deconnecterStream, connecterPrixLiveAssets, deconnecterPrixLiveAssets }
})
