<template>
  <!-- Section « Neutralisées » du poste d'observation : R1 encaissé (50 %
       vendus), le solde roule au trailing. Lecture seule. -->
  <p class="text-[11px] uppercase font-bold tracking-wider text-amber-400 mb-2">🟡 Neutralisées — R1 encaissé, solde au trailing</p>
  <div v-if="!positions.length" class="text-xs text-white py-3 text-center">Aucune position neutralisée.</div>
  <div v-else class="overflow-x-auto">
    <table class="w-full text-xs whitespace-nowrap">
      <thead>
        <tr class="text-white/70 uppercase text-[9px] border-b border-white/10">
          <th class="px-2 py-1.5 text-left">Symbole</th>
          <th class="px-2 py-1.5 text-center">Type</th>
          <th class="px-2 py-1.5 text-right">Entrée</th>
          <th class="px-2 py-1.5 text-right">R1 encaissé</th>
          <th class="px-2 py-1.5 text-right">Trailing actif</th>
          <th class="px-2 py-1.5 text-right">Cours ⚡</th>
          <th class="px-2 py-1.5 text-right">Qté restante</th>
          <th class="px-2 py-1.5 text-right">Montant restant</th>
          <th class="px-2 py-1.5 text-right">P/L (encaissé + latent)</th>
          <th class="px-2 py-1.5 text-right">Évolution R</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="p in positions" :key="p.cle"
          class="border-b border-white/5"
          :class="classeLigne(live, p, true)"
          :title="`${p.symbole} — 50 % encaissés à R1, le solde roule au trailing (sortie si le cours le touche)`"
        >
          <td class="px-2 py-2 font-bold text-white">{{ p.symbole }}</td>
          <td class="px-2 py-2 text-center">
            <span class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
              :class="p.univers === 'action' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-amber-500/10 text-amber-300 border-amber-500/30'"
            >{{ p.univers === 'action' ? 'Action' : 'Crypto' }}</span>
          </td>
          <td class="px-2 py-2 text-right font-mono text-white">{{ fmt(p.entree) }}</td>
          <td class="px-2 py-2 text-right font-mono text-emerald-300">{{ p.prix_r1 ? fmt(p.prix_r1) : '—' }}</td>
          <td class="px-2 py-2 text-right font-mono text-amber-300">{{ p.trailing ? fmt(p.trailing) : '—' }}</td>
          <td class="px-2 py-2 text-right font-mono text-white font-bold">{{ coursDe(live, p) ?? '—' }}</td>
          <td class="px-2 py-2 text-right font-mono text-white/80">{{ (p.qty_restante ?? p.qty / 2).toFixed(2) }}</td>
          <td class="px-2 py-2 text-right font-mono text-white">{{ (p.montant_restant ?? 0).toFixed(0).replace('.', ',') }} $</td>
          <td class="px-2 py-2 text-right font-mono font-bold" :class="(plNeutralisee(live, p) ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ plNeutralisee(live, p) === null ? '—' : dollars(plNeutralisee(live, p) ?? 0) }}
          </td>
          <td class="px-2 py-2 text-right font-mono font-bold" :class="(evolutionR(live, p) ?? 0) >= 1 ? 'text-emerald-400' : 'text-red-400'">
            {{ evolutionR(live, p) === null ? '—' : `${(evolutionR(live, p) ?? 0) >= 0 ? '+' : ''}${(evolutionR(live, p) ?? 0).toFixed(2)}R` }}
          </td>
        </tr>
      </tbody>
    </table>
  </div>
  <p class="text-[9px] text-white/50 mt-1.5">
    Évolution R = (cours − trailing) / risque initial + 1 — la marge au-dessus du plancher de sortie.
  </p>
</template>

<script setup lang="ts">
import {
  coursDe, plNeutralisee, evolutionR, classeLigne,
  type Position,
} from '@/composables/usePositionsRockets'

defineProps<{
  positions: Position[]
  live: Record<string, { prix: number; open: number }>
}>()

function fmt(v: number): string {
  return v >= 1 ? v.toFixed(2) : v.toFixed(4)
}
function dollars(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2).replace('.', ',')} $`
}
</script>

<style scoped>
@keyframes pulse-rouge { 0%, 100% { background: rgba(248, 113, 113, 0.08); } 50% { background: rgba(248, 113, 113, 0.22); } }
.animate-pulse-rouge { animation: pulse-rouge 1.5s ease-in-out infinite; }
</style>
