<template>
  <div class="space-y-5">
    <!-- Contrôles -->
    <div class="flex flex-wrap items-center gap-3">
      <select v-model="asset" class="glass-select" @change="charger">
        <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
      </select>
      <select v-model="timeframe" class="glass-select" @change="charger">
        <option v-for="tf in timeframes" :key="tf" :value="tf">{{ tf }}</option>
      </select>
      <select v-model="mois" class="glass-select" @change="charger">
        <option v-for="m in periodesDisponibles" :key="m" :value="m">{{ m }} mois</option>
      </select>
      <button class="btn-sm" :disabled="chargement" @click="charger">
        {{ chargement ? '⏳' : '🔄' }} Charger
      </button>
      <span v-if="reponse" class="text-xs text-gray-400 ml-auto">
        {{ reponse.nb_points_total.toLocaleString('fr-FR') }} bougies analysées
      </span>
    </div>

    <!-- Seuil Straddle calibré -->
    <div v-if="reponse" class="glass-card p-4 flex flex-wrap items-center gap-4">
      <div>
        <p class="text-xs text-gray-400 mb-0.5">Seuil Straddle calibré (P85)</p>
        <p class="text-lg font-bold text-yellow-400">{{ reponse.seuil_straddle_calibre.toFixed(1) }}</p>
      </div>
      <p class="text-xs text-gray-500 max-w-sm">
        ATR moyen au 85ème percentile sur l'historique. Le Straddle se déclenche quand l'ATR courant dépasse ce seuil.
      </p>
    </div>

    <!-- Légende clusters -->
    <div class="flex items-center gap-4 flex-wrap">
      <span v-for="c in clusters" :key="c.label" class="flex items-center gap-1.5 text-xs text-gray-300">
        <span class="w-3.5 h-3.5 rounded-sm" :style="{ background: c.couleur }" />
        {{ c.label }}
      </span>
    </div>

    <!-- Heatmap 24h × 7j -->
    <div v-if="reponse?.patterns.length" class="glass-card p-4 overflow-x-auto">
      <table class="w-full table-fixed text-xs border-separate border-spacing-0.5">
        <thead>
          <tr>
            <th class="text-gray-500 text-left px-1 pb-1 whitespace-nowrap w-10">UTC</th>
            <th
              v-for="h in heures"
              :key="h"
              class="text-gray-400 pb-1 text-center font-mono"
            >{{ h }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="j in jours" :key="j.index">
            <td class="text-gray-400 pr-3 py-0.5 whitespace-nowrap font-medium">{{ j.label }}</td>
            <td v-for="h in heures" :key="h" class="p-0">
              <div
                class="w-full h-8 rounded flex items-center justify-center cursor-default transition-transform hover:scale-110"
                :style="celluleStyle(h, j.index)"
                @mouseenter="(e) => afficherTooltip(e, h, j.index)"
                @mouseleave="masquerTooltip"
              >
                <span v-if="cellulePoints(h, j.index) > 0" class="text-[10px] text-white/80 font-mono leading-none">
                  {{ celluleAtr(h, j.index).toFixed(1) }} <span>{{ unite }}</span>
                </span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- État vide -->
    <div v-else-if="!chargement" class="glass-card p-8 text-center text-gray-500 text-sm">
      Sélectionnez un asset et un timeframe puis cliquez sur Charger.
    </div>

    <!-- Bloc d'analyse statistique -->
    <div v-if="analyse" class="grid grid-cols-1 md:grid-cols-3 gap-4">

      <!-- Meilleures fenêtres -->
      <div class="glass-card p-4 space-y-3">
        <p class="text-xs font-semibold text-emerald-400 uppercase tracking-wider">Meilleures fenêtres de trading</p>
        <div v-for="f in analyse.top3" :key="f.heureUtc" class="flex items-center justify-between">
          <span class="text-sm text-white font-mono">{{ f.heureParis }}h – {{ (f.heureParis + 1) % 24 }}h Paris</span>
          <span :class="COULEUR_CLUSTER_TEXTE[f.cluster]" class="text-xs font-medium">{{ NOM_CLUSTER[f.cluster] }}</span>
        </div>
        <p class="text-[10px] text-gray-500 pt-1">Cluster dominant sur l'ensemble de la semaine</p>
      </div>

      <!-- Heures à éviter -->
      <div class="glass-card p-4 space-y-3">
        <p class="text-xs font-semibold text-red-400 uppercase tracking-wider">Fenêtres à éviter</p>
        <div v-for="f in analyse.pires3" :key="f.heureUtc" class="flex items-center justify-between">
          <span class="text-sm text-white font-mono">{{ f.heureParis }}h – {{ (f.heureParis + 1) % 24 }}h Paris</span>
          <span :class="COULEUR_CLUSTER_TEXTE[f.cluster]" class="text-xs font-medium">{{ NOM_CLUSTER[f.cluster] }}</span>
        </div>
        <p class="text-[10px] text-gray-500 pt-1">Faible volatilité — spread défavorable</p>
      </div>

      <!-- Jours + heure actuelle -->
      <div class="glass-card p-4 flex flex-col gap-4 items-start">
        <div class="flex-1 min-w-[160px] space-y-1">
          <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Jours de la semaine</p>
          <div class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
            <span class="text-sm text-gray-300">{{ analyse.meilleurJour.label }} — jour le plus actif</span>
          </div>
          <div class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-red-400"></span>
            <span class="text-sm text-gray-300">{{ analyse.pireJour.label }} — jour le plus calme</span>
          </div>
        </div>
        <div class="flex-1 min-w-[200px] space-y-1">
          <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Maintenant — {{ analyse.hParisActuelle }}h Paris</p>
          <template v-if="analyse.patternActuel">
            <p :class="COULEUR_CLUSTER_TEXTE[analyse.patternActuel.cluster]" class="text-sm font-semibold">
              {{ NOM_CLUSTER[analyse.patternActuel.cluster] }} — ATR moyen {{ analyse.patternActuel.atr_moyen.toFixed(1) }}
            </p>
            <p class="text-xs text-gray-500">{{ analyse.patternActuel.cluster >= 2 ? 'Fenêtre favorable au trading actif.' : 'Attendre une fenêtre plus volatile.' }}</p>
          </template>
          <p v-else class="text-xs text-gray-500">Pas de données pour ce créneau.</p>
        </div>
        <p class="w-full text-[10px] text-gray-600 mt-1">Basé sur l'historique — pas une garantie de performance future.</p>
      </div>
    </div>

    <!-- Tooltip cellule heatmap -->
    <Teleport v-if="tooltipVisible" to="body">
      <div
        class="fixed z-[9999] px-3 py-2 text-xs text-gray-200 bg-gray-950 border border-white/10 rounded-lg shadow-2xl pointer-events-none whitespace-nowrap"
        :style="{ top: `${tooltipPos.top}px`, left: `${tooltipPos.left}px`, transform: 'translate(-50%, calc(-100% - 8px))' }"
      >{{ tooltipTexte }}</div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { JOURS as jours, CLUSTERS as clusters, COULEURS_CLUSTER, COULEURS_CLUSTER_PLEIN } from './heatmapConstants'
import { apiService } from '@/services/api.service'
import type { ReponsePatternsVolatilite, AssetInfo } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

const alerteStore = useAlerteStore()
const assetsInfos = ref<AssetInfo[]>([])
const assets = computed(() => assetsInfos.value.map(a => a.id))
const timeframes = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']
const periodesDisponibles = [6, 12, 18, 24]
const asset = ref('BTC')
const timeframe = ref('M15')
const mois = ref(12)
const chargement = ref(false)
const reponse = ref<ReponsePatternsVolatilite | null>(null)

/** Unité selon le type d'asset sélectionné. */
const unite = computed(() => {
  const info = assetsInfos.value.find(a => a.id === asset.value)
  return info?.type === 'crypto' ? '$' : 'pts'
})

const heures = Array.from({ length: 24 }, (_, i) => i)

const tooltipVisible = ref(false)
const tooltipTexte = ref('')
const tooltipPos = ref({ top: 0, left: 0 })

function afficherTooltip(e: MouseEvent, heure: number, jour: number) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  tooltipPos.value = { top: rect.top, left: rect.left + rect.width / 2 }
  tooltipTexte.value = celluleTitre(heure, jour)
  tooltipVisible.value = true
}
function masquerTooltip() {
  tooltipVisible.value = false
}

/** Décalage UTC→Paris actuel : +2 en été (CEST), +1 en hiver (CET). */
function decalageParis(): 1 | 2 {
  const maintenant = new Date()
  const hParis = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(maintenant))
  const hUtc = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'UTC', hour: 'numeric', hour12: false }).format(maintenant))
  return ((hParis - hUtc + 24) % 24) === 2 ? 2 : 1
}

const DECALAGE_PARIS = decalageParis()
const ZONE_PARIS = DECALAGE_PARIS === 2 ? 'CEST' : 'CET'

function heureParis(heureUtc: number): number {
  return (heureUtc + DECALAGE_PARIS) % 24
}

function trouverPattern(heure: number, jour: number) {
  return reponse.value?.patterns.find((p) => p.heure === heure && p.jour_semaine === jour)
}

function celluleAtr(heure: number, jour: number): number {
  return trouverPattern(heure, jour)?.atr_moyen ?? 0
}

function cellulePoints(heure: number, jour: number): number {
  return trouverPattern(heure, jour)?.nb_points ?? 0
}

function celluleStyle(heure: number, jour: number) {
  const p = trouverPattern(heure, jour)
  if (!p) return { background: '#ffffff08' }
  return {
    background: COULEURS_CLUSTER[p.cluster] ?? '#ffffff08',
    borderColor: COULEURS_CLUSTER_PLEIN[p.cluster] ?? 'transparent',
    border: '1px solid',
  }
}

function celluleTitre(heure: number, jour: number): string {
  const p = trouverPattern(heure, jour)
  const hParis = heureParis(heure)
  if (!p) return `${jours[jour]?.label} — ${hParis}h Paris (${ZONE_PARIS}) — aucune donnée`
  const nomCluster = ['Calme', 'Modéré', 'Élevé', 'Extrême'][p.cluster] ?? '?'
  return `${jours[jour]?.label} ${hParis}h Paris (${ZONE_PARIS}) | ATR: ${p.atr_moyen.toFixed(1)} | ${nomCluster} | ${p.nb_points} pts`
}

const NOM_CLUSTER = ['Calme', 'Modéré', 'Élevé', 'Extrême'] as const
const COULEUR_CLUSTER_TEXTE = ['text-emerald-400', 'text-amber-400', 'text-orange-400', 'text-red-400'] as const

const analyse = computed(() => {
  const patterns = reponse.value?.patterns
  if (!patterns?.length) return null

  const parHeure = heures.map(h => {
    const pts = patterns.filter(p => p.heure === h && p.nb_points > 0)
    if (!pts.length) return null
    const atrMoyen = pts.reduce((s, p) => s + p.atr_moyen, 0) / pts.length
    const clusterMoyen = Math.round(pts.reduce((s, p) => s + p.cluster, 0) / pts.length)
    return { heureUtc: h, heureParis: heureParis(h), cluster: clusterMoyen, atrMoyen }
  }).filter(Boolean) as { heureUtc: number; heureParis: number; cluster: number; atrMoyen: number }[]

  const top3 = [...parHeure].sort((a, b) => b.cluster - a.cluster || b.atrMoyen - a.atrMoyen).slice(0, 3)
  const pires3 = [...parHeure].sort((a, b) => a.cluster - b.cluster || a.atrMoyen - b.atrMoyen).slice(0, 3)

  const parJour = jours.map(j => {
    const pts = patterns.filter(p => p.jour_semaine === j.index && p.nb_points > 0)
    if (!pts.length) return null
    return { ...j, atrMoyen: pts.reduce((s, p) => s + p.atr_moyen, 0) / pts.length }
  }).filter(Boolean) as { index: number; label: string; atrMoyen: number }[]

  const meilleurJour = parJour.reduce((a, b) => a.atrMoyen > b.atrMoyen ? a : b)
  const pireJour = parJour.reduce((a, b) => a.atrMoyen < b.atrMoyen ? a : b)

  const hParisActuelle = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(new Date()))
  const heureUtcActuelle = (hParisActuelle - DECALAGE_PARIS + 24) % 24
  const jourActuel = new Date().getDay()
  const patternActuel = patterns.find(p => p.heure === heureUtcActuelle && p.jour_semaine === jourActuel) ?? null

  return { top3, pires3, meilleurJour, pireJour, patternActuel, hParisActuelle }
})

async function charger() {
  chargement.value = true
  try {
    reponse.value = await apiService.obtenirPatternsVolatilite(asset.value, timeframe.value, mois.value)
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Patterns volatilité: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

onMounted(async () => {
  assetsInfos.value = await apiService.obtenirAssets()
  await charger()
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
.glass-select {
  @apply bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2;
}
.btn-sm {
  @apply bg-gray-700 hover:bg-gray-600 disabled:opacity-40 text-white text-sm px-3 py-1.5 rounded-lg transition-all;
}
</style>
