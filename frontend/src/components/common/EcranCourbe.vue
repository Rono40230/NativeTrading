<template>
  <!-- ÉCRAN de la colonne-instrument (cockpit 24/09) : l'histoire en $ de la
       stratégie — courbe de capital bicolore au capital de départ, survol
       par clôture. L'exact bloc courbe des anciennes cartes, autonome. -->
  <div class="relative h-20 rounded-lg border border-white/10 bg-black/30 px-1 -mx-0.5" @mouseleave="survol = null">
    <CourbeCapital v-if="capital && capital.points.length > 0" :capital="capital" :id-bloc="id" />
    <div v-else class="w-full h-full flex items-center justify-center text-[10px] text-white/50">
      Courbe du capital — dès les premières clôtures
    </div>
    <!-- Zones de survol : une par clôture, ancrées sur la courbe -->
    <div v-if="capital && capital.points.length > 0" class="absolute inset-0">
      <div
        v-for="(z, i) in zonesCapital(capital)"
        :key="id + '-zec' + i"
        class="absolute w-3 h-4 -translate-x-1/2 -translate-y-1/2"
        :style="{ left: z.gauche, top: z.haut }"
        @mouseenter="survolPoint($event, z)"
      />
    </div>
    <!-- Tooltip ancré viewport, jamais rogné -->
    <div
      v-if="survol"
      class="fixed z-50 pointer-events-none bg-slate-900/95 border border-blue-400/30 rounded-lg px-2.5 py-1.5 shadow-xl whitespace-nowrap"
      :style="styleTooltip"
    >
      <p class="text-[10px] font-bold text-white">
        {{ libelleDate(survol.point.ferme_le) }} · capital {{ fmtDollars(survol.point.capital_apres) }}
      </p>
      <p class="text-[9px] font-bold" :class="survol.point.profit >= 0 ? 'text-emerald-400' : 'text-red-400'">
        trade {{ survol.point.profit >= 0 ? '+' : '−' }}{{ fmtDollars(Math.abs(survol.point.profit)) }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'
import CourbeCapital from './CourbeCapital.vue'
import { zonesCapital, type PointCapital } from '@/composables/useCourbeCapital'

const props = defineProps<{ id: string }>()

interface CapitalApi {
  capital_depart: number
  capital_actuel: number
  points: PointCapital[]
}
const capital = ref<CapitalApi | null>(null)
const survol = ref<{ point: PointCapital; x: number; y: number } | null>(null)

function survolPoint(e: MouseEvent, z: { point: PointCapital }) {
  const r = (e.target as Element).getBoundingClientRect()
  survol.value = { point: z.point, x: r.left + r.width / 2, y: r.top }
}

const styleTooltip = computed(() => {
  const s = survol.value
  if (!s) return {}
  const demi = 95
  const x = Math.min(Math.max(s.x, demi + 8), window.innerWidth - demi - 8)
  const auDessus = s.y > 220
  return {
    top: `${auDessus ? s.y - 8 : s.y + 14}px`,
    left: `${x}px`,
    transform: `translate(-50%, ${auDessus ? '-100%' : '0'})`,
  }
})

function libelleDate(ts: number): string {
  return new Date(ts * 1000).toLocaleString('fr-FR', {
    day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit',
  })
}
function fmtDollars(v: number): string {
  const n = Math.round(Math.abs(v)).toLocaleString('fr-FR')
  return `${v < 0 ? '−' : ''}${n} $`
}

let poll: ReturnType<typeof setInterval> | null = null
async function charger() {
  try {
    const c = await http.get<CapitalApi>(`/api/strategies/${props.id}/capital`)
    capital.value = c.data as CapitalApi
  } catch { capital.value = null }
}
onMounted(() => {
  void charger()
  poll = setInterval(charger, 60_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>
