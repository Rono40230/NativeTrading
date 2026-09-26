<template>
  <div class="glass-card px-4 py-2 flex flex-col gap-1.5">
    <!-- En-tête cliquable repliable (même motif que Créneaux de volatilité) —
         remplace la page /heatmap et le bouton ⚡ (décision 22/09). Le calcul
         vit dans useRadarAtr (source unique, partagée avec le badge bandeau). -->
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
      <!-- Légende des teintes — la mesure est corrigée de la saisonnalité
           jour × heure (6.6, 24/09) : 100 % = conforme à l'habitude de CET
           instant, pas à une moyenne a-saisonnière. H4 et + : l'heure n'y a
           pas de sens (fenêtres ≥ 24 h) → ratio brut vs 60 bougies. -->
      <div class="flex items-center gap-2 flex-wrap text-[9px] text-white">
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#10b981" /> calme &lt;80</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#f59e0b" /> modéré</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#ef4444" /> élevé &gt;120</span>
        <span class="text-white/50">vs habitude de cet instant (jour × heure, 24 mois)</span>
      </div>

      <!-- Anomalies saisonnières : volatilité anormale POUR CET INSTANT —
           remplace l'ancienne confluence « ratio brut chaud × slot
           habituellement chaud » qui tirait surtout sur les ouvertures de
           session (constat propriétaire 23/09). -->
      <div v-if="confluences.length" class="rounded-lg border border-orange-500/40 bg-orange-500/10 px-2 py-1.5 flex flex-wrap gap-1.5 items-center">
        <span class="text-[9px] text-orange-200/80 font-semibold uppercase tracking-wider">Anomalies</span>
        <span
          v-for="c in confluences" :key="c.asset + c.tf"
          class="flex items-center gap-1 bg-orange-500/15 border border-orange-500/30 rounded px-1.5 py-0.5 text-[9px]"
          :title="`${c.asset} ${c.tf} : ${c.atr.toFixed(0)} % de l'habitude de cet instant (brut ${c.brut.toFixed(0)} %)`"
        >
          <span class="font-bold text-white">{{ c.asset }}</span>
          <span class="text-white bg-white/10 px-1 rounded font-mono">{{ c.tf }}</span>
          <span class="text-orange-300 font-mono">{{ c.atr.toFixed(0) }}%</span>
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
          :title="titreLigne(i)"
        >
          <span class="text-[11px] font-bold text-white">{{ i.asset }}</span>
          <span class="text-[9px] text-white bg-white/10 px-1 rounded font-mono">{{ i.tf }}</span>
          <span class="ml-auto text-[10px] font-mono text-white">{{ i.atr.toFixed(0) }} %</span>
          <span class="text-[9px] text-white/80">{{ libelle(i.atr) }}</span>
        </div>
        <p class="text-[9px] text-white/40 pt-0.5">H4/D1/W1 : % de la moyenne des 60 dernières bougies (l'heure y est sans objet).</p>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useRadarAtr, type LigneRadar } from '@/composables/useRadarAtr'

const props = withDefaults(defineProps<{ ouvertDefaut?: boolean }>(), { ouvertDefaut: false })

const ouvert = ref(props.ouvertDefaut)
const { classement, confluences, chargement, demarrer } = useRadarAtr()

const INTRADAY = new Set(['M1', 'M5', 'M15', 'M30', 'H1'])

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

function titreLigne(i: LigneRadar): string {
  return INTRADAY.has(i.tf)
    ? `ATR ${i.asset} ${i.tf} : ${i.atr.toFixed(0)} % de l'habitude de cet instant (brut ${i.brut.toFixed(0)} %)`
    : `ATR ${i.asset} ${i.tf} : ${i.atr.toFixed(0)} % de la moyenne des 60 dernières bougies`
}

/// Le cycle 60 s ne vit que pendant l'ouverture — replié, le bloc ne coûte rien.
let arreter: (() => void) | null = null
watch(ouvert, async (ouvertMaintenant) => {
  if (ouvertMaintenant) {
    arreter = demarrer(60_000)
  } else if (arreter) {
    arreter()
    arreter = null
  }
})
onMounted(() => { if (ouvert.value) arreter = demarrer(60_000) })
onUnmounted(() => { if (arreter) arreter() })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
