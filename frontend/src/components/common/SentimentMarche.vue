<template>
  <div class="glass-card p-4">
    <!-- En-tête -->
    <div class="flex items-center justify-between mb-3">
      <div>
        <p class="text-[11px] font-semibold text-white uppercase tracking-widest">Sentiment de Marché</p>
        <p class="text-[10px] text-slate-400">Jauges : réf. veille · Marchés : aujourd'hui</p>
      </div>
      <div v-if="chargement" class="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
    </div>

    <!-- Jauge circulaire composite (global) + 4 mini-jauges par classe -->
    <div v-if="composite" class="mb-3 pb-3 border-b border-white/10">
      <div class="flex items-center gap-4">
        <!-- Jauge circulaire globale (conic-gradient) -->
        <div class="relative flex-shrink-0">
          <div class="jauge-circulaire" :style="styleJauge(globalScore)">
            <div class="jauge-trou">
              <span class="text-2xl font-bold tabular-nums" :class="couleurScore(globalScore)">{{ Math.round(globalScore) }}</span>
            </div>
          </div>
        </div>
        <div class="flex-1 min-w-0">
          <p class="text-sm font-semibold" :class="couleurScore(globalScore)">{{ labelSentiment(globalScore) }}</p>
          <p class="text-[10px] text-slate-400 leading-tight mt-0.5">
            <span v-if="composite.cnn_fg != null">CNN {{ Math.round(composite.cnn_fg) }} · </span>
            <span v-if="composite.fear_greed != null">F&amp;G {{ Math.round(composite.fear_greed) }} · </span>
            <span v-if="composite.vix_brut != null">VIX {{ composite.vix_brut.toFixed(1) }}</span>
          </p>
        </div>
      </div>

      <!-- 4 mini-jauges par classe -->
      <div class="grid grid-cols-4 gap-2 mt-3">
        <div v-for="c in classes" :key="c.key" class="text-center">
          <div class="mini-jauge mb-1">
            <div class="mini-jauge-fill" :style="{ width: c.score + '%', background: fondScore(c.score) }" />
          </div>
          <p class="text-[9px] uppercase tracking-wide text-slate-500">{{ c.label }}</p>
          <p class="text-xs font-semibold tabular-nums" :class="couleurScore(c.score)">{{ Math.round(c.score) }}</p>
        </div>
      </div>
    </div>

    <p v-if="erreur" class="text-xs text-red-400">Données indisponibles</p>

    <template v-if="data">
      <div class="space-y-2.5 text-xs">
        <!-- USA -->
        <div>
          <p class="text-slate-500 mb-0.5">🇺🇸 USA</p>
          <div v-for="e in data.usa" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- EUROPE -->
        <div>
          <p class="text-slate-500 mb-0.5">🇪🇺 EUROPE</p>
          <div v-for="e in data.europe" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- MATIÈRES PREMIÈRES -->
        <div>
          <p class="text-slate-500 mb-0.5">⛏️ MATIÈRES PREMIÈRES</p>
          <div v-for="e in data.matieres_premieres" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct, SEUIL_MATIERES) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct, SEUIL_MATIERES)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- CRYPTOS -->
        <div>
          <p class="text-slate-500 mb-0.5">₿ CRYPTOS</p>
          <div v-for="e in data.cryptos" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="text-sm leading-none">{{ bille(e.variation_pct, SEUIL_CRYPTOS) }}</span>
            <span class="text-slate-200">{{ e.nom }}</span>
            <span class="text-slate-400 tabular-nums text-[10px]">{{ formatPrix(e.prix) }}</span>
            <span class="ml-auto tabular-nums" :class="couleur(e.variation_pct, SEUIL_CRYPTOS)">
              {{ e.variation_pct > 0 ? '+' : '' }}{{ e.variation_pct.toFixed(2) }}%
            </span>
          </div>
        </div>

        <!-- VIX -->
        <div v-if="data.vix != null" class="pt-2 border-t border-white/10 flex items-center gap-2">
          <span class="text-slate-400">VIX {{ data.vix.toFixed(1) }}</span>
          <span
            class="font-semibold"
            :class="data.vix >= 30 ? 'text-red-400' : data.vix >= 20 ? 'text-orange-400' : 'text-emerald-400'"
          >
            {{ data.vix >= 30 ? '⚠ Peur' : data.vix >= 20 ? '⚡ Volatil' : '✓ Stable' }}
          </span>
        </div>
      </div>
    </template>

    <!-- Skeleton si premier chargement -->
    <div v-if="!data && !composite" class="space-y-2 animate-pulse">
      <div v-for="i in 8" :key="i" class="h-3 rounded bg-white/10" :style="{ width: `${55 + (i % 3) * 15}%` }" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { formatParis } from '@/utils/date'
import { useSentimentStore } from '@/stores/sentiment.store'

const store = useSentimentStore()
const { data, chargement, erreur, composite } = storeToRefs(store)

const globalScore = computed(() => composite.value?.global ?? 50)

const classes = computed(() => {
  const c = composite.value
  return [
    { key: 'crypto', label: 'Crypto', score: c?.crypto ?? 50 },
    { key: 'forex', label: 'Forex', score: c?.forex ?? 50 },
    { key: 'metaux', label: 'Métaux', score: c?.metaux ?? 50 },
    { key: 'indices', label: 'Indices', score: c?.indices ?? 50 },
  ]
})

const dateAffichee = computed(() => {
  if (!data.value) return ''
  return formatParis(new Date(data.value.date), {
    day: '2-digit', month: '2-digit', year: 'numeric',
  })
})

/// Construit le conic-gradient de la jauge circulaire globale.
function styleJauge(score: number) {
  const col = couleurHex(score)
  return {
    background: `conic-gradient(${col} ${score * 3.6}deg, rgba(255,255,255,0.08) ${score * 3.6}deg)`,
  }
}

/// Couleur hex selon le score (rouge < 40, jaune 40-60, vert > 60).
function couleurHex(v: number): string {
  if (v >= 60) return '#34d399' // emerald-400
  if (v < 40) return '#f87171' // red-400
  return '#fbbf24' // amber-400
}

function couleurScore(v: number): string {
  if (v >= 60) return 'text-emerald-400'
  if (v < 40) return 'text-red-400'
  return 'text-amber-400'
}

function fondScore(v: number): string {
  return couleurHex(v)
}

/// Label textuel du sentiment global.
function labelSentiment(v: number): string {
  if (v >= 80) return 'Greed extrême'
  if (v >= 60) return 'Greed'
  if (v > 40) return 'Neutre'
  if (v > 20) return 'Peur'
  return 'Peur extrême'
}

/// Échelle des pastilles par classe (décision 2026-08-18) : un mouvement
/// n'est significatif qu'au-delà de la volatilité normale de sa classe.
/// Avant : ±0,3 % partout — un indice quasi plat (-0,4 %) s'affichait rouge.
const SEUIL_INDICES = 1.0; // indices boursiers (ajusté propriétaire 2026-08-18)
const SEUIL_MATIERES = 0.75; // or, pétrole, agriculture
const SEUIL_CRYPTOS = 2.0; // Bitcoin : ±2 % est un jour calme

function bille(v: number, seuil = SEUIL_INDICES): string {
  if (v > seuil) return '🟢'
  if (v < -seuil) return '🔴'
  return '🔵'
}

function couleur(v: number, seuil = SEUIL_INDICES): string {
  if (v > seuil) return 'text-emerald-400'
  if (v < -seuil) return 'text-red-400'
  return 'text-sky-400' // neutre : suit la pastille bleue (gris illisible)
}

function formatPrix(p: number): string {
  return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 0 }).format(p) + ' $'
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }

/* Jauge circulaire globale (60×60) */
.jauge-circulaire {
  width: 60px;
  height: 60px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
}
.jauge-trou {
  width: 46px;
  height: 46px;
  border-radius: 50%;
  background: rgba(15, 23, 42, 0.9); /* slate-900 */
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Mini-jauges horizontales par classe */
.mini-jauge {
  height: 5px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.08);
  overflow: hidden;
}
.mini-jauge-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease;
}
</style>
