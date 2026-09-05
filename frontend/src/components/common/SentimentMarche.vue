<template>
  <div class="glass-card p-3 pt-2">
    <!-- En-tête -->
    <div class="flex items-center justify-between mb-2 border-b border-white/10 pb-1">
      <div>
        <p class="text-[11px] font-semibold text-white uppercase tracking-widest">🌡️ Sentiment de Marché</p>
      </div>
      <div v-if="chargement" class="w-2 h-2 rounded-full bg-blue-500 animate-pulse" />
    </div>

    <p v-if="erreur" class="text-xs text-red-400">Données indisponibles</p>

    <template v-if="data">
      <div class="space-y-2.5 text-xs">
        <!-- En-tête des deux colonnes de variation -->
        <div class="flex items-center gap-2 text-[9px] uppercase tracking-wider text-white border-b border-white/5 pb-0.5">
          <span class="flex-1" />
          <span class="w-[64px] text-right">Cours</span>
          <span class="w-[64px] text-center">Veille</span>
          <span class="w-[64px] text-center">Jour</span>
        </div>
        <!-- USA -->
        <div>
          <p class="text-white mb-0.5">🇺🇸 USA</p>
          <div v-for="e in data.usa" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="flex-1 min-w-0 truncate text-white">{{ e.nom }}</span>
            <span
              class="w-[64px] text-right tabular-nums text-[11px] font-medium"
              :class="couleurCours(e.variation_pct)"
              :title="`Cours ${e.nom}`"
            >{{ prixFmt(e.prix) }}</span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_veille ?? e.variation_pct) }}</span>
              <span :class="couleur(e.variation_veille ?? e.variation_pct)">{{ pct(e.variation_veille ?? e.variation_pct) }}</span>
            </span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
              <span :class="couleur(e.variation_pct)">{{ pct(e.variation_pct) }}</span>
            </span>
          </div>
        </div>

        <!-- EUROPE -->
        <div>
          <p class="text-white mb-0.5">🇪🇺 EUROPE</p>
          <div v-for="e in data.europe" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="flex-1 min-w-0 truncate text-white">{{ e.nom }}</span>
            <span
              class="w-[64px] text-right tabular-nums text-[11px] font-medium"
              :class="couleurCours(e.variation_pct)"
              :title="`Cours ${e.nom}`"
            >{{ prixFmt(e.prix) }}</span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_veille ?? e.variation_pct) }}</span>
              <span :class="couleur(e.variation_veille ?? e.variation_pct)">{{ pct(e.variation_veille ?? e.variation_pct) }}</span>
            </span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
              <span :class="couleur(e.variation_pct)">{{ pct(e.variation_pct) }}</span>
            </span>
          </div>
        </div>

        <!-- MATIÈRES PREMIÈRES -->
        <div>
          <p class="text-white mb-0.5">⛏️ MATIÈRES PREMIÈRES</p>
          <div v-for="e in data.matieres_premieres" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="flex-1 min-w-0 truncate text-white">{{ e.nom }}</span>
            <span
              class="w-[64px] text-right tabular-nums text-[11px] font-medium"
              :class="couleurCours(e.variation_pct)"
              :title="`Cours ${e.nom}`"
            >{{ prixFmt(e.prix) }}</span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_veille ?? e.variation_pct) }}</span>
              <span :class="couleur(e.variation_veille ?? e.variation_pct)">{{ pct(e.variation_veille ?? e.variation_pct) }}</span>
            </span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
              <span :class="couleur(e.variation_pct)">{{ pct(e.variation_pct) }}</span>
            </span>
          </div>
        </div>

        <!-- CRYPTOS -->
        <div>
          <p class="text-white mb-0.5">₿ CRYPTOS</p>
          <div v-for="e in data.cryptos" :key="e.nom" class="flex items-center gap-2 py-0.5">
            <span class="flex-1 min-w-0 truncate text-white">{{ e.nom }}</span>
            <span
              class="w-[64px] text-right tabular-nums text-[11px] font-medium"
              :class="couleurCours(e.variation_pct)"
              :title="`Cours ${e.nom}`"
            >{{ prixFmt(e.prix) }}</span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_veille ?? e.variation_pct) }}</span>
              <span :class="couleur(e.variation_veille ?? e.variation_pct)">{{ pct(e.variation_veille ?? e.variation_pct) }}</span>
            </span>
            <span class="w-[64px] flex items-center justify-end gap-1 tabular-nums">
              <span class="text-sm leading-none">{{ bille(e.variation_pct) }}</span>
              <span :class="couleur(e.variation_pct)">{{ pct(e.variation_pct) }}</span>
            </span>
          </div>
        </div>

        <!-- VIX -->
        <div v-if="data.vix != null" class="pt-2 border-t border-white/10 flex items-center gap-2">
          <span class="text-white">VIX {{ data.vix.toFixed(1) }} <span class="text-[9px] text-white">(veille)</span></span>
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
    <div v-if="!data" class="space-y-2 animate-pulse">
      <div v-for="i in 8" :key="i" class="h-3 rounded bg-white/10" :style="{ width: `${55 + (i % 3) * 15}%` }" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useSentimentStore } from '@/stores/sentiment.store'

const store = useSentimentStore()
const { data, chargement, erreur } = storeToRefs(store)

/// Pastilles (règle propriétaire 26/08) : verte dès que la variation est
/// POSITIVE, bleue entre -0,50 % et 0 inclus, rouge sous -0,50 %.
function pct(v: number): string {
  return (v > 0 ? '+' : '') + v.toFixed(2) + '%'
}

/// Cours en direct : séparateur fin fr-FR, décimales adaptatives
/// (entier ≥ 1 000 — ex. BTC « 87 432 », S&P « 5 864 » ; 2 décimales sinon).
/// Cours coloré au mouvement du jour : vert en hausse, rouge en baisse,
/// neutre si strictement stable.
/// Échelle propriétaire : ≥ −1 % vert · −3 % à −1 % bleu · < −3 % rouge.
function couleurCours(v: number): string {
  if (v >= -1) return 'text-emerald-400'
  if (v >= -3) return 'text-blue-400'
  return 'text-red-400'
}

function prixFmt(v: number): string {
  if (v >= 1000) return new Intl.NumberFormat('fr-FR', { maximumFractionDigits: 0 }).format(v) + ' $'
  return new Intl.NumberFormat('fr-FR', { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v) + ' $'
}

/// Échelle propriétaire : ≥ −1 % vert · −3 % à −1 % bleu · < −3 % rouge.
function bille(v: number): string {
  if (v >= -1) return '🟢'
  if (v >= -3) return '🔵'
  return '🔴'
}

/// Échelle propriétaire : ≥ −1 % vert · −3 % à −1 % bleu · < −3 % rouge.
function couleur(v: number): string {
  if (v >= -1) return 'text-emerald-400'
  if (v >= -3) return 'text-blue-400'
  return 'text-red-400'
}

</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
