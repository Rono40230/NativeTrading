<template>
  <div class="glass-card p-5">
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-sm font-semibold text-white">📅 Calendrier économique</h3>
      <button
        @click="charger"
        class="text-xs text-slate-400 hover:text-white transition-colors px-2 py-1 rounded hover:bg-white/5"
        title="Actualiser"
      >↻</button>
    </div>

    <div v-if="chargement" class="text-slate-400 text-xs text-center py-6">
      Chargement…
    </div>
    <div v-else-if="annonces.length === 0" class="text-slate-500 text-xs text-center py-6">
      Aucune annonce à venir (7 prochains jours)
    </div>

    <div v-else class="space-y-2">
      <div
        v-for="a in annonces"
        :key="a.id"
        class="border border-white/5 rounded-lg p-3 space-y-1.5"
        :class="a.impact === 'High' ? 'bg-red-500/5' : 'bg-orange-500/5'"
      >
        <!-- Ligne 1 : badge impact + devise + titre -->
        <div class="flex items-center gap-2 min-w-0">
          <span
            class="text-[10px] font-bold px-1.5 py-0.5 rounded shrink-0"
            :class="a.impact === 'High'
              ? 'bg-red-500/25 text-red-400'
              : 'bg-orange-500/25 text-orange-400'"
          >{{ a.impact === 'High' ? '● HIGH' : '● MED' }}</span>
          <span class="text-xs font-mono font-bold text-slate-300 shrink-0">{{ a.devise }}</span>
          <span class="text-xs text-white truncate">{{ a.titre }}</span>
        </div>

        <!-- Ligne 2 : horaires + countdown -->
        <div class="flex items-center gap-2 text-[10px] text-slate-400 flex-wrap">
          <span>{{ formatHeureLocale(a.date_heure) }}</span>
          <span class="text-slate-600">·</span>
          <span>{{ formatUTC(a.date_heure) }} UTC</span>
          <span
            class="ml-auto font-semibold shrink-0"
            :class="couleurCountdown(a.date_heure)"
          >{{ countdown(a.date_heure) }}</span>
        </div>

        <!-- Ligne 3 : précédent / prévision -->
        <div class="flex gap-4 text-[10px] text-slate-500">
          <span>Préc: {{ a.precedent ?? '—' }}</span>
          <span>Prévis: {{ a.prevision ?? '—' }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { AnnonceCalendrier } from '@/services/api.types'
import { useAlerteStore } from '@/stores/alerte.store'

const alerteStore = useAlerteStore()
const annonces = ref<AnnonceCalendrier[]>([])
const chargement = ref(false)
const annoncesAlertees = new Set<string>()

async function charger() {
  chargement.value = true
  try {
    annonces.value = await apiService.obtenirCalendrier(7)
  } catch {
    // Dégradation silencieuse — liste vide
  } finally {
    chargement.value = false
  }
}

function formatHeureLocale(iso: string): string {
  return new Date(iso).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function formatUTC(iso: string): string {
  const d = new Date(iso)
  return `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`
}

function countdown(iso: string): string {
  const diffMs = new Date(iso).getTime() - Date.now()
  if (diffMs <= 0) return 'En cours'
  const diffMin = Math.floor(diffMs / 60_000)
  if (diffMin < 60) return `dans ${diffMin}min`
  const h = Math.floor(diffMin / 60)
  const m = diffMin % 60
  if (h < 24) return `dans ${h}h${m > 0 ? ` ${m}min` : ''}`
  const j = Math.floor(h / 24)
  return j === 1 ? 'demain' : `dans ${j} jours`
}

function couleurCountdown(iso: string): string {
  const diffMin = (new Date(iso).getTime() - Date.now()) / 60_000
  if (diffMin > 0 && diffMin <= 15) return 'text-red-400 animate-pulse'
  if (diffMin <= 60) return 'text-orange-400'
  return 'text-slate-400'
}

function verifierAlertes() {
  for (const a of annonces.value) {
    if (a.impact !== 'High') continue
    const diffMin = (new Date(a.date_heure).getTime() - Date.now()) / 60_000
    if (diffMin > 0 && diffMin <= 15 && !annoncesAlertees.has(a.id)) {
      annoncesAlertees.add(a.id)
      alerteStore.afficherAvertissement(
        `⚠️ ${a.titre} (${a.devise}) dans ${Math.round(diffMin)}min — Fort impact`
      )
    }
  }
}

let intervalle: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  await charger()
  verifierAlertes()
  intervalle = setInterval(verifierAlertes, 60_000)
})

onUnmounted(() => {
  if (intervalle) clearInterval(intervalle)
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
</style>
