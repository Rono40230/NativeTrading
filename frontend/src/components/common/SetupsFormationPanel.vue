<template>
  <div class="flex flex-col gap-1.5">
    <div v-if="!affiches.length" class="text-[11px] text-white">Aucun setup en formation</div>

    <div v-for="s in affiches" :key="s.cle"
      class="flex items-center gap-2 text-xs rounded-lg border px-2.5 py-1.5"
      :class="classeCarte(s.statut)">
      <span class="text-[13px] leading-none">{{
        s.statut === 'EnFormation' ? '⏳' : s.statut === 'Confirme' ? '✓' : '✗'
      }}</span>
      <span class="font-semibold text-white">{{ s.asset }}</span>
      <span class="text-white">{{ s.tf }}</span>
      <span :class="s.direction === 'Long' ? 'text-emerald-400' : 'text-red-400'">
        {{ s.direction === 'Long' ? '🟢 Achat' : '🔴 Vente' }}
      </span>
      <span class="text-white font-mono">force {{ s.force }}/10</span>
      <span v-if="s.statut === 'EnFormation'" class="ml-auto text-amber-300/90 font-mono text-[11px]">
        se confirme dans {{ compteARebours(s.cloture_barre) }}
      </span>
      <span v-else-if="s.statut === 'Confirme'" class="ml-auto text-emerald-400 text-[11px]">confirmé → signaux actifs</span>
      <span v-else class="ml-auto text-white text-[11px]">dissipé à la clôture</span>
    </div>

    <div v-if="dissipes > 3" class="text-[10px] text-white">
      + {{ dissipes - 3 }} dissipé(s) plus ancien(s) ces 2 dernières heures
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

interface SetupFormation {
  strategie: string; asset: string; tf: string; direction: string
  force: number; entree: number; sl: number; tps: number[]
  cle: string; debut_barre: number; cloture_barre: number
  ts_annonce: number; statut: 'EnFormation' | 'Confirme' | 'Dissipe'
}

const props = defineProps<{ strategie?: string }>()

const setups = ref<SetupFormation[]>([])
let minuteur: ReturnType<typeof setInterval> | null = null
let horloge: ReturnType<typeof setInterval> | null = null
const tic = ref(0)

const affiches = computed(() => {
  const filtres = props.strategie
    ? setups.value.filter(s => s.strategie === props.strategie)
    : setups.value
  const enForm = filtres.filter(s => s.statut === 'EnFormation')
  const confirmes = filtres.filter(s => s.statut === 'Confirme')
  const diss = filtres.filter(s => s.statut === 'Dissipe')
  return [...enForm, ...confirmes, ...diss.slice(0, 3)]
})
const dissipes = computed(() =>
  setups.value.filter(s => s.statut === 'Dissipe' && (!props.strategie || s.strategie === props.strategie)).length,
)
async function charger() {
  try {
    const res = await http.get<SetupFormation[]>('/api/setups-formation')
    const tous = res.data as SetupFormation[]
    setups.value = props.strategie ? tous.filter(s => s.strategie === props.strategie) : tous
  } catch { /* silencieux */ }
}

function compteARebours(ts: number): string {
  tic.value // dépendance réactive
  const d = ts - Math.floor(Date.now() / 1000)
  if (d <= 0) return '…'
  const m = Math.floor(d / 60)
  const s = d % 60
  return m > 0 ? `${m} min ${String(s).padStart(2, '0')}s` : `${s} s`
}

function classeCarte(statut: string): string {
  if (statut === 'EnFormation') return 'border-amber-500/30 bg-amber-500/5'
  if (statut === 'Confirme') return 'border-emerald-500/25 bg-emerald-500/5'
  return 'border-white/8 bg-white/3 opacity-60'
}

onMounted(() => {
  void charger()
  minuteur = setInterval(charger, 15_000)
  horloge = setInterval(() => { tic.value++ }, 1000)
})
onUnmounted(() => {
  if (minuteur !== null) clearInterval(minuteur)
  if (horloge !== null) clearInterval(horloge)
})
</script>
