<template>
  <!-- Section « À risque » du poste d'observation (lecture seule — le moteur
       décide, cycle 30 s). Le conteneur (carte + barre latérale) vit dans la
       page ; ce composant rend le titre et la table. -->
  <p class="text-[11px] uppercase font-bold tracking-wider text-red-400 mb-2">🔴 Positions à risque — {{ positions.length }}</p>
  <div v-if="!positions.length" class="text-xs text-white py-3 text-center">Aucune — les cassures confirmées du scanner ouvrent ici.</div>
  <div v-else class="overflow-x-auto">
    <table class="w-full text-xs whitespace-nowrap">
      <thead>
        <tr class="text-white/70 uppercase text-[9px] border-b border-white/10">
          <th class="px-2 py-1.5 text-left">Ouvert le</th>
          <th class="px-2 py-1.5 text-left">Symbole</th>
          <th class="px-2 py-1.5 text-center">Type</th>
          <th class="px-2 py-1.5 text-right">Risque</th>
          <th class="px-2 py-1.5 text-right">Invalidation</th>
          <th class="px-2 py-1.5 text-right">Entrée</th>
          <th class="px-2 py-1.5 text-right">Cours ⚡</th>
          <th class="px-2 py-1.5 text-right">Tend.</th>
          <th class="px-2 py-1.5 text-right">Qté</th>
          <th class="px-2 py-1.5 text-right">Montant</th>
          <th class="px-2 py-1.5 text-right">P/L latent</th>
          <th class="px-2 py-1.5 text-right">R latent</th>
          <th class="px-2 py-1.5 text-right">R1</th>
          <th class="px-2 py-1.5 text-right">Vente R1</th>
          <th class="px-2 py-1.5 text-center">⋯</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="p in positions" :key="p.cle"
          class="border-b border-white/5"
          :class="classeLigne(live, p)"
          :title="titre(p)"
        >
          <td class="px-2 py-2 text-white">{{ dateCourte(p.ouvert_le) }}</td>
          <td class="px-2 py-2 font-bold text-white">{{ p.symbole }}</td>
          <td class="px-2 py-2 text-center">
            <span class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
              :class="p.univers === 'action' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-amber-500/10 text-amber-300 border-amber-500/30'"
            >{{ p.univers === 'action' ? 'Action' : 'Crypto' }}</span>
          </td>
          <td class="px-2 py-2 text-right font-mono" :class="classeRisque(p.risque_pct)">{{ p.risque_pct.toFixed(1).replace('.', ',') }} %</td>
          <td class="px-2 py-2 text-right font-mono text-red-400">{{ fmt(p.stop) }}</td>
          <td class="px-2 py-2 text-right font-mono text-white">{{ fmt(p.entree) }}</td>
          <td class="px-2 py-2 text-right font-mono text-white font-bold">{{ coursDe(live, p) ?? '—' }}</td>
          <td class="px-2 py-2 text-right font-mono" :class="(tendanceDe(live, p) ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ tendanceDe(live, p) === null ? '—' : `${(tendanceDe(live, p) ?? 0) >= 0 ? '▲' : '▼'} ${Math.abs(tendanceDe(live, p) ?? 0).toFixed(1)} %` }}
          </td>
          <td class="px-2 py-2 text-right font-mono text-white">{{ p.qty.toFixed(2) }}</td>
          <td class="px-2 py-2 text-right font-mono text-white">{{ p.montant.toFixed(0).replace('.', ',') }} $</td>
          <td class="px-2 py-2 text-right font-mono font-bold" :class="(plLatent(live, p) ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ plLatent(live, p) === null ? '—' : dollars(plLatent(live, p) ?? 0) }}
          </td>
          <td class="px-2 py-2 text-right font-mono font-bold" :class="(rLatent(live, p) ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ rLatent(live, p) === null ? '—' : `${(rLatent(live, p) ?? 0) >= 0 ? '+' : ''}${(rLatent(live, p) ?? 0).toFixed(2)}R` }}
          </td>
          <td class="px-2 py-2 text-right font-mono text-emerald-300">{{ fmt(p.r1) }}</td>
          <td class="px-2 py-2 text-right font-mono text-white/80">{{ (p.qty / 2).toFixed(2) }}</td>
          <td class="px-2 py-2 text-center">
            <button
              class="px-2 py-0.5 rounded bg-red-900/40 text-red-300 hover:bg-red-800/60 border border-red-500/30 text-[9px] font-semibold transition-colors disabled:opacity-40"
              :disabled="clotureEnCours === p.cle"
              title="Clôturer la position au prix de marché — alimente l'historique (verdict Manuel)"
              @click="cloturer(p)"
            >{{ clotureEnCours === p.cle ? '…' : '✕ Clôturer' }}</button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
  <!-- Modale de clôture (remplace le confirm() natif — design de l'app) -->
  <div v-if="aCloturer" class="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-4" @click.self="aCloturer = null">
    <div class="rounded-xl border border-red-500/30 p-5 w-[26rem] flex flex-col gap-4" style="background:#0d1117;">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-bold text-white uppercase tracking-wider">Clôturer la position</h3>
        <button class="text-white hover:text-white text-lg leading-none" @click="aCloturer = null">×</button>
      </div>

      <div class="rounded-lg bg-white/5 border border-white/10 px-3 py-2.5 flex flex-col gap-1.5 text-xs">
        <div class="flex items-center gap-2">
          <span class="font-bold text-white text-sm">{{ aCloturer.symbole }}</span>
          <span class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
                :class="aCloturer.univers === 'action' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-amber-500/10 text-amber-300 border-amber-500/30'"
          >{{ aCloturer.univers === 'action' ? 'Action' : 'Crypto' }}</span>
          <span class="ml-auto text-white/60">ouverte le {{ dateCourte(aCloturer.ouvert_le) }}</span>
        </div>
        <div class="grid grid-cols-3 gap-2 pt-1">
          <div><span class="text-white/50">Entrée</span><br><span class="font-mono text-white">{{ fmt(aCloturer.entree) }}</span></div>
          <div><span class="text-white/50">Cours ⚡</span><br><span class="font-mono text-white font-bold">{{ coursDe(live, aCloturer) ?? '—' }}</span></div>
          <div><span class="text-white/50">R latent</span><br>
            <span class="font-mono font-bold" :class="(rLatent(live, aCloturer) ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
              {{ rLatent(live, aCloturer) === null ? '—' : `${(rLatent(live, aCloturer) ?? 0) >= 0 ? '+' : ''}${(rLatent(live, aCloturer) ?? 0).toFixed(2)}R` }}
            </span>
          </div>
        </div>
      </div>

      <p class="text-[11px] text-white/80 leading-relaxed">
        Sortie de <span class="text-white font-semibold">toute la position restante</span> au prix de marché.
        Verdict <span class="text-blue-300 font-semibold">👤 Manuel</span> dans l'historique — le capital et les métriques s'ajustent.
        <span v-if="aCloturer.trailing != null" class="block mt-1 text-white/60">Position neutralisée : les 50 % vendus à R1 restent acquis, seul le solde sort.</span>
      </p>

      <div class="flex gap-2 justify-end">
        <button class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-white/10 hover:bg-white/20 text-white transition-colors" @click="aCloturer = null">Annuler</button>
        <button
          class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-red-600/80 hover:bg-red-500 text-white transition-colors disabled:opacity-40"
          :disabled="clotureEnCours === aCloturer.cle"
          @click="confirmerCloture"
        >{{ clotureEnCours === aCloturer.cle ? '⏳ Clôture…' : '✕ Clôturer au marché' }}</button>
      </div>
    </div>
  </div>

  <p class="text-[9px] text-white/50 mt-1.5">
    Le moteur neutralise dès que R1 est touché (cycle 30 s) puis le solde roule au trailing ; invalidation = sortie sèche −1R. « Clôturer » = sortie manuelle au prix de marché (verdict Manuel dans l'historique).
  </p>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { http } from '@/services/http.client'
import {
  coursDe, tendanceDe, plLatent, rLatent, classeLigne,
  type Position,
} from '@/composables/usePositionsRockets'

const props = defineProps<{
  positions: Position[]
  live: Record<string, { prix: number; open: number }>
}>()

const emit = defineEmits<{ cloturee: [] }>()
const clotureEnCours = ref('')
const aCloturer = ref<Position | null>(null)

function cloturer(p: Position) {
  aCloturer.value = p
}

async function confirmerCloture() {
  const p = aCloturer.value
  if (!p) return
  clotureEnCours.value = p.cle
  try {
    await http.post('/api/rockets/positions/cloturer', { cle: p.cle })
    aCloturer.value = null
    emit('cloturee')
  } catch { /* position déjà fermée ou cours indisponible — le refresh dit la vérité */ }
  clotureEnCours.value = ''
}

function titre(p: Position): string {
  const c = coursDe(props.live, p)
  if (c === null) return `${p.symbole} — cours indisponible`
  if (c >= p.r1) return `${p.symbole} — R1 ATTEINT : le moteur vend 50 % au prix R1 dès le prochain cycle (≤ 30 s) et déclenche le trailing`
  if (c <= p.stop) return `${p.symbole} — INVALIDATION TOUCHÉE : sortie sèche −1R dès le prochain cycle (≤ 30 s)`
  return `${p.symbole} — position pleine, ${p.risque_pct.toFixed(1)} % du capital engagé`
}

function classeRisque(pct: number): string {
  if (pct <= 0.5) return 'text-emerald-400'
  if (pct <= 1) return 'text-lime-400'
  if (pct <= 2) return 'text-orange-400'
  return 'text-red-400'
}
function fmt(v: number): string {
  return v >= 1 ? v.toFixed(2) : v.toFixed(4)
}
function dollars(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2).replace('.', ',')} $`
}
function dateCourte(ts: number): string {
  const d = new Date(ts < 1e12 ? ts * 1000 : ts)
  return `${String(d.getDate()).padStart(2, '0')}/${String(d.getMonth() + 1).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}
</script>

<style scoped>
@keyframes pulse-vert { 0%, 100% { background: rgba(52, 211, 153, 0.08); } 50% { background: rgba(52, 211, 153, 0.22); } }
@keyframes pulse-rouge { 0%, 100% { background: rgba(248, 113, 113, 0.08); } 50% { background: rgba(248, 113, 113, 0.22); } }
.animate-pulse-vert { animation: pulse-vert 1.5s ease-in-out infinite; }
.animate-pulse-rouge { animation: pulse-rouge 1.5s ease-in-out infinite; }
</style>
