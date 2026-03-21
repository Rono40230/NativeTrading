<template>
  <div class="glass-card p-5 space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider flex items-center">
        🤖 Monitoring ML
        <TooltipInfo texte="Suivi de la précision du modèle hybride (LSTM 60% + RF 40%) à chaque entraînement. Le seuil minimal est 60% — en dessous, une dérive est signalée." />
      </h2>
      <div class="flex items-center gap-3">
        <span v-if="historiqueML?.nb_entrainements && historiqueML?.derive_detectee" class="inline-flex items-center text-xs text-red-400 font-semibold animate-pulse">
          ⚠️ Dérive détectée
          <TooltipInfo texte="La précision moyenne des derniers entraînements est sous 60%. Réentraînement ou vérification des données source recommandée." />
        </span>
        <span v-else-if="historiqueML?.nb_entrainements" class="inline-flex items-center text-xs text-emerald-400">
          ✓ Modèle stable
          <TooltipInfo texte="La précision moyenne des entraînements récents dépasse 60%. Le modèle est fiable pour la génération de signaux de trading." />
        </span>
        <button class="text-xs text-blue-400 hover:text-blue-300 underline" @click="chargerHistoriqueML">Actualiser</button>
      </div>
    </div>

    <!-- Courbe accuracy -->
    <div v-if="historiqueML?.historique.length" ref="accuracyChart" class="h-48 w-full" />
    <p v-else class="text-center text-sm text-gray-500 py-6">Aucun entraînement enregistré — lancez un entraînement via POST /api/ml/train</p>

    <!-- Tableau derniers entraînements -->
    <div v-if="historiqueML?.historique.length" class="overflow-x-auto">
      <table class="w-full text-xs text-left">
        <thead>
          <tr class="text-gray-400 border-b border-white/10">
            <th class="pb-2 pr-4">Date</th>
            <th class="pb-2 pr-4">Asset/TF</th>
            <th class="pb-2 pr-4 text-right">
              <span class="inline-flex items-center justify-end">RF <TooltipInfo texte="Précision du modèle Random Forest seul, évalué sur 25% de données out-of-sample (walk-forward)." /></span>
            </th>
            <th class="pb-2 pr-4 text-right">
              <span class="inline-flex items-center justify-end">LSTM <TooltipInfo texte="Précision du réseau LSTM seul (3 couches : 128→64→32), évalué sur 25% de données out-of-sample." /></span>
            </th>
            <th class="pb-2 pr-4 text-right">
              <span class="inline-flex items-center justify-end">Finale <TooltipInfo texte="Score combiné : 0.6 × LSTM + 0.4 × RF. C'est ce score qui déclenche la détection de dérive (< 60%)." /></span>
            </th>
            <th class="pb-2 text-right">
              <span class="inline-flex items-center justify-end">Durée <TooltipInfo texte="Temps total d'entraînement du pipeline ML complet : RF + LSTM + évaluation walk-forward." /></span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="e in historiqueML.historique.slice(0, 10)"
            :key="e.id"
            class="border-b border-white/5"
            :class="e.derive_detectee ? 'bg-red-500/5' : ''"
          >
            <td class="py-1.5 pr-4 text-gray-300">{{ formatDate(e.cree_le) }}</td>
            <td class="py-1.5 pr-4 text-gray-300">{{ e.asset }}/{{ e.timeframe }}</td>
            <td class="py-1.5 pr-4 text-right" :class="e.accuracy_rf >= 0.60 ? 'text-emerald-400' : 'text-red-400'">{{ pct(e.accuracy_rf) }}</td>
            <td class="py-1.5 pr-4 text-right" :class="e.accuracy_lstm >= 0.60 ? 'text-emerald-400' : 'text-red-400'">{{ pct(e.accuracy_lstm) }}</td>
            <td class="py-1.5 pr-4 text-right font-semibold" :class="e.accuracy_finale >= 0.60 ? 'text-emerald-400' : 'text-red-400'">{{ pct(e.accuracy_finale) }}</td>
            <td class="py-1.5 text-right text-gray-400">{{ Math.round(e.duree_ms / 1000) }}s</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { createChart, type IChartApi } from 'lightweight-charts'
import { apiService } from '@/services/api.service'
import type { HistoriqueML } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import TooltipInfo from '@/components/common/TooltipInfo.vue'

const alerteStore = useAlerteStore()
const historiqueML = ref<HistoriqueML | null>(null)
const accuracyChart = ref<HTMLElement | null>(null)
let chartAccuracy: IChartApi | null = null
let roAccuracy: ResizeObserver | null = null

function pct(v: number): string {
  return `${(v * 100).toFixed(1)}%`
}

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString('fr-FR', {
    day: '2-digit',
    month: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

async function chargerHistoriqueML() {
  try {
    historiqueML.value = await apiService.obtenirHistoriqueML(30)
    await nextTick()
    afficherCourbeAccuracy()
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Historique ML: ${(e as Error).message}`)
  }
}

function afficherCourbeAccuracy() {
  if (!accuracyChart.value || !historiqueML.value?.historique.length) return
  chartAccuracy?.remove()
  chartAccuracy = createChart(accuracyChart.value, {
    layout: { background: { color: 'transparent' }, textColor: '#9ca3af' },
    grid: { vertLines: { color: '#1f2937' }, horzLines: { color: '#1f2937' } },
    width: accuracyChart.value.clientWidth,
    height: 192,
  })
  const serie = chartAccuracy.addLineSeries({ color: '#3b82f6', lineWidth: 2 })
  const seuil = chartAccuracy.addLineSeries({ color: '#ef444466', lineWidth: 1, lineStyle: 2 })
  const sorted = [...historiqueML.value.historique].reverse()
  serie.setData(
    sorted.map((e) => ({
      time: e.cree_le as unknown as import('lightweight-charts').Time,
      value: +(e.accuracy_finale * 100).toFixed(2),
    })),
  )
  seuil.setData(
    sorted.map((e) => ({
      time: e.cree_le as unknown as import('lightweight-charts').Time,
      value: 60,
    })),
  )
}

watch(accuracyChart, (el) => {
  if (el && historiqueML.value?.historique.length) afficherCourbeAccuracy()
})

watch(accuracyChart, (el, old) => {
  roAccuracy?.disconnect()
  if (!el) return
  roAccuracy = new ResizeObserver(() => {
    chartAccuracy?.applyOptions({ width: el.clientWidth })
  })
  roAccuracy.observe(el)
  if (old) roAccuracy.disconnect()
})

onMounted(() => {
  chargerHistoriqueML()
})

onUnmounted(() => {
  roAccuracy?.disconnect()
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
