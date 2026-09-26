import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { Candle, ReponsePatternsVolatilite } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'

/// Radar ATR temps réel — source UNIQUE (extrait du bloc 26/09) : le
/// bandeau (badge top-5, cycle 1 h) et le bloc repliable (cycle 60 s)
/// partagent exactement le même calcul. Volatilité actuelle vs HABITUDE
/// DE L'INSTANT : le ratio ATR(6)/ATR(60) brut est divisé par le facteur
/// saisonnier jour × heure de l'asset (patterns 24 mois, M1 — même source
/// que les Créneaux). 100 % = conforme à l'habitude de CET instant.
/// H4 et + : l'heure n'a pas de sens (fenêtres ≥ 24 h) → ratio brut.
const TIMEFRAMES = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']
const INTRADAY = new Set(['M1', 'M5', 'M15', 'M30', 'H1'])

export interface LigneRadar { cle: string; asset: string; tf: string; atr: number; brut: number }

/// Baselines saisonnières par asset : ATR moyen par cellule jour × heure
/// (UTC — sémantique des patterns), moyenne globale pondérée, repli
/// heure-semaine puis moyenne si la cellule est absente.
interface BaseAsset { moyenne: number; parJourHeure: Map<string, number>; parHeure: Map<number, number> }

function calcAtr(candles: Candle[], periode = 14): number {
  if (candles.length < 2) return 0
  const trs = candles.slice(1).map((c, i) => {
    const prev = candles[i].close
    return Math.max(c.high - c.low, Math.abs(c.high - prev), Math.abs(c.low - prev))
  })
  const fenetre = trs.slice(-Math.min(periode, trs.length))
  return fenetre.reduce((s, v) => s + v, 0) / fenetre.length
}

/// Ratio brut (ATR 6 dernières / ATR 60 dernières, ×100).
function calcAtrRatio(candles: Candle[]): number {
  if (candles.length < 30) return 0
  const atrActuel = calcAtr(candles.slice(-7), 6)
  const atrMoyen = calcAtr(candles, Math.min(candles.length - 1, 60))
  return atrMoyen > 0 ? (atrActuel / atrMoyen) * 100 : 100
}

export function useRadarAtr() {
  const assetsStore = useAssetsStore()
  const chargement = ref(false)
  const donnees = ref<Record<string, LigneRadar>>({})
  const baselines = ref<Map<string, BaseAsset>>(new Map())
  const assets = computed(() => assetsStore.assets.map(a => a.id))

  function construireBaselines(reponses: ReponsePatternsVolatilite[]) {
    const m = new Map<string, BaseAsset>()
    for (const r of reponses) {
      let sp = 0, sn = 0
      const parJourHeure = new Map<string, number>()
      const accHeure = new Map<number, { somme: number; n: number }>()
      for (const p of r.patterns) {
        if (p.nb_points <= 0 || p.atr_moyen <= 0) continue
        sp += p.atr_moyen * p.nb_points
        sn += p.nb_points
        parJourHeure.set(`${p.jour_semaine}_${p.heure}`, p.atr_moyen)
        const e = accHeure.get(p.heure) ?? { somme: 0, n: 0 }
        e.somme += p.atr_moyen * p.nb_points
        e.n += p.nb_points
        accHeure.set(p.heure, e)
      }
      if (sn > 0) {
        const parHeure = new Map<number, number>()
        for (const [h, e] of accHeure) parHeure.set(h, e.somme / e.n)
        m.set(r.asset, { moyenne: sp / sn, parJourHeure, parHeure })
      }
    }
    baselines.value = m
  }

  /// Facteur saisonnier de l'instant : 1,15 = cet asset est habituellement
  /// 15 % plus volatile à ce jour × heure que sa moyenne globale.
  function facteurSaisonnier(asset: string): number {
    const b = baselines.value.get(asset)
    if (!b || b.moyenne <= 0) return 1
    const now = new Date()
    const base = b.parJourHeure.get(`${now.getUTCDay()}_${now.getUTCHours()}`)
      ?? b.parHeure.get(now.getUTCHours())
      ?? b.moyenne
    return base / b.moyenne
  }

  const classement = computed(() =>
    Object.values(donnees.value).filter(i => i.atr > 0).sort((a, b) => b.atr - a.atr)
  )

  /// Anomalies saisonnières : intraday seulement, ≥ 120 % de l'habitude.
  const confluences = computed(() =>
    classement.value.filter(i => i.atr >= 120 && INTRADAY.has(i.tf)).slice(0, 6)
  )

  async function actualiser() {
    chargement.value = true
    // Baselines saisonnières (cache serveur 1 h — une requête par cycle).
    try {
      construireBaselines(await apiService.obtenirPatternsJourTousActifs())
    } catch {
      baselines.value = new Map()
    }
    if (!assets.value.length) await assetsStore.chargerAssets().catch(() => null)
    const paires = assets.value.flatMap(a => TIMEFRAMES.map(tf => ({ a, tf })))
    const resultats = await Promise.allSettled(
      paires.map(({ a, tf }) => apiService.getCandles(a, tf, 80).then(c => ({ a, tf, c })))
    )
    for (const r of resultats) {
      if (r.status === 'fulfilled') {
        const { a, tf, c } = r.value
        const brut = calcAtrRatio(c)
        if (brut <= 0) continue
        const facteur = facteurSaisonnier(a)
        donnees.value[`${a}_${tf}`] = {
          cle: `${a}_${tf}`, asset: a, tf,
          atr: INTRADAY.has(tf) ? brut / facteur : brut,
          brut,
        }
      }
    }
    chargement.value = false
  }

  /// Cycle périodique piloté par l'appelant (badge bandeau : 1 h —
  /// bloc : 60 s à l'ouverture). Retourne la fonction d'arrêt.
  function demarrer(periodMs: number): () => void {
    void actualiser()
    const id = setInterval(actualiser, periodMs)
    return () => clearInterval(id)
  }

  return { classement, confluences, chargement, actualiser, demarrer }
}
