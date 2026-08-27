<template>
  <!-- ⏰ Créneaux de volatilité — heatmap 24h par asset (heures Paris).
       Matin à gauche (00→12h), après-midi à droite (12→24h). L'intensité
       de couleur = volatilité relative à l'asset lui-même. -->
  <div class="glass-card px-4 py-2 flex flex-col gap-1.5">
    <div class="flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">⏰ Créneaux de volatilité</p>
      <span class="text-[9px] text-white">heures Paris · fenêtres actives · 24 mois glissants</span>
    </div>

    <div v-if="chargement" class="text-center text-slate-500 text-xs py-3">Calcul…</div>
    <div v-else-if="!creneaux.length" class="text-center text-slate-500 text-xs py-3">Aucune fenêtre</div>

    <div v-else class="flex flex-col gap-0.5">
      <!-- En-têtes : MATIN | APRÈS-MIDI -->
      <div class="flex items-center gap-1 text-[8px] text-slate-500 font-semibold uppercase tracking-wide">
        <span class="w-14 shrink-0"></span>
        <span class="flex-1 text-center text-white">Matin (00h→12h)</span>
        <span class="w-px bg-white/10 self-stretch"></span>
        <span class="flex-1 text-center text-white">Après-midi (12h→24h)</span>
      </div>

      <!-- Heures : 00..11 | 12..23 -->
      <div class="flex items-center gap-[2px] text-[7px] text-white font-mono tabular-nums">
        <span class="w-14 shrink-0"></span>
        <div class="flex-1 flex gap-[2px]">
          <span v-for="h in 12" :key="'m' + h" class="flex-1 text-center">{{ String(h - 1).padStart(2, '0') }}</span>
        </div>
        <span class="w-px bg-white/10 self-stretch"></span>
        <div class="flex-1 flex gap-[2px]">
          <span v-for="h in 24" :key="'a' + h" v-show="h >= 12" class="flex-1 text-center">{{ h }}</span>
        </div>
      </div>

      <!-- Une rangée par asset -->
      <div v-for="c in creneaux" :key="c.asset" class="flex items-center gap-1">
        <span class="w-14 shrink-0 text-[10px] font-semibold text-white truncate" :title="c.asset">{{ c.asset }}</span>
        <div class="flex-1 flex gap-[2px] h-6">
          <div v-for="h in 24" :key="h"
            class="flex-1 rounded-[2px] transition-colors cursor-help"
            :style="{ backgroundColor: couleurCellule(c, h - 1) }"
            :class="{ 'ring-2 ring-red-500 shadow-[0_0_6px_rgba(239,68,68,0.5)]': estHeureCourante(h - 1) }"
            :title="titreCellule(c, h - 1)"
          />
          <!-- Séparateur midi inséré après la 12e cellule via rendu conditionnel -->
          <template v-if="false" />
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
        <span class="ml-auto flex items-center gap-1"><span class="w-3 h-2 rounded-[2px] ring-2 ring-red-500" /> heure en cours</span>
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

/// Heures individuelles renvoyées par le backend (recalculées côté client
/// à partir des plages pour la heatmap).
interface HeureData { vol: number; fiab: number }
const heuresParAsset = ref<Record<string, Record<number, HeuneData>>>({})

interface HeuneData { vol: number; fiab: number }

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

function estHeureCourante(h: number): boolean {
  return heureParis.value === h
}

/// Couleur d'une cellule : PALIERS discrets — vert → jaune → orange → rouge
/// (normalisé sur le max de vol de l'asset). Chaque palier est immédiatement
/// distinguable, contrairement à un dégradé d'une seule teinte.
function couleurCellule(c: ParAsset, h: number): string {
  const plage = c.top.find(t => h >= t.debut && h < t.fin)
  if (!plage) return 'rgba(255,255,255,0.04)'
  const volMax = Math.max(...c.top.map(t => t.vol_pct), 0.001)
  const ratio = plage.vol_pct / volMax
  if (ratio < 0.35) return 'rgba(34,197,94,0.35)'   // vert — calme
  if (ratio < 0.55) return 'rgba(34,197,94,0.65)'   // vert fort — modéré
  if (ratio < 0.75) return 'rgba(250,204,21,0.65)'  // jaune — actif
  if (ratio < 0.90) return 'rgba(249,115,22,0.75)'  // orange — très actif
  return 'rgba(239,68,68,0.85)'                      // rouge — bouillant
}

function titreCellule(c: ParAsset, h: number): string {
  const plage = c.top.find(t => h >= t.debut && h < t.fin)
  if (!plage) return `${c.asset} ${String(h).padStart(2, '0')}h — calme`
  return `${c.asset} ${String(h).padStart(2, '0')}h — ${plage.vol_pct.toFixed(3)}% · fiabilité ${Math.round(plage.fiabilite * 100)}%`
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
