<template>
  <!-- ⏰ Créneaux de volatilité — fenêtres horaires Paris les plus actives
       par asset (plages fusionnées : « 15h→18h » plutôt que 3 pastilles). -->
  <div class="glass-card p-3 pt-2 flex flex-col gap-2 min-h-0 shrink-0">
    <div class="shrink-0">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">⏰ Créneaux de volatilité</p>
      <p class="text-[9px] text-slate-500">fenêtres actives · heures Paris · 24 mois</p>
    </div>

    <div v-if="chargement" class="text-center text-slate-500 text-xs py-3">Calcul…</div>
    <div v-else-if="!creneaux.length" class="text-center text-slate-500 text-xs py-3">Aucune fenêtre</div>

    <div v-else class="space-y-1.5 overflow-y-auto">
      <div v-for="c in creneaux" :key="c.asset" class="flex items-center gap-2">
        <span class="w-14 shrink-0 font-semibold text-white text-xs">{{ c.asset }}</span>
        <div class="flex gap-1.5 flex-wrap flex-1 min-w-0">
          <span v-for="t in c.top" :key="t.debut"
            :title="`${t.vol_pct.toFixed(3)} % / bougie M15 · fiabilité ${Math.round(t.fiabilite * 100)}%`"
            :class="[
              'px-2 py-0.5 rounded-md text-[10px] font-mono tabular-nums border transition-colors whitespace-nowrap',
              estEnCours(t)
                ? 'bg-cyan-500/25 border-cyan-400/60 text-cyan-100'
                : 'bg-white/5 border-white/10 text-slate-300',
            ]"
          >{{ plage(t) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

interface Plage { debut: number; fin: number; vol_pct: number; fiabilite: number; nb_heures: number }
interface ParAsset { asset: string; top: Plage[] }

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
  Number(new Intl.DateTimeFormat('fr-FR', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(maintenant.value))
)

/// La plage couvre-t-elle l'heure courante ?
function estEnCours(t: Plage): boolean {
  const h = heureParis.value
  return h >= t.debut && h < t.fin
}

/// Libellé : « 15h→18h » ou « 12h→13h » (1 heure) ou « 9h » (début=fin-1).
function plage(t: Plage): string {
  const d = String(t.debut).padStart(2, '0')
  const f = String(t.fin).padStart(2, '0')
  return t.nb_heures === 1 ? `${d}h` : `${d}h→${f}h`
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
