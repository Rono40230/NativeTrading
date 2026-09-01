<template>
  <div class="flex flex-col gap-1.5">
    <div v-if="annonces.length" class="flex flex-col gap-1">
      <div v-for="a in annonces.slice(0, 6)" :key="a.ts"
           class="flex items-center gap-2 text-xs">
        <span class="text-amber-400">📅</span>
        <span class="text-gray-300 font-medium truncate">{{ a.titre || 'Annonce US' }}</span>
        <span class="text-gray-500">{{ heureLocale(a.ts) }}</span>
        <span class="ml-auto text-amber-300/90 font-mono text-[11px]">{{ compteARebours(a.ts) }}</span>
      </div>
    </div>
    <div v-else class="text-[11px] text-gray-600">Aucune annonce US forte à 7 jours</div>
    <div v-if="passes.length" class="text-[11px] text-emerald-400/80">
      {{ passes.length }} passe(s) en cours sur {{ [...new Set(passes.map(p => p.asset))].join(', ') }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

interface AgendaApi {
  annonces: { ts: number; titre: string; devise: string; actifs: string[] }[]
  passes: { asset: string; direction: string }[]
}
const agenda = ref<AgendaApi | null>(null)
const annonces = ref<AgendaApi['annonces']>([])
const passes = ref<AgendaApi['passes']>([])

function heureLocale(ts: number): string {
  return new Intl.DateTimeFormat('fr-FR', { hour: '2-digit', minute: '2-digit' }).format(new Date(ts * 1000))
}

function compteARebours(ts: number): string {
  const d = ts - Math.floor(Date.now() / 1000)
  if (d <= 0) return 'en cours'
  const j = Math.floor(d / 86400)
  const h = Math.floor((d % 86400) / 3600)
  const m = Math.floor((d % 3600) / 60)
  if (j > 0) return `J-${j} ${h}h`
  if (h > 0) return `${h}h${String(m).padStart(2, '0')}`
  return `${m} min`
}

async function charger() {
  try {
    const res = await http.get<AgendaApi>('/api/straddle/agenda')
    const d = res.data as AgendaApi
    annonces.value = d.annonces ?? []
    passes.value = d.passes ?? []
  } catch { /* agenda indisponible */ }
}

let minuteur: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void charger()
  minuteur = setInterval(charger, 60_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>
