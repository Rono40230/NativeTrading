<template>
  <div class="glass-bar px-4 py-2.5 flex flex-col gap-1 h-full">
    <span class="text-xs font-semibold uppercase tracking-widest text-blue-400 shrink-0">
      🕐 Prochains créneaux
    </span>

    <div v-if="chargement" class="text-xs text-gray-600 animate-pulse">Chargement…</div>

    <template v-else-if="prochain && secondesRestantes <= 86400">
      <!-- Asset + heure UTC -->
      <div class="flex items-center justify-between text-xs">
        <span class="font-bold text-white">{{ prochain.asset }}</span>
        <span class="text-gray-400">{{ prochain.heure_debut }}–{{ prochain.heure_fin }} UTC</span>
      </div>
      <!-- Rebours principal -->
      <div class="text-center">
        <span class="text-xl font-bold font-mono leading-tight" :class="reboursCls">{{ rebours }}</span>
      </div>
      <!-- Métriques compactes -->
      <div class="flex items-center gap-2 text-xs text-gray-500 flex-wrap">
        <span v-if="prochain.backtest_winrate !== null">
          WR <span class="font-semibold" :class="prochain.backtest_winrate >= 60 ? 'text-emerald-400' : prochain.backtest_winrate >= 50 ? 'text-yellow-400' : 'text-red-400'">{{ prochain.backtest_winrate }}%</span>
        </span>
        <span v-if="prochain.fenetre_entree" class="text-gray-500">· {{ prochain.fenetre_entree }}</span>
        <span v-if="prochain.whipsaw_minutes" class="text-orange-400 ml-auto">⚠ whipsaw {{ prochain.whipsaw_minutes }}min</span>
      </div>
    </template>

    <div v-else-if="prochain && secondesRestantes > 86400" class="flex flex-col gap-1 text-xs">
      <span class="text-gray-500 italic">Prochain dans {{ Math.floor(secondesRestantes / 3600) }}h</span>
      <span class="text-gray-600">{{ prochain.asset }} · {{ jourLabel(prochain.jour_semaine) }} {{ prochain.heure_debut }} UTC</span>
    </div>

    <span v-else class="text-xs text-gray-500 italic">Aucun créneau validé</span>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { StraddleCreneau } from '@/services/api.types'

const JOURS = ['Dim', 'Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam']

function jourLabel(jour: number | null): string {
  return jour !== null ? (JOURS[jour] ?? '') : 'Quotidien'
}

const creneaux = ref<StraddleCreneau[]>([])
const chargement = ref(true)
const now = ref(Date.now())

function secondesAvant(c: StraddleCreneau): number {
  const [hd, md] = c.heure_debut.split(':').map(Number)
  const jourCible = c.jour_semaine
  const base = new Date()
  base.setUTCHours(hd, md ?? 0, 0, 0)

  if (jourCible !== null) {
    const jourAujourdhuiUTC = new Date().getUTCDay()
    let delta = (jourCible - jourAujourdhuiUTC + 7) % 7
    if (delta === 0 && base.getTime() <= Date.now()) delta = 7
    base.setUTCDate(base.getUTCDate() + delta)
  } else if (base.getTime() <= Date.now()) {
    base.setUTCDate(base.getUTCDate() + 1)
  }

  return Math.max(0, Math.floor((base.getTime() - Date.now()) / 1000))
}

const prochain = computed<StraddleCreneau | null>(() => {
  void now.value
  const valides = creneaux.value.filter(c => c.statut === 'valide')
  if (!valides.length) return null
  return valides.reduce((min, c) => secondesAvant(c) < secondesAvant(min) ? c : min)
})

const secondesRestantes = computed(() => {
  void now.value
  return prochain.value ? secondesAvant(prochain.value) : 0
})

const rebours = computed(() => {
  const s = secondesRestantes.value
  if (s === 0) return 'Maintenant !'
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  if (h > 0) return `${h}h ${String(m).padStart(2, '0')}m`
  if (m > 0) return `${m}m ${String(sec).padStart(2, '0')}s`
  return `${sec}s`
})

const reboursCls = computed(() => {
  const s = secondesRestantes.value
  if (s === 0) return 'text-emerald-400 animate-pulse'
  if (s < 300) return 'text-red-400'
  if (s < 1800) return 'text-yellow-400'
  return 'text-blue-300'
})

async function charger() {
  try {
    creneaux.value = await apiService.getStraddleCreneaux()
  } catch {
    creneaux.value = []
  } finally {
    chargement.value = false
  }
}

let _tick: ReturnType<typeof setInterval> | null = null
let _poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
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
.glass-bar { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>

