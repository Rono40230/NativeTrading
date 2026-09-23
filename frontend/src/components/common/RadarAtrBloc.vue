<template>
  <div class="glass-card px-4 py-2 flex flex-col gap-1.5">
    <!-- En-tête cliquable repliable (même motif que Créneaux de volatilité) —
         remplace la page /heatmap et le bouton ⚡ (décision 22/09). -->
    <div class="flex items-center justify-between shrink-0 gap-2 cursor-pointer select-none" @click="basculer()">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">
        <span class="inline-block transition-transform" :class="ouvert ? 'rotate-90' : ''">▸</span>
        🌡️ Radar ATR temps réel
        <span v-if="!ouvert && classement.length" class="text-white font-normal normal-case tracking-normal">
          · top {{ classement[0].asset }} {{ classement[0].tf }}
        </span>
      </p>
      <span class="text-[9px] text-white shrink-0">MAJ 60 s</span>
    </div>

    <template v-if="ouvert">
      <!-- Légende des teintes (le bouton d'analyse LLM a été retiré le 23/09 :
           son résultat éphémère dupliquait le pipeline créneaux IA matinal,
           décision propriétaire). -->
      <div class="flex items-center gap-2 flex-wrap text-[9px] text-white">
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#10b981" /> calme &lt;80</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#f59e0b" /> modéré</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#ef4444" /> élevé &gt;120</span>
      </div>

      <div v-if="confluences.length" class="rounded-lg border border-orange-500/40 bg-orange-500/10 px-2 py-1.5 flex flex-wrap gap-1.5">
        <span
          v-for="c in confluences" :key="c.asset + c.tf"
          class="flex items-center gap-1 bg-orange-500/15 border border-orange-500/30 rounded px-1.5 py-0.5 text-[9px]"
        >
          <span class="font-bold text-white">{{ c.asset }}</span>
          <span class="text-white bg-white/10 px-1 rounded font-mono">{{ c.tf }}</span>
          <span class="text-orange-300 font-mono">{{ c.atrRatio.toFixed(0) }}%</span>
        </span>
      </div>

      <div v-if="chargement" class="text-center text-white text-xs py-3">Calcul…</div>
      <div v-else-if="!classement.length" class="text-center text-white text-xs py-3">Aucune donnée</div>

      <!-- Classement : les cellules les plus volatiles d'abord (le tableau
           complet asset×TF ne tenait pas dans la colonne latérale). -->
      <div v-else class="flex flex-col gap-1 max-h-[38vh] overflow-y-auto pr-0.5">
        <div
          v-for="i in classement.slice(0, 12)" :key="i.cle"
          class="rounded-lg px-2 py-1 flex items-center gap-1.5 border"
          :style="ligneStyle(i.atr)"
          :title="`ATR ${i.asset} ${i.tf} : ${i.atr.toFixed(1)} % de la moyenne`"
        >
          <span class="text-[11px] font-bold text-white">{{ i.asset }}</span>
          <span class="text-[9px] text-white bg-white/10 px-1 rounded font-mono">{{ i.tf }}</span>
          <span class="ml-auto text-[10px] font-mono text-white">{{ i.atr.toFixed(0) }} %</span>
          <span class="text-[9px] text-white/80">{{ libelle(i.atr) }}</span>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { apiService } from '@/services/api.service'
import type { Candle } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'
import { useHeatmapConfluence } from '@/composables/useHeatmapConfluence'

/// Radar ATR temps réel en bloc repliable du dashboard (ex-page /heatmap,
/// décision 22/09) : volatilité actuelle vs moyenne par asset × timeframe.
/// Chargé UNIQUEMENT ouvert (216 requêtes de bougies par cycle — zéro coût
/// replié), rafraîchi toutes les 60 s tant que le bloc reste ouvert.
const TIMEFRAMES = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']

const assetsStore = useAssetsStore()
const { confluences, detecterConfluences } = useHeatmapConfluence()

const ouvert = ref(false)
const chargement = ref(false)
const donnees = ref<Record<string, number>>({})
const assets = computed(() => assetsStore.assets.map(a => a.id))

const classement = computed(() => {
  const items = assets.value.flatMap(a => TIMEFRAMES.map(tf => ({
    cle: `${a}_${tf}`, asset: a, tf, atr: donnees.value[`${a}_${tf}`] ?? 0,
  })))
  return items.filter(i => i.atr > 0).sort((a, b) => b.atr - a.atr)
})

function basculer() {
  ouvert.value = !ouvert.value
}

function libelle(ratio: number): string {
  if (ratio < 80) return 'calme'
  if (ratio < 120) return 'modéré'
  return 'élevé'
}

function ligneStyle(ratio: number) {
  const couleur = ratio < 80 ? '#10b981' : ratio < 120 ? '#f59e0b' : '#ef4444'
  return {
    background: `${couleur}20`,
    borderColor: ratio >= 120 ? '#ef4444' : `${couleur}55`,
  }
}

function calcAtr(candles: Candle[], periode = 14): number {
  if (candles.length < 2) return 0
  const trs = candles.slice(1).map((c, i) => {
    const prev = candles[i].close
    return Math.max(c.high - c.low, Math.abs(c.high - prev), Math.abs(c.low - prev))
  })
  const fenetre = trs.slice(-Math.min(periode, trs.length))
  return fenetre.reduce((s, v) => s + v, 0) / fenetre.length
}

function calcAtrRatio(candles: Candle[]): number {
  if (candles.length < 30) return 0
  const atrActuel = calcAtr(candles.slice(-7), 6)
  const atrMoyen = calcAtr(candles, Math.min(candles.length - 1, 60))
  return atrMoyen > 0 ? (atrActuel / atrMoyen) * 100 : 100
}

async function actualiser() {
  chargement.value = true
  const paires = assets.value.flatMap(a => TIMEFRAMES.map(tf => ({ a, tf })))
  const resultats = await Promise.allSettled(
    paires.map(({ a, tf }) => apiService.getCandles(a, tf, 80).then(c => ({ a, tf, c })))
  )
  for (const r of resultats) {
    if (r.status === 'fulfilled') {
      const { a, tf, c } = r.value
      donnees.value[`${a}_${tf}`] = calcAtrRatio(c)
    }
  }
  chargement.value = false
  detecterConfluences(classement.value)
}

/// Le cycle 60 s ne vit que pendant l'ouverture — replié, le bloc ne coûte rien.
let intervalId: ReturnType<typeof setInterval> | null = null
watch(ouvert, async (ouvertMaintenant) => {
  if (ouvertMaintenant) {
    await actualiser()
    intervalId = setInterval(actualiser, 60_000)
  } else if (intervalId) {
    clearInterval(intervalId)
    intervalId = null
  }
})
onMounted(() => { if (!assetsStore.assets.length) void assetsStore.chargerAssets() })
onUnmounted(() => { if (intervalId) clearInterval(intervalId) })
</script>
