<template>
  <div class="space-y-4">
    <!-- Bandeau dérive -->
    <div v-if="monitoring?.derive_detectee"
      class="flex items-center gap-2 rounded-lg bg-orange-900/30 border border-orange-500/30 px-3 py-2 text-xs text-orange-300">
      <span class="text-base">⚠️</span>
      <span class="font-semibold">Dérive LLM détectée</span>
      <span class="text-orange-400/70">— Win rate &lt; 45% sur les 20 derniers trades. Recalibration en cours (prochaine
        run dans &lt; 6h).</span>
    </div>

    <!-- Métriques globales -->
    <template v-if="monitoring">
      <!-- Vue compact (colonne) -->
      <div v-if="props.compact">
        <p class="text-[9px] text-gray-500 mb-2">{{ monitoring.nb_signals_total }} signaux · {{
          monitoring.nb_feedbacks_clotures }} clôturés · {{ monitoring.nb_invalides }} invalides</p>
        <div class="grid grid-cols-2 gap-1.5 mb-3">
          <div class="rounded-md border border-emerald-500/20 bg-emerald-900/10 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-emerald-600 uppercase tracking-wider">✅ Gagnants</span>
            <span class="text-base font-bold text-emerald-400">{{ monitoring.nb_gagnants }}</span>
          </div>
          <div class="rounded-md border border-red-500/20 bg-red-900/10 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-red-600 uppercase tracking-wider">❌ Perdants</span>
            <span class="text-base font-bold text-red-400">{{ monitoring.nb_perdants }}</span>
          </div>
          <div class="rounded-md border border-white/10 bg-white/5 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-gray-500 uppercase tracking-wider">Win Rate</span>
            <span class="text-base font-bold"
              :class="monitoring.win_rate_global >= 0.55 ? 'text-emerald-400' : monitoring.win_rate_global >= 0.45 ? 'text-yellow-400' : 'text-red-400'">{{
                pct(monitoring.win_rate_global) }}</span>
          </div>
          <div class="rounded-md border border-white/10 bg-white/5 px-2 py-1.5 flex flex-col gap-0.5">
            <span class="text-[9px] text-gray-500 uppercase tracking-wider">P&L moy (R)</span>
            <span class="text-base font-bold"
              :class="(monitoring.pnl_moyen_r ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">{{
                monitoring.pnl_moyen_r != null ? monitoring.pnl_moyen_r.toFixed(2) + 'R' : '—' }}</span>
          </div>
        </div>
      </div>
      <!-- Vue complète -->
      <div v-else class="grid grid-cols-4 gap-2">
        <div class="rounded-lg border border-white/10 bg-white/5 px-3 py-2.5 flex flex-col gap-0.5">
          <span class="text-[10px] text-gray-500 uppercase tracking-wider">Total signaux</span>
          <span class="text-xl font-bold text-white">{{ monitoring.nb_signals_total }}</span>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/5 px-3 py-2.5 flex flex-col gap-0.5">
          <span class="text-[10px] text-gray-500 uppercase tracking-wider">Clôturés</span>
          <span class="text-xl font-bold text-gray-300">{{ monitoring.nb_feedbacks_clotures }}</span>
        </div>
        <div class="rounded-lg border border-emerald-500/20 bg-emerald-900/10 px-3 py-2.5 flex flex-col gap-0.5">
          <span class="text-[10px] text-emerald-600 uppercase tracking-wider">✅ Gagnants</span>
          <span class="text-xl font-bold text-emerald-400">{{ monitoring.nb_gagnants }}</span>
        </div>
        <div class="rounded-lg border border-red-500/20 bg-red-900/10 px-3 py-2.5 flex flex-col gap-0.5">
          <span class="text-[10px] text-red-600 uppercase tracking-wider">❌ Perdants</span>
          <span class="text-xl font-bold text-red-400">{{ monitoring.nb_perdants }}</span>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/5 px-3 py-2.5 flex flex-col gap-0.5">
          <span class="text-[10px] text-gray-500 uppercase tracking-wider">⚠️ Invalides</span>
          <span class="text-xl font-bold text-gray-400">{{ monitoring.nb_invalides }}</span>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/5 px-3 py-2.5 flex flex-col gap-0.5">
          <span class="text-[10px] text-gray-500 uppercase tracking-wider">Win Rate global</span>
          <span class="text-xl font-bold"
            :class="monitoring.win_rate_global >= 0.55 ? 'text-emerald-400' : monitoring.win_rate_global >= 0.45 ? 'text-yellow-400' : 'text-red-400'">
            {{ pct(monitoring.win_rate_global) }}
          </span>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/5 px-3 py-2.5 flex flex-col gap-0.5 col-span-2">
          <span class="text-[10px] text-gray-500 uppercase tracking-wider">P&L moyen (R)</span>
          <span class="text-xl font-bold"
            :class="(monitoring.pnl_moyen_r ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ monitoring.pnl_moyen_r != null ? monitoring.pnl_moyen_r.toFixed(2) + 'R' : '—' }}
          </span>
        </div>
      </div>
    </template>
    <div v-else-if="chargementMonitoring" class="text-center text-xs text-gray-500 py-4 animate-pulse">Chargement
      stats...
    </div>
    <div v-else class="text-center text-xs text-gray-500 py-4">
      Aucun trade Straddle clôturé — les stats apparaîtront ici après les premiers signaux.
    </div>

    <!-- Tableau calibration par catégorie -->
    <div v-if="calibration.length">
      <p :class="props.compact ? 'text-[10px]' : 'text-[11px]'"
        class="font-semibold uppercase tracking-wider text-gray-400 mb-2">Calibration par catégorie</p>
      <div class="overflow-x-auto">
        <table class="w-full" :class="props.compact ? 'text-[10px]' : 'text-xs'">
          <thead>
            <tr class="text-gray-500 border-b border-white/10">
              <th class="pb-1.5 text-left pr-3">Asset</th>
              <th class="pb-1.5 text-left pr-3">Catégorie</th>
              <th class="pb-1.5 text-right pr-3">Trades</th>
              <th class="pb-1.5 text-right pr-3">Win Rate</th>
              <th class="pb-1.5 text-right pr-3">Seuil LLM</th>
              <th class="pb-1.5 text-right pr-3">ATR min</th>
              <th class="pb-1.5 text-left">Fiabilité</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(r, i) in calibration" :key="i" class="border-b border-white/5 hover:bg-white/5">
              <td class="py-1.5 pr-3 font-semibold text-white">{{ r.asset }}</td>
              <td class="py-1.5 pr-3">
                <span class="text-[10px] px-1.5 py-0.5 rounded font-semibold" :class="badgeCategorie(r.categorie)">
                  {{ labelCategorie(r.categorie) }}
                </span>
              </td>
              <td class="py-1.5 pr-3 text-right text-gray-300">{{ r.nb_trades }}</td>
              <td class="py-1.5 pr-3 text-right font-semibold"
                :class="r.win_rate >= 0.55 ? 'text-emerald-400' : r.win_rate >= 0.45 ? 'text-yellow-400' : 'text-red-400'">
                {{ pct(r.win_rate) }}
              </td>
              <td class="py-1.5 pr-3 text-right text-blue-300 font-mono">{{ r.score_seuil.toFixed(1) }}</td>
              <td class="py-1.5 pr-3 text-right text-yellow-300 font-mono">{{ r.ratio_atr_min.toFixed(2) }}×</td>
              <td class="py-1.5">
                <span class="text-[10px] px-1.5 py-0.5 rounded font-semibold" :class="badgeFiabilite(r.fiabilite)">
                  {{ r.fiabilite }}
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
    <div v-else-if="chargementCalib" class="text-center text-xs text-gray-500 py-2 animate-pulse">Chargement
      calibration...
    </div>
    <div v-else class="text-center text-xs text-gray-600 py-2 italic">
      Calibration disponible après 10+ trades par catégorie
    </div>

    <!-- Tableau par catégorie (métriques monitoring) -->
    <div v-if="!props.compact && monitoring?.par_categorie?.length">
      <p class="text-[11px] font-semibold uppercase tracking-wider text-gray-400 mb-2">Performance par catégorie</p>
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-gray-500 border-b border-white/10">
              <th class="pb-1.5 text-left pr-3">Catégorie</th>
              <th class="pb-1.5 text-right pr-3">Trades</th>
              <th class="pb-1.5 text-right pr-3">Win Rate</th>
              <th class="pb-1.5 text-right pr-3">Score LLM ✓</th>
              <th class="pb-1.5 text-right pr-3">Score LLM ✗</th>
              <th class="pb-1.5 text-right">P&L moy R</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(c, i) in monitoring.par_categorie" :key="i" class="border-b border-white/5 hover:bg-white/5">
              <td class="py-1.5 pr-3">
                <span class="text-[10px] px-1.5 py-0.5 rounded font-semibold" :class="badgeCategorie(c.categorie)">
                  {{ labelCategorie(c.categorie) }}
                </span>
              </td>
              <td class="py-1.5 pr-3 text-right text-gray-300">{{ c.nb_trades }}</td>
              <td class="py-1.5 pr-3 text-right font-semibold"
                :class="c.win_rate >= 0.55 ? 'text-emerald-400' : c.win_rate >= 0.45 ? 'text-yellow-400' : 'text-red-400'">
                {{ pct(c.win_rate) }}
              </td>
              <td class="py-1.5 pr-3 text-right text-emerald-400">{{ c.score_llm_win != null ?
                c.score_llm_win.toFixed(1) :
                '—' }}</td>
              <td class="py-1.5 pr-3 text-right text-red-400">{{ c.score_llm_lose != null ? c.score_llm_lose.toFixed(1)
                :
                '—' }}</td>
              <td class="py-1.5 text-right" :class="(c.pnl_r_moyen ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
                {{ c.pnl_r_moyen != null ? c.pnl_r_moyen.toFixed(2) + 'R' : '—' }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Footer -->
    <div v-if="derniereMaj" class="pt-1 flex justify-between text-[10px] text-gray-600">
      <button class="hover:text-gray-400 transition" @click="charger">↻ Actualiser</button>
      <span>MAJ {{ formatHeure(derniereMaj) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { StraddleMonitoringData, StraddleCalibrationRow } from '@/services/api.types'
import { useAlerteStore } from '@/stores/alerte.store'

const props = withDefaults(defineProps<{ compact?: boolean }>(), { compact: false })
const alerteStore = useAlerteStore()
const monitoring = ref<StraddleMonitoringData | null>(null)
const calibration = ref<StraddleCalibrationRow[]>([])
const chargementMonitoring = ref(false)
const chargementCalib = ref(false)
const derniereMaj = ref<number | null>(null)

function pct(v: number): string {
  return `${(v * 100).toFixed(1)}%`
}

function formatHeure(ts: number): string {
  return new Date(ts).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

function badgeCategorie(cat: string): string {
  const map: Record<string, string> = {
    annonce_high: 'bg-red-900/60 text-red-300',
    annonce_medium: 'bg-orange-900/50 text-orange-300',
    overlap_lnd_ny: 'bg-purple-900/50 text-purple-300',
    ny_open: 'bg-blue-900/50 text-blue-300',
    london_open: 'bg-sky-900/50 text-sky-300',
    tokyo_open: 'bg-cyan-900/50 text-cyan-300',
    creneau_recurrent: 'bg-yellow-900/50 text-yellow-300',
    choc_isole: 'bg-gray-800 text-gray-400',
  }
  return map[cat] ?? 'bg-gray-800 text-gray-400'
}

function labelCategorie(cat: string): string {
  const map: Record<string, string> = {
    annonce_high: '🔴 High', annonce_medium: '🟡 Medium',
    overlap_lnd_ny: '🟣 Overlap', ny_open: '🔵 NY', london_open: '🔵 London',
    tokyo_open: '🩵 Tokyo', creneau_recurrent: '⭐ Récurrent', choc_isole: '⬜ Isolé',
  }
  return map[cat] ?? cat
}

function badgeFiabilite(f: string): string {
  if (f === 'fort') return 'bg-emerald-900/50 text-emerald-300'
  if (f === 'correct') return 'bg-blue-900/50 text-blue-300'
  if (f === 'faible') return 'bg-yellow-900/50 text-yellow-300'
  return 'bg-gray-800 text-gray-400'
}

async function charger() {
  chargementMonitoring.value = true
  chargementCalib.value = true
  try {
    const [m, c] = await Promise.all([
      apiService.getStraddleMonitoringML(),
      apiService.getStraddleCalibration(),
    ])
    monitoring.value = m
    calibration.value = c
    derniereMaj.value = Date.now()
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Monitoring ML Straddle: ${(e as Error).message}`)
  } finally {
    chargementMonitoring.value = false
    chargementCalib.value = false
  }
}

onMounted(charger)
</script>
