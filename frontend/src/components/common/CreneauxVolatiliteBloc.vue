<template>
  <!-- ⏰ Créneaux de volatilité — heures Paris les plus actives par asset
       (stats 24 mois glissants, job backend quotidien). Créneau en cours
       surligné, prochain avec compte à rebours. -->
  <div class="glass-card p-3 flex flex-col gap-2 min-h-0">
    <div class="shrink-0">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">⏰ Créneaux de volatilité</p>
      <p class="text-[9px] text-slate-500">heures Paris · 24 mois</p>
    </div>

    <div v-if="chargement" class="text-center text-slate-500 text-xs py-3">Calcul des créneaux…</div>
    <div v-else-if="!creneaux.length" class="text-center text-slate-500 text-xs py-3">Aucun créneau calculé</div>

    <div v-else class="space-y-1.5 overflow-y-auto">
      <div v-for="c in creneaux" :key="c.asset" class="flex items-center gap-2">
        <span class="w-16 shrink-0 font-semibold text-white text-xs">{{ c.asset }}</span>
        <div class="flex gap-1.5 flex-wrap">
          <span v-for="t in c.top" :key="t.heure"
            :title="`${t.vol_pct.toFixed(3)} % / bougie M15 en moyenne`"
            :class="[
              'px-1.5 py-0.5 rounded text-[10px] font-mono tabular-nums border transition-colors',
              estEnCours(t.heure)
                ? 'bg-cyan-500/25 border-cyan-400/60 text-cyan-100'
                : estProchain(t, c)
                  ? 'bg-amber-500/15 border-amber-400/40 text-amber-200'
                  : 'bg-white/5 border-white/10 text-slate-300',
            ]"
          >{{ label(t) }}</span>
        </div>
      </div>
    </div>

    <p v-if="prochainGlobal" class="text-[10px] text-slate-500 mt-auto">
      Prochain : <span class="text-amber-300 font-semibold">{{ prochainGlobal.nom }}</span> dans {{ compteRebours }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

interface Creneau { heure: number; vol_pct: number; fiabilite: number }
interface ParAsset { asset: string; top: Creneau[] }

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

/// Heure Paris courante (0-23).
const heureParis = computed(() =>
  new Intl.DateTimeFormat('fr-FR', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(maintenant.value)
)

function estEnCours(heure: number): boolean {
  return Number(heureParis.value) === heure
}

/// Le premier créneau du top de l'asset qui viendra après l'heure courante
/// (cyclique sur 24h) — celui dont le compte à rebours est le plus court.
function estProchain(t: Creneau, c: ParAsset): boolean {
  const h = Number(heureParis.value)
  const candidats = c.top.filter(x => !estEnCours(x.heure))
  if (!candidats.length) return false
  const distances = candidats.map(x => ({ x, d: (x.heure - h + 24) % 24 }))
  distances.sort((a, b) => a.d - b.d)
  return distances[0].x.heure === t.heure
}

function label(t: Creneau): string {
  const h = t.heure
  return `${String(h).padStart(2, '0')}h·${Math.round(t.fiabilite * 100)}%`
}

/// Prochain créneau toutes assets confondues + compte à rebours.
const prochainGlobal = computed(() => {
  const h = Number(heureParis.value)
  let meilleur: { nom: string; minutes: number } | null = null
  for (const c of creneaux.value) {
    for (const t of c.top) {
      const d = (t.heure - h + 24) % 24
      if (d === 0) continue
      const minutes = Math.max(0, d * 60 - maintenant.value.getMinutes())
      if (!meilleur || minutes < meilleur.minutes) {
        meilleur = { nom: `${c.asset} ${String(t.heure).padStart(2, '0')}h`, minutes }
      }
    }
  }
  return meilleur
})

const compteRebours = computed(() => {
  const p = prochainGlobal.value
  if (!p || !isFinite(p.minutes) || p.minutes < 0) return '—'
  const h = Math.floor(p.minutes / 60)
  const m = p.minutes % 60
  return h > 0 ? `${h}h${String(m).padStart(2, '0')}` : `${m} min`
})

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
