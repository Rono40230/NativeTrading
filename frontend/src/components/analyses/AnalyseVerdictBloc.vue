<template>
  <!-- VERDICT — la réponse en 5 secondes (P3-1, maquette validée 23/09) :
       feu tricolore par règles sur les chiffres (jamais de texte LLM libre)
       + la ligne KPI + le conseil actionnable le plus fort tiré des
       classements. Toutes les mesures viennent du même payload que le reste
       de l'onglet (signaux Fermé+rempli, R encaissé — miroir du ML). -->
  <div class="glass-card p-3 flex flex-col gap-2" :class="bordure">
    <div class="flex items-center gap-3 flex-wrap">
      <span class="text-2xl leading-none">{{ verdict.icone }}</span>
      <div class="flex flex-col">
        <span class="text-sm font-bold" :class="verdict.couleur">{{ verdict.titre }}</span>
        <span class="text-[11px] text-white/80">{{ verdict.sousTitre }}</span>
      </div>
      <div class="ml-auto flex items-center gap-3 text-[11px] font-mono flex-wrap">
        <span class="text-white">{{ a.nb_trades }} clôturés</span>
        <span class="text-white">WR {{ Math.round(a.taux_reussite * 100) }} %</span>
        <span :class="a.r_total > 0 ? 'text-emerald-400' : a.r_total < 0 ? 'text-red-400' : 'text-white'">Σ {{ fmtR(a.r_total) }}</span>
        <span :class="a.capital_actuel >= a.capital_depart ? 'text-emerald-400' : 'text-red-400'">
          {{ fmtDollars(a.capital_actuel) }}
          <span class="text-white/60">({{ perf >= 0 ? '+' : '' }}{{ perf.toFixed(1) }} %)</span>
        </span>
        <span v-if="a.hier" class="text-white/60" title="Hier">· hier {{ fmtDollars(a.hier.dollars) }}</span>
      </div>
    </div>
    <p v-if="conseil" class="text-[11px] text-white/90 border-t border-white/10 pt-2">
      <span class="font-semibold text-white">Conseil :</span> {{ conseil }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { AnalyseStrategie } from '@/composables/useAnalyses'

const props = defineProps<{ a: AnalyseStrategie }>()

const perf = computed(() =>
  props.a.capital_depart > 0
    ? (props.a.capital_actuel / props.a.capital_depart - 1) * 100
    : 0,
)

/// Feu tricolore PAR RÈGLES sur les chiffres encaissés (règle des 30
/// d'abord — sous 30 clôtures, aucun verdict tranché).
const verdict = computed(() => {
  const { nb_trades, r_total, r_moyen, taux_reussite } = props.a
  if (nb_trades < 30) {
    return {
      icone: '⚪', couleur: 'text-white', titre: 'LECTURE PRÉMATURÉE',
      sousTitre: `${nb_trades}/30 clôtures — pas encore de conclusion possible (règle des 30)`,
      bordure: 'border-white/10',
    }
  }
  if (r_total > 0) {
    return {
      icone: '🟢', couleur: 'text-emerald-400', titre: 'CONTINUER',
      sousTitre: `Le bord est positif : Σ ${fmtR(r_total)} encaissés, ${Math.round(taux_reussite * 100)} % de gagnants`,
      bordure: 'border-emerald-500/30',
    }
  }
  if (r_moyen >= -0.05) {
    return {
      icone: '🟡', couleur: 'text-amber-400', titre: 'SURVEILLER',
      sousTitre: `Léger déficit (${fmtR(r_moyen)}/trade) — pas de signal d'alarme, mais aucune marge`,
      bordure: 'border-amber-500/30',
    }
  }
  return {
    icone: '🔴', couleur: 'text-red-400', titre: 'RÉVISER',
    sousTitre: `Déficit net : ${fmtR(r_moyen)}/trade, Σ ${fmtR(r_total)} — les classements ci-dessous disent où agir`,
    bordure: 'border-red-500/30',
  }
})
const bordure = computed(() => verdict.value.bordure)

/// Le conseil actionnable le plus fort : le meilleur et le pire élément
/// SIGNIFICATIF (≥ 30 clôtures) du premier classement de l'onglet (TF pour
/// SMC/Rockets/KDJ, événements pour straddle), sinon des assets.
const conseil = computed(() => {
  const dims: Array<{ nom: string; lignes: Array<{ label: string; n: number; rMoyen: number }> }> = []

  const tfsOuEvents = props.a.strategie === 'straddle' ? props.a.evenements : props.a.tfs
  if (tfsOuEvents?.length) {
    dims.push({ nom: props.a.strategie === 'straddle' ? 'événement' : 'timeframe', lignes: tfsOuEvents.map(c => ({ label: c.label, n: c.n, rMoyen: c.r / c.n })) })
  }
  if (props.a.assets?.length) {
    dims.push({ nom: 'asset', lignes: props.a.assets.map(c => ({ label: c.label, n: c.n, rMoyen: c.r / c.n })) })
  }
  for (const d of dims) {
    const valides = d.lignes.filter(l => l.n >= 30)
    if (valides.length < 2) continue
    const trie = [...valides].sort((x, y) => y.rMoyen - x.rMoyen)
    const meilleur = trie[0], pire = trie[trie.length - 1]
    if (meilleur.rMoyen <= 0) break // aucun élément positif → pas de conseil « continuer sur »
    const pireTxt = pire.rMoyen < 0 ? ` · ${pire.label} dilue (${fmtR(pire.rMoyen)}/trade sur ${pire.n})` : ''
    return `${meilleur.label} tient (${fmtR(meilleur.rMoyen)}/trade sur ${meilleur.n} clôtures)${pireTxt}`
  }
  return ''
})

function fmtR(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2)} R`
}
function fmtDollars(v: number): string {
  return `${Math.round(v).toLocaleString('fr-FR')} $`
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
