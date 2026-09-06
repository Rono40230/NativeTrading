<template>
  <!-- Aperçu compact des candidats rockets SUIVIS, pour la colonne « Setups
       en attente » de la page stratégie — l'équivalent des setups en
       formation SMC / de l'agenda straddle. La table complète (filtres,
       critères, news) vit dans la page 🔭 Scanner dédiée. -->
  <div class="flex flex-col gap-1.5">
    <div v-if="chargement && !candidats.length" class="text-[11px] text-white">Chargement…</div>
    <template v-else>
      <div v-if="!suivis.length" class="text-[11px] text-white">Aucun candidat suivi — le scanner quotidien n'a rien retenu (seuil 5/10)</div>
      <div
        v-for="c in suivis" :key="c.symbole"
        class="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-xs rounded-lg border px-2.5 py-1.5 cursor-help"
        :class="c.cassure ? 'bg-emerald-500/10 border-emerald-500/30' : c.points >= 7 ? 'bg-blue-500/10 border-blue-500/30' : 'bg-white/5 border-white/10'"
        :title="`${c.symbole} · ${c.points}/10 · pivot ${c.pivot} · invalidation ${c.stop}${c.cassure ? ' · PIVOT CASSÉ' : ''}${c.earnings_le ? `\n📊 résultats le ${c.earnings_le} (risque de gap)` : ''}\nDétail complet : page 🔭 Scanner`"
      >
        <span class="text-[13px] leading-none">{{ c.cassure ? '🚀' : '⏳' }}</span>
        <span class="font-semibold text-white">{{ c.symbole }}</span>
        <span class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
          :class="c.univers === 'action' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-amber-500/10 text-amber-300 border-amber-500/30'"
        >{{ c.univers === 'action' ? 'Action' : 'Crypto' }}</span>
        <span class="font-mono" :class="c.points >= 7 ? 'text-blue-300' : 'text-white'">{{ c.points }}/10</span>
        <span v-if="c.verdict !== 'Elimine'" class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
          :class="c.verdict === 'Alpha' ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30' : 'bg-blue-500/10 text-blue-300 border-blue-500/30'"
        >{{ c.verdict === 'Alpha' ? 'ALPHA' : 'ROCKET' }}</span>
        <span v-if="c.earnings_le" class="text-[9px] px-1 py-0.5 rounded bg-orange-900/60 text-orange-300">📊 {{ c.earnings_le.slice(5) }}</span>
        <span class="ml-auto font-mono text-[10px] text-white/80 whitespace-nowrap">{{ c.cassure ? 'pivot cassé' : `→ ${c.pivot}` }}</span>
      </div>
      <div v-if="nbElimines" class="text-[10px] text-white/60"
        :title="`${nbElimines} candidats sortis du suivi, conservés en base — consultables dans la page Scanner (faux négatifs)`"
      >{{ suivis.length }} suivi{{ suivis.length > 1 ? 's' : '' }} · {{ nbElimines }} éliminé{{ nbElimines > 1 ? 's' : '' }}</div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { http } from '@/services/http.client'

interface Candidat {
  symbole: string; points: number; verdict: string
  univers?: string
  elimine_le?: number | null
  earnings_le?: string | null
  pivot: number; stop: number; cassure: boolean
}

const candidats = ref<Candidat[]>([])
const chargement = ref(true)

/// Candidats SUIVIS uniquement (elimine_le null), ordre serveur
/// (actifs d'abord, points décroissants).
const suivis = computed(() => candidats.value.filter(c => !c.elimine_le))
const nbElimines = computed(() => candidats.value.length - suivis.value.length)

onMounted(async () => {
  try {
    const res = await http.get<Candidat[]>('/api/rockets/candidats')
    candidats.value = res.data as Candidat[]
  } catch { candidats.value = [] }
  chargement.value = false
})
</script>
