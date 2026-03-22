<template>
  <div class="glass-card p-4 flex flex-col h-full">
    <div class="flex items-center justify-between mb-3 shrink-0">
      <h3 class="text-sm font-semibold text-white">📅 Calendrier économique</h3>
      <button
        @click="charger"
        class="text-xs text-slate-400 hover:text-white transition-colors px-2 py-0.5 rounded hover:bg-white/5"
        title="Actualiser"
      >↻</button>
    </div>

    <div v-if="chargement" class="text-slate-400 text-xs text-center py-3 shrink-0">Chargement…</div>
    <div v-else-if="annonces.length === 0" class="text-slate-500 text-xs text-center py-3 shrink-0">
      Aucune annonce à venir (7j)
    </div>

    <div v-else class="flex flex-col gap-1.5 overflow-y-auto scroll-zone flex-1 pr-0.5">
      <div
        v-for="a in annonces"
        :key="a.id"
        class="relative group cursor-pointer"
        @click.stop="survolee = survolee === a.id ? null : a.id"
      >
        <!-- Carte évenement -->
        <div
          class="rounded-md border px-2.5 py-2 select-none transition-colors flex flex-col gap-1"
          :class="a.est_passe
            ? 'border-white/5 bg-white/5 opacity-50'
            : a.impact === 'High'
              ? 'border-red-500/20 bg-red-500/5 hover:bg-red-500/10'
              : 'border-orange-500/20 bg-orange-500/5 hover:bg-orange-500/10'"
        >
          <!-- Ligne 1 : badge + devise -->
          <div class="flex items-center gap-1.5">
            <span
              class="w-1.5 h-1.5 rounded-full shrink-0"
              :class="a.est_passe ? 'bg-slate-600' : a.impact === 'High' ? 'bg-red-400' : 'bg-orange-400'"
            />
            <span class="text-[11px] font-mono font-bold text-slate-300">{{ a.devise }}</span>
            <span v-if="a.est_passe" class="text-[9px] text-slate-500 border border-slate-600/40 rounded-full px-1.5 py-0.5 leading-none">Terminé</span>
            <span class="ml-auto text-[10px] font-semibold shrink-0" :class="couleurCountdown(a.date_heure)">{{ countdown(a.date_heure) }}</span>
          </div>
          <!-- Ligne 2 : titre complet sur 2 lignes max -->
          <p class="text-[11px] text-white leading-snug line-clamp-2">{{ a.titre }}</p>
        </div>

        <!-- Tooltip détail au survol -->
        <Transition name="fade">
          <div
            v-if="survolee === a.id"
            class="tooltip-detail"
            @click.stop
          >
            <p class="text-[10px] font-semibold text-white mb-1.5 leading-snug">{{ a.titre }}</p>
            <div class="flex items-center gap-3 text-[10px] text-slate-400 mb-1">
              <span>🕐 {{ formatHeureLocale(a.date_heure) }}</span>
              <span class="text-slate-600">·</span>
              <span>{{ formatUTC(a.date_heure) }} UTC</span>
            </div>
            <div class="flex gap-3 text-[10px] text-slate-400 mb-1.5">
              <span>Préc: <span class="text-white">{{ a.precedent ?? '—' }}</span></span>
              <span>Prévis: <span class="text-white">{{ a.prevision ?? '—' }}</span></span>
            </div>
            <span
              class="text-[10px] font-bold"
              :class="couleurCountdown(a.date_heure)"
            >{{ countdown(a.date_heure) }}</span>
          </div>
        </Transition>
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
const survolee = ref<string | null>(null)
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

function fermerSurvolee() { survolee.value = null }

onMounted(async () => {
  await charger()
  verifierAlertes()
  intervalle = setInterval(verifierAlertes, 60_000)
  document.addEventListener('click', fermerSurvolee)
})

onUnmounted(() => {
  if (intervalle) clearInterval(intervalle)
  document.removeEventListener('click', fermerSurvolee)
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}
.scroll-zone {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.1) transparent;
}
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 2px; }

.tooltip-detail {
  @apply absolute bottom-full left-0 z-50 mb-1.5 w-52
         rounded-xl border border-white/15 bg-[#0f1629]
         p-3 shadow-2xl pointer-events-none;
  min-width: 13rem;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.12s ease, transform 0.12s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
