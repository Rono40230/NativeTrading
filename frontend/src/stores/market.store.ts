import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiService, type Candle } from '@/services/api.service'
import { WS_BASE_URL } from '@/services/http.client'

const WS_URL = WS_BASE_URL

export const useMarketStore = defineStore('market', () => {
  const bougies = ref<Record<string, Candle[]>>({})
  const chargement = ref(false)
  const erreur = ref<string | null>(null)
  const erreurWs = ref<string | null>(null)
  const wsMiseAJour = ref<{ asset: string; timeframe: string; bougie: Candle; estNouvelle: boolean } | null>(null)
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

    // WebSocket pour tous les assets : crypto (Binance) et métaux/forex/indices (IG Markets)
    ws = new WebSocket(`${WS_URL}/api/stream?asset=${asset}&timeframe=${timeframe}`)
    ws.onopen = () => { wsConnecte.value = true }
    ws.onclose = () => { wsConnecte.value = false }

    // Buffer pour les bougies historiques reçues en batch avant les updates temps réel
    let historiqueBuffer: Candle[] = []
    let enModeHistorique = false

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data as string)
        const key = `${asset}_${timeframe}`

        // Début du batch historique : basculer en mode buffer
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
          erreurWs.value = msg.message ?? 'Erreur flux IG Markets'
          return
        }

        // Prix live 5s (IG polling) — met à jour la dernière bougie sans modifier le chart
        if (msg.type === 'price' && msg.price != null) {
          const liste = bougies.value[key]
          if (liste && liste.length > 0) {
            const derniere = liste[liste.length - 1]
            liste[liste.length - 1] = { ...derniere, close: msg.price, high: Math.max(derniere.high, msg.price), low: Math.min(derniere.low, msg.price) }
            wsMiseAJour.value = { asset, timeframe, bougie: liste[liste.length - 1], estNouvelle: false }
          }
          return
        }

        if ((msg.type !== 'candle' && msg.type !== 'bar_update') || !msg.data) return

        // Convertir en ms depuis epoch
        const rawTs = msg.data.timestamp as string | number
        const rawMs = typeof rawTs === 'string' ? new Date(rawTs).getTime() : rawTs * 1000
        // Arrondir à l'ouverture de la barre (UTM = heure courante ≠ open time de la barre)
        // Sans ça, chaque tick crée une nouvelle mini-barre au lieu de mettre à jour la barre courante
        const tf_ms = dureeMs(timeframe)
        const barOpenMs = Math.floor(rawMs / tf_ms) * tf_ms
        const timestamp = new Date(barOpenMs).toISOString()
        const nouvelleBougie: Candle = { timestamp, open: msg.data.open, high: msg.data.high, low: msg.data.low, close: msg.data.close, volume: msg.data.volume }

        // Phase historique : accumuler dans le buffer
        if (enModeHistorique) {
          historiqueBuffer.push(nouvelleBougie)
          return
        }

        // Phase temps réel : mise à jour incrémentale
        const liste = bougies.value[key]
        if (!liste || liste.length === 0) {
          // Aucune donnée historique — initialiser avec cette bougie
          bougies.value[key] = [nouvelleBougie]
          wsMiseAJour.value = { asset, timeframe, bougie: nouvelleBougie, estNouvelle: true }
          return
        }
        const tsDerniere = new Date(liste[liste.length - 1].timestamp).getTime()
        const tsNouvelle = new Date(timestamp).getTime()
        const duree = dureeMs(timeframe)
        // Si timestamp identique ou dans la même barre → mise à jour de la dernière bougie
        if (tsNouvelle <= tsDerniere || tsNouvelle - tsDerniere < duree) {
          // Conserver les valeurs OHLC correctes : ne pas écraser si bougie live sans historique
          liste[liste.length - 1] = nouvelleBougie
          wsMiseAJour.value = { asset, timeframe, bougie: nouvelleBougie, estNouvelle: false }
        } else {
          liste.push(nouvelleBougie)
          wsMiseAJour.value = { asset, timeframe, bougie: nouvelleBougie, estNouvelle: true }
        }
      } catch { /* message invalide ignoré */ }
    }
    ws.onerror = () => { erreurWs.value = 'WebSocket déconnecté'; wsConnecte.value = false }
  }

  function deconnecterStream() {
    if (ws) { ws.close(); ws = null }
    wsConnecte.value = false
    erreurWs.value = null
  }

  // ─── fin du store ────────────────────────────────────────────────────────────
  return { bougies, chargement, erreur, erreurWs, wsMiseAJour, wsConnecte, dernierPrix, chargerBougies, getBougies, connecterStream, deconnecterStream }
})
