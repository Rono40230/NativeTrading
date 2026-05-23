<template>
  <div class="flex flex-col gap-3 min-w-0">

    <!-- En-tête colonne -->
    <div class="flex items-center justify-between px-1">
      <h2 class="text-sm font-bold text-gray-200">
        {{ titre }}
        <span v-if="result" class="ml-1 text-[10px] font-normal text-gray-500">
          · {{ result.nb_trades }} trades · {{ nbJours }} j
        </span>
      </h2>
      <span v-if="duree_ms !== null" class="text-[10px] text-gray-500">{{ duree_ms }} ms</span>
    </div>

    <!-- Skeleton -->
    <div v-if="chargement" class="space-y-2 animate-pulse">
      <div class="h-24 rounded-xl bg-white/5" />
      <div class="h-32 rounded-xl bg-white/5" />
    </div>

    <!-- Vide -->
    <div
      v-else-if="!result"
      class="flex-1 flex items-center justify-center text-gray-600 text-xs italic py-12"
    >
      En attente…
    </div>

    <template v-else>

      <!-- Equity curve -->
      <BacktestEquityCurve
        :equity-curve="result.equity_curve"
        :capital-initial="result.config.capital_initial"
        :trades="result.trades"
      />

      <!-- Distribution R:R -->
      <BacktestDistribRR :trades="result.trades" />

      <!-- Recommandations -->
      <section
        v-if="recommandations.length > 0"
        class="glass-card rounded-xl border border-amber-500/30 bg-amber-500/5 p-3"
      >
        <h3 class="text-xs font-semibold text-amber-400 mb-2">
          ⚠ {{ recommandations.length }} recommandation{{ recommandations.length > 1 ? 's' : '' }}
        </h3>
        <div class="flex flex-col gap-2">
          <div
            v-for="(rec, i) in recommandations"
            :key="i"
            class="rounded-lg border p-2.5 flex flex-col gap-1"
            :class="rec.priorite === 1 ? 'border-red-500/40 bg-red-500/5' : 'border-yellow-500/30 bg-yellow-500/5'"
          >
            <div class="flex items-start gap-2">
              <span class="text-[11px] font-semibold" :class="rec.priorite === 1 ? 'text-red-300' : 'text-yellow-300'">
                {{ rec.priorite === 1 ? '🔴' : '🟡' }} {{ rec.titre }}
              </span>
            </div>
            <p class="text-[10px] text-gray-400 leading-relaxed">{{ rec.explication }}</p>
            <div class="flex items-center gap-3 text-[10px] mt-0.5">
              <span class="text-gray-500">Actuel : <span class="text-white">{{ rec.valeur_actuelle }}</span></span>
              <span class="text-gray-500">→ <span class="text-emerald-400 font-semibold">{{ rec.valeur_suggeree }}</span></span>
              <span class="ml-auto text-emerald-500 italic">{{ rec.impact_estime }}</span>
            </div>
          </div>
        </div>
      </section>
      <section
        v-else
        class="glass-card rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-3 text-center"
      >
        <p class="text-[11px] text-emerald-400">
          ✅ Aucune recommandation — stratégie dans les clous
        </p>
      </section>

      <!-- Stats par heure -->
      <section
        v-if="heuresSignificatives.length > 0"
        class="glass-card rounded-xl border border-white/10 bg-white/5 p-3"
      >
        <h3 class="text-xs font-semibold text-gray-300 mb-2">Top créneaux (≥2 trades)</h3>
        <table class="w-full text-xs">
          <thead>
            <tr class="text-gray-500 border-b border-white/10">
              <th class="text-left pb-1.5">Heure</th>
              <th class="text-right pb-1.5">Trades</th>
              <th class="text-right pb-1.5">Win %</th>
              <th class="text-right pb-1.5">R moy</th>
              <th v-if="aEvenements" class="text-left pb-1.5 pl-3">Événement macro</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="s in heuresSignificatives"
              :key="s.heure"
              class="border-b border-white/5 hover:bg-white/5"
            >
              <td class="py-1 font-mono text-gray-300">{{ String(s.heure).padStart(2, '0') }}h</td>
              <td class="text-right text-gray-400">{{ s.nb_trades }}</td>
              <td
                class="text-right font-semibold"
                :class="s.win_rate >= 0.55 ? 'text-emerald-400' : s.win_rate >= 0.4 ? 'text-yellow-400' : 'text-red-400'"
              >{{ pct(s.win_rate) }}</td>
              <td
                class="text-right font-semibold"
                :class="s.pnl_r_moyen >= 0 ? 'text-emerald-400' : 'text-red-400'"
              >{{ s.pnl_r_moyen.toFixed(2) }}R</td>
              <td v-if="aEvenements" class="pl-3 text-[10px] text-gray-400">{{ s.evenement ?? '—' }}</td>
            </tr>
          </tbody>
        </table>
      </section>

    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { BacktestResult, Recommandation } from '@/services/api.backtest'
import BacktestEquityCurve from '@/components/common/BacktestEquityCurve.vue'
import BacktestDistribRR from '@/components/common/BacktestDistribRR.vue'

const props = defineProps<{
  titre:           string
  result:          BacktestResult | null
  recommandations: Recommandation[]
  duree_ms:        number | null
  chargement:      boolean
}>()

const pct = (v: number) => (v * 100).toFixed(1) + '%'
const usd = (v: number) => '$' + v.toLocaleString('fr-FR', { maximumFractionDigits: 0 })

const nbJours = computed(() => {
  const trades = props.result?.trades
  if (!trades?.length) return 0
  const debut = new Date(trades[0].ouvert_a)
  const fin = new Date(trades[trades.length - 1].ouvert_a)
  return Math.round((fin.getTime() - debut.getTime()) / 86_400_000)
})

const heuresSignificatives = computed(() => {
  const fenetresMap = new Map(
    (props.result?.fenetres_propices ?? []).map(f => [f.heure, f.evenement_type])
  )
  const triees = (props.result?.stats_par_heure ?? [])
    .filter(s => s.nb_trades >= 2)
    .sort((a, b) => b.pnl_r_moyen - a.pnl_r_moyen)
    .map(s => ({ ...s, evenement: fenetresMap.get(s.heure) ?? null }))
  // Top 3 minimum + tous les 100% au-delà
  const top3 = triees.slice(0, 3)
  const extra = triees.slice(3).filter(s => s.win_rate >= 1.0)
  const seen = new Set(top3.map(s => s.heure))
  return [...top3, ...extra.filter(s => !seen.has(s.heure))]
})

const aEvenements = computed(() => heuresSignificatives.value.some(s => s.evenement))
</script>
