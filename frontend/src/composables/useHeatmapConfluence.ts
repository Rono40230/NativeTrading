import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import type { PatternHoraire } from '@/services/api.service'

export interface ConfluenceItem {
  asset: string
  tf: string
  atrRatio: number
  cluster: string
  clusterIndex: number
}

const NOM_CLUSTER = ['Calme', 'Modéré', 'Élevé', 'Extrême'] as const

// Cache : évite de refetch les patterns à chaque tick de 60s
const cachePatterns = new Map<string, { data: PatternHoraire[]; ts: number }>()
const TTL_MS = 5 * 60 * 1000 // 5 minutes

async function fetchPatternsCaches(asset: string, tf: string): Promise<PatternHoraire[]> {
  const key = `${asset}_${tf}`
  const cached = cachePatterns.get(key)
  if (cached && Date.now() - cached.ts < TTL_MS) return cached.data
  try {
    const reponse = await apiService.obtenirPatternsVolatilite(asset, tf, 12)
    cachePatterns.set(key, { data: reponse.patterns, ts: Date.now() })
    return reponse.patterns
  } catch {
    return []
  }
}

export function useHeatmapConfluence() {
  const confluences = ref<ConfluenceItem[]>([])
  const chargementConfluence = ref(false)

  async function detecterConfluences(
    classementVol: { asset: string; tf: string; atr: number }[]
  ) {
    const rouges = classementVol.filter(i => i.atr > 120)
    if (!rouges.length) { confluences.value = []; return }

    // Par asset : garder le TF avec le ratio ATR le plus élevé
    const parAsset = new Map<string, typeof rouges[0]>()
    for (const r of rouges) {
      const ex = parAsset.get(r.asset)
      if (!ex || r.atr > ex.atr) parAsset.set(r.asset, r)
    }

    chargementConfluence.value = true
    const now = new Date()
    const heureUtc = now.getUTCHours()
    const jourSemaine = now.getUTCDay()

    const resultats: ConfluenceItem[] = []
    await Promise.allSettled(
      [...parAsset.values()].map(async item => {
        const patterns = await fetchPatternsCaches(item.asset, item.tf)
        const p = patterns.find(x => x.heure === heureUtc && x.jour_semaine === jourSemaine)
        if (p && p.cluster >= 2) {
          resultats.push({
            asset: item.asset,
            tf: item.tf,
            atrRatio: item.atr,
            cluster: NOM_CLUSTER[p.cluster] ?? 'Élevé',
            clusterIndex: p.cluster,
          })
        }
      })
    )

    // Trier par cluster desc puis atr desc
    resultats.sort((a, b) => b.clusterIndex - a.clusterIndex || b.atrRatio - a.atrRatio)
    confluences.value = resultats
    chargementConfluence.value = false
  }

  return { confluences, chargementConfluence, detecterConfluences }
}
