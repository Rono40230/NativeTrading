<template>
  <div class="glass-bar px-4 py-2.5 flex flex-col gap-2 h-full overflow-y-auto">
    <!-- Meilleure fenêtre du jour -->
    <section class="border-b border-white/5 pb-2 shrink-0 flex flex-col gap-1.5 mt-1">
      <div class="flex items-center justify-between shrink-0">
        <span class="text-xs font-semibold uppercase tracking-widest text-white">⚡ CRÉNEAUX DU JOUR</span>
      </div>
      
      <div v-if="chargementHeatmap && !classementVolatiliteJour.length" class="text-[10px] text-gray-500 italic mt-1">Analyse historique en cours...</div>
      <div v-else-if="!classementVolatiliteJour.length" class="text-[10px] text-gray-500 italic mt-1">Aucune donnée historique trouvée.</div>
      <div v-else class="flex flex-col gap-1 mt-1">
        <div v-for="(res, idx) in classementVolatiliteJour" :key="res.asset" class="flex items-center justify-between text-[10px] bg-orange-900/10 border border-orange-500/10 px-2 py-1 rounded">
          <span class="text-gray-400">
            <span v-if="idx === 0">🔥</span><span v-else class="opacity-50">#{{ idx + 1 }}</span>
            <span class="text-white font-medium ml-1">{{ res.asset }}</span> : 
            <span class="text-orange-400 font-bold ml-0.5">{{ res.heuresFormatees }}</span>
          </span>
          <span :class="res.maxCluster === 3 ? 'text-red-400 font-semibold' : res.maxCluster === 2 ? 'text-orange-400 font-semibold' : res.maxCluster === 1 ? 'text-yellow-500' : 'text-gray-500'">
            {{ NOM_CLUSTER[res.maxCluster] || 'Niv. '+res.maxCluster }}
          </span>
        </div>
      </div>
      
      <div v-if="fenDujour" class="flex items-center gap-2 text-[11px] mt-1 pt-1 border-t border-white/5">
        <span class="text-gray-400">Créneau programmé :</span>
        <span class="font-bold text-white">{{ fenDujour.asset }}</span>
        <span class="text-gray-500">{{ fenDujour.heure_debut }}–{{ fenDujour.heure_fin }} Paris</span>
      </div>
    </section>

    <!-- Prochains créneaux -->
    <div v-if="chargement" class="text-[10px] text-gray-600 animate-pulse">Chargement…</div>
    <div v-else-if="!prochainsList.length" class="text-[11px] text-gray-500 italic">Aucun créneau validé</div>
    <div v-else class="flex flex-col gap-1.5 overflow-y-auto flex-1 min-h-0">
      <div v-for="c in prochainsList" :key="c.id"
        class="flex flex-col gap-0.5 border-t border-white/5 pt-1 first:border-0 first:pt-0">
        <div class="flex items-center gap-2 text-[11px]">
          <span class="font-bold text-white">{{ c.asset }}</span>
          <span class="text-gray-600">{{ c.heure_debut }}–{{ c.heure_fin }}</span>
          <span class="font-mono font-bold ml-auto text-[10px]" :class="reboursCls(c)">{{ rebours(c) }}</span>
        </div>
        <div class="flex items-center gap-2 text-[10px] text-gray-500 flex-wrap">
          <span v-if="c.whipsaw_minutes" class="text-orange-400 ml-auto">⚠ ws {{ c.whipsaw_minutes }}min</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { apiService } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'
import type { StraddleCreneau, ReponsePatternsVolatilite, PatternHoraire } from '@/services/api.types'

const JOURS = ['Dim', 'Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam']

const assetsStore = useAssetsStore()
const creneaux = ref<StraddleCreneau[]>([])
const chargement = ref(true)
const now = ref(Date.now())

const jourUTCAujourdhui = computed(() => new Date().getUTCDay())
const jourLabel = computed(() => JOURS[jourUTCAujourdhui.value] ?? '')

const fenDujour = computed<StraddleCreneau | null>(() => {
  const actifs = new Set(assetsStore.assets.map(a => a.id))
  const d = jourUTCAujourdhui.value
  const candidats = creneaux.value.filter(c =>
    c.statut === 'valide' && actifs.has(c.asset) && c.jour_semaine === d
  )
  if (!candidats.length) return null
  return candidats.sort((a, b) =>
    (b.llm_conviction ?? 0) - (a.llm_conviction ?? 0)
  )[0]
})

const reponsesHeatmap = ref<Record<string, ReponsePatternsVolatilite>>({})
const chargementHeatmap = ref(false)

watch(() => assetsStore.assets.filter(a => a.actif).map(a => a.id), async (actifs) => {
  if (!actifs.length) return
  chargementHeatmap.value = true
  
  await Promise.allSettled(actifs.map(async (asset) => {
    if (!reponsesHeatmap.value[asset]) {
      try {
        reponsesHeatmap.value[asset] = await apiService.obtenirPatternsVolatilite(asset, 'M15', 12)
      } catch (e) {
        console.error(`Erreur chargement heatmap ${asset}`, e)
      }
    }
  }))
  
  chargementHeatmap.value = false
}, { immediate: true, deep: true })

function decalageParis(): 1 | 2 {
  const maintenant = new Date()
  const hParis = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(maintenant))
  const hUtc = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'UTC', hour: 'numeric', hour12: false }).format(maintenant))
  return ((hParis - hUtc + 24) % 24) === 2 ? 2 : 1
}

const DECALAGE_PARIS = decalageParis()

function heureParis(heureUtc: number): number {
  return (heureUtc + DECALAGE_PARIS) % 24
}

function formaterHeures(heuresUtc: number[]): string {
  if (!heuresUtc.length) return ''
  const heuresFormatteesParis = heuresUtc.map(h => heureParis(h)).sort((a, b) => a - b)
  
  const blocs: string[] = []
  let debut = heuresFormatteesParis[0]
  let fin = heuresFormatteesParis[0]

  for (let i = 1; i < heuresFormatteesParis.length; i++) {
    if (heuresFormatteesParis[i] === fin + 1) {
      fin = heuresFormatteesParis[i]
    } else {
      blocs.push(debut === fin ? `${debut}h` : `${debut}h-${fin}h`)
      debut = heuresFormatteesParis[i]
      fin = heuresFormatteesParis[i]
    }
  }
  blocs.push(debut === fin ? `${debut}h` : `${debut}h-${fin}h`)
  return blocs.join(', ') + ' Paris'
}

type ClassementJour = {
  asset: string
  maxCluster: number
  heuresFormatees: string
}

const NOM_CLUSTER = ['Calme', 'Modéré', 'Élevé', 'Extrême']

const classementVolatiliteJour = computed<ClassementJour[]>(() => {
  const d = jourUTCAujourdhui.value
  const actifs = assetsStore.assets.filter(a => a.actif).map(a => a.id)
  const resultats: ClassementJour[] = []

  for (const asset of actifs) {
    const rep = reponsesHeatmap.value[asset]
    if (!rep) continue
    const pts = rep.patterns.filter(p => p.jour_semaine === d && p.nb_points > 0)
    if (!pts.length) continue

    const topPts = [...pts].sort((a, b) => b.cluster - a.cluster || b.atr_moyen - a.atr_moyen)
    const maxCluster = topPts[0].cluster

    // On conserve les 4 meilleures heures (départagées par l'ATR max pour éviter d'afficher 0h-23h)
    const meilleuresHeures = topPts
      .slice(0, 4)
      .map(p => p.heure)

    resultats.push({
      asset,
      maxCluster,
      heuresFormatees: formaterHeures(meilleuresHeures)
    })
  }

  // Tri par intensité du cluster maximal, puis par nom d'actif
  return resultats.sort((a, b) => b.maxCluster - a.maxCluster || a.asset.localeCompare(b.asset))
})

function secondesAvant(c: StraddleCreneau): number {
  const [hd, md] = c.heure_debut.split(':').map(Number)
  const base = new Date()
  base.setUTCHours(hd, md ?? 0, 0, 0)
  if (c.jour_semaine !== null) {
    const jourAujourdhuiUTC = new Date().getUTCDay()
    let delta = (c.jour_semaine - jourAujourdhuiUTC + 7) % 7
    if (delta === 0 && base.getTime() <= Date.now()) delta = 7
    base.setUTCDate(base.getUTCDate() + delta)
  } else if (base.getTime() <= Date.now()) {
    base.setUTCDate(base.getUTCDate() + 1)
  }
  return Math.max(0, Math.floor((base.getTime() - Date.now()) / 1000))
}

const prochainsList = computed<StraddleCreneau[]>(() => {
  void now.value
  const actifs = new Set(assetsStore.assets.map(a => a.id))
  return creneaux.value
    .filter(c => c.statut === 'valide' && actifs.has(c.asset))
    .sort((a, b) => secondesAvant(a) - secondesAvant(b))
    .slice(0, 3)
})

function rebours(c: StraddleCreneau): string {
  const s = secondesAvant(c)
  if (s === 0) return 'Maintenant !'
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  if (h > 0) return `${h}h ${String(m).padStart(2, '0')}m`
  if (m > 0) return `${m}m ${String(sec).padStart(2, '0')}s`
  return `${sec}s`
}

function reboursCls(c: StraddleCreneau): string {
  const s = secondesAvant(c)
  if (s === 0) return 'text-emerald-400 animate-pulse'
  if (s < 300) return 'text-red-400'
  if (s < 1800) return 'text-yellow-400'
  return 'text-blue-300'
}

async function charger() {
  try { creneaux.value = await apiService.getStraddleCreneaux() }
  catch { creneaux.value = [] }
  finally { chargement.value = false }
}

let _tick: ReturnType<typeof setInterval> | null = null
let _poll: ReturnType<typeof setInterval> | null = null
onMounted(async () => {
  if (!assetsStore.assets.length) await assetsStore.chargerAssets()
  charger()
  _tick = setInterval(() => { now.value = Date.now() }, 1000)
  _poll = setInterval(charger, 5 * 60_000)
})
onUnmounted(() => {
  if (_tick !== null) { clearInterval(_tick); _tick = null }
  if (_poll !== null) { clearInterval(_poll); _poll = null }
})
</script>

<style scoped>
.glass-bar {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
