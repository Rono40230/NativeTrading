<template>
  <!-- 📍 Zones à l'approche (spec docs/spec_alertes_zones_smc.md) : face app
       du watcher — zones d'achat/vente SMC fraîches que le prix live va
       toucher (≤ seuil × ATR). Polling 30 s, état mémoire backend. -->
  <div class="glass-card p-2.5 flex flex-col gap-1.5">
    <div class="flex items-center gap-2 flex-wrap">
      <p class="text-xs font-semibold text-white">📍 Zones à l'approche</p>
      <span class="text-[10px] text-white/50" title="Distance au bord de la zone ≤ seuil × ATR du TF — zones jamais touchées des couples SMC armés uniquement">zones fraîches, couples armés · seuil × ATR</span>
      <span v-if="enBande.length" class="ml-auto text-[10px] font-mono px-1.5 py-0.5 rounded-full border border-amber-500/40 bg-amber-500/10 text-amber-300">{{ enBande.length }} en approche</span>
    </div>

    <div v-if="enBande.length" class="flex flex-wrap gap-1.5">
      <div
        v-for="z in enBande" :key="`${z.asset}-${z.tf}-${z.ts_zone}`"
        class="flex items-center gap-1.5 text-[11px] px-2 py-1 rounded-lg border"
        :class="z.achat
          ? 'border-emerald-500/40 bg-emerald-500/10'
          : 'border-red-500/40 bg-red-500/10'"
        :title="`Zone ${z.achat ? 'ACHAT' : 'VENTE'} [${z.zone_haut.toFixed(2)} ; ${z.zone_bas.toFixed(2)}] — prix ${z.prix.toFixed(2)}, à ${z.distance_atr.toFixed(2)} × ATR (${z.distance_pct.toFixed(2)} %)`"
      >
        <span class="font-mono font-bold text-white">{{ z.asset }}</span>
        <span class="text-white/60 font-mono">{{ z.tf }}</span>
        <span :class="z.achat ? 'text-emerald-300' : 'text-red-300'">{{ z.achat ? '▲ achat' : '▼ vente' }}</span>
        <span class="font-mono text-white/70">{{ z.zone_bas.toFixed(2) }}–{{ z.zone_haut.toFixed(2) }}</span>
        <span class="font-mono" :class="z.distance_atr <= 0.1 ? 'text-amber-300 font-bold' : 'text-white/50'">{{ z.distance_atr.toFixed(2) }} × ATR</span>
        <span v-if="z.alerte_le" class="text-[9px] text-white/40" title="Alerte déjà envoyée pour cette approche (Telegram + ici)">🔔 {{ heure(z.alerte_le) }}</span>
      </div>
    </div>
    <p v-else class="text-[11px] text-white/50 py-1">
      Aucune zone en approche — le watcher surveille chaque prix des couples SMC armés et
      préviendra ici (et sur Telegram) dès qu'une zone fraîche est à {{ seuilInvite }}.
    </p>

    <p v-if="recentes.length" class="text-[9px] text-white/40 truncate" :title="recentes.map(r => `${r.asset} ${r.tf}`).join(', ')">
      Récemment alertées (2 h) : {{ recentes.map(r => `${r.asset} ${r.tf}`).join(' · ') }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { http } from '@/services/http.client'

interface ZoneWatch {
  asset: string
  tf: string
  achat: boolean
  zone_haut: number
  zone_bas: number
  distance_atr: number
  distance_pct: number
  prix: number
  ts_zone: number
  alerte_le: number | null
}

const enBande = ref<ZoneWatch[]>([])
const recentes = ref<ZoneWatch[]>([])
const seuilInvite = '≤ 0,25 × ATR'
let minuteur: ReturnType<typeof setInterval> | null = null

async function charger() {
  try {
    const r = await http.get<{ en_bande: ZoneWatch[]; recentes: ZoneWatch[] }>('/api/smc/zones-approche')
    enBande.value = r.data?.en_bande ?? []
    recentes.value = r.data?.recentes ?? []
  } catch { /* watcher démarré après l'API : réessaiera au prochain tick */ }
}

/// Heure HH:MM depuis un epoch secondes.
function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

onMounted(() => {
  void charger()
  minuteur = setInterval(() => void charger(), 30_000)
})
onUnmounted(() => { if (minuteur) clearInterval(minuteur) })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
