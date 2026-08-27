<template>
  <div class="glass-card px-4 py-2 flex flex-col gap-1.5">
    <div class="flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">⏰ Créneaux de volatilité</p>
      <span class="text-[9px] text-white">heures Paris · 24 mois glissants</span>
    </div>

    <div v-if="chargement" class="text-center text-slate-500 text-xs py-3">Calcul…</div>
    <div v-else-if="!creneaux.length" class="text-center text-slate-500 text-xs py-3">Aucune donnée</div>

    <template v-else>
      <!-- Heures : celle en cours en gros gras encadré -->
      <div class="flex items-center gap-1">
        <span class="w-14 shrink-0"></span>
        <div class="flex-1 flex gap-[2px]">
          <span v-for="h in 24" :key="'lbl'+h"
            class="flex-1 text-center font-mono"
            :class="estHeureCourante(h-1)
              ? 'text-[11px] font-extrabold text-white bg-white/20 rounded border border-white/50 py-0.5'
              : 'text-[7px] text-white'"
          >{{ String(h-1).padStart(2,'0') }}</span>
        </div>
      </div>

      <!-- Une rangée par asset -->
      <div v-for="c in creneaux" :key="c.asset" class="flex items-center gap-1">
        <span class="w-14 shrink-0 text-[10px] font-semibold text-white truncate">{{ c.asset }}</span>
        <div class="flex-1 flex gap-[2px] h-6">
          <div v-for="h in 24" :key="c.asset+'-'+h"
            class="flex-1 rounded-[2px]"
            :style="{ backgroundColor: couleurCellule(c, h-1) }"
          />
        </div>
      </div>

      <!-- Légende -->
      <div class="flex items-center gap-2 text-[8px] text-slate-500 mt-0.5">
        <span class="w-14 shrink-0"></span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:rgba(255,255,255,0.06)" /> fermé</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:rgba(34,197,94,0.35)" /> calme</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:rgba(34,197,94,0.65)" /> modéré</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:rgba(250,204,21,0.65)" /> actif</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:rgba(249,115,22,0.75)" /> fort</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:rgba(239,68,68,0.85)" /> bouillant</span>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

interface Plage { debut: number; fin: number; vol_pct: number; fiabilite: number; nb_heures: number }
interface Heure { heure: number; vol_pct: number; fiabilite: number }
interface ParAsset { asset: string; top: Plage[]; heures?: Heure[] }

const creneaux = ref<ParAsset[]>([])
const chargement = ref(true)
const maintenant = ref(new Date())

let poll: ReturnType<typeof setInterval> | null = null

async function charger() {
  try {
    const res = await http.get('/api/creneaux-volatilite')
    creneaux.value = res.data
  } catch { /* silencieux */ } finally {
    chargement.value = false
  }
}

const heureParis = computed(() =>
  Number(new Intl.DateTimeFormat('fr-FR', { timeZone: 'Europe/Paris', hour: 'numeric', hourCycle: 'h23' }).format(maintenant.value))
)

function estHeureCourante(h: number): boolean {
  return heureParis.value === h
}

function couleurCellule(c: ParAsset, h: number): string {
  const heure = c.heures?.find(x => x.heure === h)
  const vol = heure?.vol_pct ?? c.top.find(t => h >= t.debut && h < t.fin)?.vol_pct
  if (vol === undefined) return 'rgba(255,255,255,0.04)'
  const toutes = c.heures ?? []
  const volMax = toutes.length > 0
    ? Math.max(...toutes.map(x => x.vol_pct), 0.001)
    : Math.max(...c.top.map(t => t.vol_pct), 0.001)
  const ratio = vol / volMax
  if (ratio < 0.30) return 'rgba(34,197,94,0.20)'
  if (ratio < 0.45) return 'rgba(34,197,94,0.45)'
  if (ratio < 0.60) return 'rgba(34,197,94,0.70)'
  if (ratio < 0.75) return 'rgba(250,204,21,0.65)'
  if (ratio < 0.88) return 'rgba(249,115,22,0.78)'
  return 'rgba(239,68,68,0.88)'
}

onMounted(() => {
  void charger()
  poll = setInterval(() => {
    maintenant.value = new Date()
    void charger()
  }, 30_000)
})

onUnmounted(() => {
  if (poll !== null) clearInterval(poll)
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
