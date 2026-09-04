<template>
  <!-- Vue d'ensemble du rapport d'activité : les trois stratégies côte à
       côte — capital, R, réussite, hiér — pour comparer les allocations. -->
  <div class="flex flex-col gap-3">
    <div class="glass-card p-3">
      <p class="text-xs font-semibold text-white mb-2">🧭 Les trois stratégies</p>
      <div v-if="liste.length" class="overflow-x-auto">
        <table class="w-full text-[11px]">
          <thead>
            <tr class="text-white text-left">
              <th class="py-1 pr-2 font-semibold">Stratégie</th>
              <th class="py-1 pr-2 font-semibold text-right" title="Clôtures analysées">Clôtures</th>
              <th class="py-1 pr-2 font-semibold text-right" title="WR — part des clôtures gagnantes ($ > 0)">WR</th>
              <th class="py-1 pr-2 font-semibold text-right" title="Σ R de la convention du moteur">Σ R</th>
              <th class="py-1 pr-2 font-semibold text-right" title="Capital simulé départ → actuel">Capital</th>
              <th class="py-1 font-semibold text-right" title="Journée d'hier (heure locale)">Hier</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="s in liste"
              :key="s.strategie"
              class="cursor-pointer border-t border-white/5 hover:bg-white/5 transition-colors"
              :title="`Ouvrir l'analyse ${s.strategie}`"
              @click="$emit('ouvrir', s.strategie)"
            >
              <td class="py-1.5 pr-2">
                <span class="mr-1">{{ icone(s.strategie) }}</span>
                <span class="font-semibold text-white">{{ s.strategie }}</span>
                <span class="ml-1.5 text-[9px] px-1.5 py-0.5 rounded-full border" :class="badgeClasse(s.etat)">{{ s.etat }}</span>
                <span class="ml-1 text-[9px] text-white">{{ s.source === 'rejeu' ? '· re-jeu' : '' }}</span>
              </td>
              <td class="py-1.5 pr-2 text-right text-white">{{ s.nb_trades }}</td>
              <td class="py-1.5 pr-2 text-right text-white">{{ (s.taux_reussite * 100).toFixed(0) }} %</td>
              <td class="py-1.5 pr-2 text-right font-mono" :class="s.r_total > 0 ? 'text-emerald-400' : s.r_total < 0 ? 'text-red-400' : 'text-white'">{{ fmtR(s.r_total) }}</td>
              <td class="py-1.5 pr-2 text-right font-mono" :class="s.capital_actuel >= s.capital_depart ? 'text-emerald-400' : 'text-red-400'">
                {{ fmtDollars(s.capital_actuel) }}
                <span class="text-white/60 text-[10px]">/ {{ fmtDollars(s.capital_depart) }}</span>
              </td>
              <td class="py-1.5 text-right font-mono" :class="!s.hier ? 'text-white/50' : s.hier.dollars >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ s.hier ? fmtDollars(s.hier.dollars) : '—' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-else class="text-xs text-white py-4 text-center">Analyses indisponibles — le backend calcule à la première demande.</p>
    </div>

    <!-- Contributions cumulées : qui pèse combien sur le capital total -->
    <div class="glass-card p-3">
      <p class="text-xs font-semibold text-white mb-2">💰 Capital par stratégie (départ → actuel)</p>
      <div v-if="liste.length" class="flex flex-col gap-1.5">
        <div v-for="s in liste" :key="s.strategie" class="flex items-center gap-2 text-[11px]">
          <span class="w-20 shrink-0 text-white truncate">{{ icone(s.strategie) }} {{ s.strategie }}</span>
          <div class="flex-1 h-4 bg-white/5 rounded overflow-hidden relative">
            <div class="absolute inset-y-0 left-1/2 w-px bg-white/15" />
            <div
              class="absolute inset-y-0.5 rounded"
              :class="s.capital_actuel - s.capital_depart >= 0 ? 'bg-emerald-400/70 left-1/2' : 'bg-red-400/70 right-1/2'"
              :style="{ width: `${largeur(s)}%` }"
            />
          </div>
          <span class="w-24 text-right font-mono shrink-0" :class="s.capital_actuel - s.capital_depart >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ fmtDollars(s.capital_actuel - s.capital_depart) }}
          </span>
        </div>
      </div>
      <p v-else class="text-xs text-white py-4 text-center">—</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { chargerAnalyses, fmtDollars, fmtR, type ResumeStrategie } from '@/composables/useAnalyses'

defineEmits<{ ouvrir: [strategie: string] }>()

const liste = ref<ResumeStrategie[]>([])

const ICONES: Record<string, string> = { SMC: '📐', straddle: '⚡', rockets: '🚀' }
function icone(id: string): string {
  return ICONES[id] ?? '📊'
}

function badgeClasse(etat: string): string {
  if (etat === 'Officielle') return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
  if (etat === 'Observation') return 'bg-amber-500/10 text-amber-400 border-amber-500/30'
  return 'bg-gray-500/10 text-white border-gray-500/30'
}

/// Largeur (% de la demi-zone) de la barre de variation de capital.
function largeur(s: ResumeStrategie): number {
  const variations = liste.value.map(x => Math.abs(x.capital_actuel - x.capital_depart))
  const maxAbs = Math.max(...variations, 1)
  return Math.max(2, (Math.abs(s.capital_actuel - s.capital_depart) / maxAbs) * 50)
}

onMounted(async () => {
  liste.value = await chargerAnalyses()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
