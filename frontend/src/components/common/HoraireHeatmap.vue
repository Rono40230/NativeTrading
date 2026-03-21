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
        <p class="text-lg font-bold text-yellow-400">{{ reponse.seuil_straddle_calibre.toFixed(4) }}</p>
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
                  {{ celluleAtr(h, j.index).toFixed(1) }}
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
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { ReponsePatternsVolatilite } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'

const alerteStore = useAlerteStore()
const assets = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD']
const timeframes = ['M5', 'M15', 'H1', 'H4']
const asset = ref('BTC')
const timeframe = ref('M15')
const chargement = ref(false)
const reponse = ref<ReponsePatternsVolatilite | null>(null)

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

const jours = [
  { index: 0, label: 'Dim' },
  { index: 1, label: 'Lun' },
  { index: 2, label: 'Mar' },
  { index: 3, label: 'Mer' },
  { index: 4, label: 'Jeu' },
  { index: 5, label: 'Ven' },
  { index: 6, label: 'Sam' },
]

const clusters = [
  { label: 'Calme (Q0-25)', couleur: '#10b981' },
  { label: 'Modéré (Q25-50)', couleur: '#f59e0b' },
  { label: 'Élevé (Q50-75)', couleur: '#f97316' },
  { label: 'Extrême (Q75+)', couleur: '#ef4444' },
]

const COULEURS_CLUSTER = ['#10b98166', '#f59e0b66', '#f9731666', '#ef444466']
const COULEURS_CLUSTER_PLEIN = ['#10b981', '#f59e0b', '#f97316', '#ef4444']

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
  return `${jours[jour]?.label} ${hParis}h Paris (${ZONE_PARIS}) | ATR: ${p.atr_moyen.toFixed(4)} | ${nomCluster} | ${p.nb_points} pts`
}

async function charger() {
  chargement.value = true
  try {
    reponse.value = await apiService.obtenirPatternsVolatilite(asset.value, timeframe.value)
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Patterns volatilité: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

onMounted(() => charger())
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
