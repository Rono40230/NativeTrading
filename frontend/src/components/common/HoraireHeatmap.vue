<template>
  <div class="space-y-5">
    <!-- Contrôles -->
    <div class="flex flex-wrap items-center gap-3">
      <select v-model="asset" class="glass-select" @change="charger">
        <option v-for="a in assetsDropdown" :key="a" :value="a">{{ a }}</option>
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

    <div v-if="reponse" class="glass-card p-4 flex flex-wrap items-center gap-4">
      <div>
        <p class="text-xs text-gray-400 mb-0.5">Seuil Straddle calibré (P85)</p>
        <p class="text-lg font-bold text-yellow-400">{{ reponse.seuil_straddle_calibre.toFixed(1) }}</p>
      </div>
      <p class="text-xs text-gray-500 max-w-sm">
        ATR moyen au 85ème percentile sur l'historique. Le Straddle se déclenche quand l'ATR courant dépasse ce seuil.
      </p>
    </div>

    <div class="flex items-center gap-4 flex-wrap">
      <span v-for="c in clusters" :key="c.label" class="flex items-center gap-1.5 text-xs text-gray-300">
        <span class="w-3.5 h-3.5 rounded-sm" :style="{ background: c.couleur }" />
        {{ c.label }}
      </span>
    </div>

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
                class="w-full h-8 rounded flex items-center justify-center cursor-pointer transition-transform hover:scale-110"
                :style="celluleStyle(h, j.index)"
                @mouseenter="(e) => afficherTooltip(e, h, j.index)"
                @mouseleave="masquerTooltip"
                @click="selectionnerCellule(h, j.index)"
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

    <div v-else-if="!chargement" class="glass-card p-8 text-center text-gray-500 text-sm">
      Sélectionnez un asset et cliquez sur Charger.
    </div>

    <div v-if="analyse" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">

      <div class="glass-card p-4 space-y-3">
        <p class="text-xs font-semibold text-emerald-400 uppercase tracking-wider">Meilleures fenêtres de trading</p>
        <div v-for="f in analyse.top3" :key="f.heureDebut" class="flex items-center justify-between">
          <span class="text-sm text-white font-mono">{{ f.heureDebut }}h – {{ f.heureFin }}h Paris</span>
          <span :class="COULEUR_CLUSTER_TEXTE[f.cluster]" class="text-xs font-medium">{{ NOM_CLUSTER[f.cluster] }}</span>
        </div>
        <p class="text-[10px] text-gray-500 pt-1">Cluster dominant sur l'ensemble de la semaine</p>
      </div>

      <div class="glass-card p-4 space-y-3">
        <p class="text-xs font-semibold text-red-400 uppercase tracking-wider">Fenêtres à éviter</p>
        <div v-for="f in analyse.pires3" :key="f.heureDebut" class="flex items-center justify-between">
          <span class="text-sm text-white font-mono">{{ f.heureDebut }}h – {{ f.heureFin }}h Paris</span>
          <span :class="COULEUR_CLUSTER_TEXTE[f.cluster]" class="text-xs font-medium">{{ NOM_CLUSTER[f.cluster] }}</span>
        </div>
        <p class="text-[10px] text-gray-500 pt-1">Faible volatilité — spread défavorable</p>
      </div>

      <!-- Bloc jours de la semaine -->
      <div class="glass-card p-4 space-y-3">
        <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider">Jours de la semaine</p>
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
          <span class="text-sm text-gray-300">{{ analyse.meilleurJour.label }} — jour le plus actif</span>
        </div>
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-red-400"></span>
          <span class="text-sm text-gray-300">{{ analyse.pireJour.label }} — jour le plus calme</span>
        </div>
      </div>

      <!-- Bloc maintenant -->
      <div class="glass-card p-4 space-y-3">
        <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider">Maintenant — {{ analyse.hParisActuelle }}h Paris</p>
        <template v-if="analyse.patternActuel">
          <p :class="COULEUR_CLUSTER_TEXTE[analyse.patternActuel.cluster]" class="text-sm font-semibold">
            {{ NOM_CLUSTER[analyse.patternActuel.cluster] }} — ATR moyen {{ analyse.patternActuel.atr_moyen.toFixed(1) }}
          </p>
          <p class="text-xs text-gray-500">{{ analyse.patternActuel.cluster >= 2 ? 'Fenêtre favorable au trading actif.' : 'Attendre une fenêtre plus volatile.' }}</p>
        </template>
        <p v-else class="text-xs text-gray-500">Pas de données pour ce créneau.</p>
        <p class="text-[10px] text-gray-600">Basé sur l'historique — pas une garantie.</p>
      </div>
    </div>

    <!-- Tooltip cellule heatmap -->
    <Teleport v-if="tooltipVisible" to="body">
      <div
        class="fixed z-[9999] px-3 py-2 text-xs text-gray-200 bg-gray-950 border border-white/10 rounded-lg shadow-2xl pointer-events-none whitespace-nowrap"
        :style="{ top: `${tooltipPos.top}px`, left: `${tooltipPos.left}px`, transform: 'translate(-50%, calc(-100% - 8px))' }"
      >{{ tooltipTexte }}</div>
    </Teleport>

    <HoraireHeatmapPrecisionPanel :asset="asset" :cellule="celluleSelectionnee"
      :jour-label="celluleSelectionnee ? (jours[celluleSelectionnee.jour]?.label ?? '') : ''"
      :heure-paris="celluleSelectionnee ? heureParis(celluleSelectionnee.heure) : null" @fermer="celluleSelectionnee = null" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { JOURS as jours, CLUSTERS as clusters, COULEURS_CLUSTER, COULEURS_CLUSTER_PLEIN } from './heatmapConstants'
import { offsetParisHeures } from '@/utils/date'
import { apiService } from '@/services/api.service'
import type { ReponsePatternsVolatilite, AssetInfo } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import HoraireHeatmapPrecisionPanel from './HoraireHeatmapPrecisionPanel.vue'

const props = defineProps<{ assetsHeatmap?: string[] }>()

const alerteStore = useAlerteStore()
const assetsInfos = ref<AssetInfo[]>([])
const assetsInterne = computed(() => assetsInfos.value.map(a => a.id))
const assetsDropdown = computed(() => props.assetsHeatmap?.length ? props.assetsHeatmap : assetsInterne.value)
const periodesDisponibles = [6, 12, 18, 24]
const asset = ref('BTC')
const mois = ref(12)
const chargement = ref(false)
const reponse = ref<ReponsePatternsVolatilite | null>(null)
const celluleSelectionnee = ref<{ heure: number; jour: number } | null>(null)

function selectionnerCellule(heure: number, jour: number) {
  if (cellulePoints(heure, jour) === 0) return
  const meme = celluleSelectionnee.value?.heure === heure && celluleSelectionnee.value?.jour === jour
  celluleSelectionnee.value = meme ? null : { heure, jour }
}

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

/** Offset UTC→Paris recalculé à chaque appel (DST auto via IANA Europe/Paris). */
function zoneParis(): string {
  return offsetParisHeures() === 2 ? 'CEST' : 'CET'
}

function heureParis(heureUtc: number): number {
  return (heureUtc + offsetParisHeures()) % 24
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
  const decalage = offsetParisHeures()
  const hParis = (heure + decalage) % 24
  // Roulement de jour : si heureUTC + offset >= 24, l'heure Paris bascule au
  // jour suivant (convention jours = 0=Dim ... 6=Sam, +1 modulo 7).
  const roulement = heure + decalage >= 24
  const jourLabel = roulement ? jours[(jour + 1) % 7]?.label : jours[jour]?.label
  if (!p) return `${jourLabel} — ${hParis}h Paris (${zoneParis()}) — aucune donnée`
  const nomCluster = ['Calme', 'Modéré', 'Élevé', 'Extrême'][p.cluster] ?? '?'
  return `${jourLabel} ${hParis}h Paris (${zoneParis()}) | ATR: ${p.atr_moyen.toFixed(1)} | ${nomCluster} | ${p.nb_points} pts`
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

  type Slot = { heureDebut: number; heureFin: number; cluster: number }
  const fusionner = (h: typeof parHeure): Slot[] => [...h].sort((a, b) => a.heureParis - b.heureParis).reduce<Slot[]>((r, x) => { const l = r.at(-1); l && x.heureParis === l.heureFin ? (l.heureFin++, l.cluster = Math.max(l.cluster, x.cluster)) : r.push({ heureDebut: x.heureParis, heureFin: x.heureParis + 1, cluster: x.cluster }); return r }, []).slice(0, 3)
  const top3 = fusionner([...parHeure].sort((a, b) => b.cluster - a.cluster || b.atrMoyen - a.atrMoyen).slice(0, 6))
  const pires3 = fusionner([...parHeure].sort((a, b) => a.cluster - b.cluster || a.atrMoyen - b.atrMoyen).slice(0, 6))

  const parJour = jours.map(j => {
    const pts = patterns.filter(p => p.jour_semaine === j.index && p.nb_points > 0)
    if (!pts.length) return null
    return { ...j, atrMoyen: pts.reduce((s, p) => s + p.atr_moyen, 0) / pts.length }
  }).filter(Boolean) as { index: number; label: string; atrMoyen: number }[]

  const meilleurJour = parJour.reduce((a, b) => a.atrMoyen > b.atrMoyen ? a : b)
  const pireJour = parJour.reduce((a, b) => a.atrMoyen < b.atrMoyen ? a : b)

  // Recherche directe dans la convention des données (heure UTC, jour 0=Dim) :
  // on lit l'heure/jour UTC courants pour matcher le bucket exact, et l'heure
  // Paris uniquement pour le label du panneau.
  const maintenant = new Date()
  const hParisActuelle = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(maintenant))
  const heureUtcActuelle = maintenant.getUTCHours()
  const jourActuel = maintenant.getUTCDay()
  const patternActuel = patterns.find(p => p.heure === heureUtcActuelle && p.jour_semaine === jourActuel) ?? null

  return { top3, pires3, meilleurJour, pireJour, patternActuel, hParisActuelle }
})

async function charger() {
  chargement.value = true
  try {
    reponse.value = await apiService.obtenirPatternsVolatilite(asset.value, 'M1', mois.value)
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
