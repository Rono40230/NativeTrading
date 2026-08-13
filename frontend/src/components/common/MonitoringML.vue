<template>
  <div class="glass-card p-5 space-y-4">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3 min-w-0">
        <h2 class="text-xs uppercase font-bold text-white flex items-center shrink-0">
          🤖 Monitoring ML
          <TooltipInfo texte="Suivi de la précision du modèle hybride (LSTM 60% + XGBoost 40%) à chaque entraînement. Le seuil minimal est 60% — en dessous, une dérive est signalée." />
        </h2>
        <p v-if="historiqueML?.historique.length" class="text-[11px] text-gray-500 truncate">
          Période affichée: {{ periodeHistoriqueTexte() }} | Entraînements: {{ historiqueML.historique.length }}
        </p>
      </div>
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
        <span v-if="derniereMaj" class="text-[11px] text-gray-500">MAJ {{ formatHeure(derniereMaj) }}</span>
      </div>
    </div>

    <!-- Courbe accuracy -->
    <div v-if="historiqueML?.historique.length" class="w-full">
      <div ref="accuracyChart" class="h-48 w-full" />
    </div>
    <p v-else class="text-center text-sm text-gray-500 py-6">Aucun entraînement enregistré — lancez un entraînement via POST /api/ml/train</p>

    <!-- Tableau derniers entraînements -->
    <div v-if="historiqueML?.historique.length" class="overflow-x-auto">
      <table class="w-full text-xs text-left">
        <thead>
          <tr class="text-gray-400 border-b border-white/10">
            <th class="pb-2 pr-4">Date</th>
            <th class="pb-2 pr-4">Asset/TF</th>
            <th class="pb-2 pr-4 text-right">
              <span class="inline-flex items-center justify-end">Train <TooltipInfo texte="Précision d'entraînement du pipeline. Si non disponible dans l'historique ancien, fallback sur la métrique finale." /></span>
            </th>
            <th class="pb-2 pr-4 text-right">
              <span class="inline-flex items-center justify-end">Val <TooltipInfo texte="Précision de validation (holdout). Si non disponible dans l'historique ancien, fallback sur la métrique finale." /></span>
            </th>
            <th class="pb-2 pr-4 text-right">
              <span class="inline-flex items-center justify-end">XGB <TooltipInfo texte="Précision du modèle XGBoost seul. Compatible historique via accuracy_xgb ou accuracy_rf." /></span>
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
            <td class="py-1.5 pr-4 text-right" :class="trainPct(e) >= 0.60 ? 'text-emerald-400' : 'text-red-400'">{{ pct(trainPct(e)) }}</td>
            <td class="py-1.5 pr-4 text-right font-semibold" :class="valPct(e) >= 0.52 ? 'text-emerald-400' : 'text-amber-400'">{{ pct(valPct(e)) }}</td>
            <td class="py-1.5 pr-4 text-right" :class="xgbPct(e) >= 0.60 ? 'text-emerald-400' : 'text-red-400'">{{ pct(xgbPct(e)) }}</td>
            <td class="py-1.5 pr-4 text-right" :class="num(e.accuracy_lstm) >= 0.60 ? 'text-emerald-400' : 'text-red-400'">{{ pct(num(e.accuracy_lstm)) }}</td>
            <td class="py-1.5 pr-4 text-right font-semibold" :class="num(e.accuracy_finale) >= 0.60 ? 'text-emerald-400' : 'text-red-400'">{{ pct(num(e.accuracy_finale)) }}</td>
            <td class="py-1.5 text-right text-gray-400">{{ Math.round(e.duree_ms / 1000) }}s</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted, onUnmounted } from 'vue'
import { createChart, type IChartApi } from 'lightweight-charts'
import { formatParis } from '@/utils/date'
import { apiService } from '@/services/api.service'
import type { HistoriqueML } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import TooltipInfo from '@/components/common/TooltipInfo.vue'
import { tickMarkFormatterMl } from '@/composables/chartTimeScale'

const alerteStore = useAlerteStore()
const historiqueML = ref<HistoriqueML | null>(null)
const accuracyChart = ref<HTMLElement | null>(null)
const derniereMaj = ref<number | null>(null)
let chartAccuracy: IChartApi | null = null
let roAccuracy: ResizeObserver | null = null

function pct(v: number): string {
  if (Number.isNaN(v)) return '--'
  return `${(v * 100).toFixed(1)}%`
}

function num(v: unknown): number {
  return typeof v === 'number' && !Number.isNaN(v) ? v : 0
}

function trainPct(e: Record<string, unknown>): number {
  const train = e.accuracy_train
  if (typeof train === 'number' && !Number.isNaN(train)) return train
  return num(e.accuracy_finale)
}

function valPct(e: Record<string, unknown>): number {
  const val = e.accuracy_val
  if (typeof val === 'number' && !Number.isNaN(val)) return val
  return num(e.accuracy_finale)
}

function xgbPct(e: Record<string, unknown>): number {
  const xgb = e.accuracy_xgb
  if (typeof xgb === 'number' && !Number.isNaN(xgb)) return xgb
  return num(e.accuracy_rf)
}

function formatHeure(tsMs: number): string {
  return formatParis(new Date(tsMs), { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function formatDate(ts: number): string {
  return formatParis(ts, {
    day: '2-digit',
    month: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function periodeHistoriqueTexte(): string {
  const hist = historiqueML.value?.historique
  if (!hist || hist.length === 0) return '-'
  const recent = hist[0]
  const ancien = hist[hist.length - 1]
  const dRecent = new Date(recent.cree_le * 1000)
  const dAncien = new Date(ancien.cree_le * 1000)
  const fmt = (d: Date) =>
    formatParis(d, {
      day: '2-digit',
      month: '2-digit',
      year: 'numeric',
    })
  return `${fmt(dAncien)} -> ${fmt(dRecent)}`
}

async function chargerHistoriqueML() {
  try {
    historiqueML.value = await apiService.obtenirHistoriqueML(30)
    derniereMaj.value = Date.now()
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
    timeScale: { timeVisible: true, secondsVisible: false, tickMarkFormatter: tickMarkFormatterMl },
    width: accuracyChart.value.clientWidth,
    height: 192,
  })
  const serie = chartAccuracy.addLineSeries({ color: '#3b82f6', lineWidth: 2 })
  const seuil = chartAccuracy.addLineSeries({ color: '#ef444466', lineWidth: 1, lineStyle: 2 })
  const uniqueData=new Map();historiqueML.value.historique.forEach((e:any)=>uniqueData.set(e.cree_le,e));const sorted=Array.from(uniqueData.values()).sort((a:any,b:any)=>a.cree_le-b.cree_le)
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
  chartAccuracy.timeScale().fitContent()
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

let intervalML: ReturnType<typeof setInterval>

onMounted(() => {
  chargerHistoriqueML()
  intervalML = setInterval(chargerHistoriqueML, 5 * 60_000)
})

onUnmounted(() => {
  roAccuracy?.disconnect()
  clearInterval(intervalML)
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
