<template>
  <!-- Historique des trades rockets clôturés — LA logique rocket (pas de
       TP1/2/3) : le cycle Entrée → R1 (50 %) → Trailing → Sortie, jugé par
       verdict TS/SL, R réalisé, P/L $, et le trio R1 encaissé / Trailing
       final / Sommet qui mesure la qualité de la gestion. -->
  <div class="flex flex-col gap-2 flex-1 min-h-0 overflow-y-auto pr-1">
    <!-- Synthèse (décision 06/09) : effectif · WR · ΣR · Σ$ -->
    <div class="text-sm text-white flex flex-wrap items-center gap-x-4">
      <span>{{ triés.length }} trade{{ triés.length > 1 ? 's' : '' }}</span>
      <span class="text-white">WR <span class="font-mono font-semibold" :class="wr >= 50 ? 'text-emerald-400' : 'text-red-400'">{{ wr.toFixed(0) }} %</span></span>
      <span v-if="sommeR !== null" class="font-mono font-semibold" :class="classe(sommeR)">ΣR {{ rFmt(sommeR) }}</span>
      <span class="font-mono font-semibold" :class="classe(sommeDollars)">Σ{{ dollars(sommeDollars) }}</span>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full text-xs whitespace-nowrap">
        <thead>
          <tr class="text-white/70 uppercase text-[9px] border-b border-white/10">
            <th class="px-2 py-2 text-left">#</th>
            <th class="px-1 py-2 text-center" title="Journal de bord du trade (notes du propriétaire)">📝</th>
            <th class="px-2 py-2 text-left cursor-pointer hover:text-white" @click="trier('symbole')">Symbole</th>
            <th class="px-2 py-2 text-center cursor-pointer hover:text-white" @click="trier('univers')">Type</th>
            <th class="px-2 py-2 text-right cursor-pointer hover:text-white" @click="trier('classement')">Classement</th>
            <th class="px-2 py-2 text-left cursor-pointer hover:text-white" @click="trier('ouvert_le')">Ouvert le</th>
            <th class="px-2 py-2 text-right cursor-pointer hover:text-white" @click="trier('duree')">Durée</th>
            <th class="px-2 py-2 text-right cursor-pointer hover:text-white" @click="trier('entree')">Entrée</th>
            <th class="px-2 py-2 text-right">Invalidation</th>
            <th class="px-2 py-2 text-right">Qté</th>
            <th class="px-2 py-2 text-right">Montant</th>
            <th class="px-2 py-2 text-right" title="Prix de la vente de 50 % (neutralisation)">R1 encaissé</th>
            <th class="px-2 py-2 text-right" title="Dernier plancher suivi — celui qui a sorti">Trailing final</th>
            <th class="px-2 py-2 text-right" title="Plus haut atteint pendant la vie du trade — juge la largeur du trailing">Sommet</th>
            <th class="px-2 py-2 text-right">Sortie</th>
            <th class="px-2 py-2 text-center cursor-pointer hover:text-white" @click="trier('verdict')">Verdict</th>
            <th class="px-2 py-2 text-right cursor-pointer hover:text-white" @click="trier('r_realise')">R réalisé</th>
            <th class="px-2 py-2 text-right cursor-pointer hover:text-white" @click="trier('pl_dollars')">P/L $</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="!triés.length">
            <td colspan="18" class="text-center text-white py-8">Aucun trade clôturé — les sorties TS/SL vivront ici.</td>
          </tr>
          <tr
            v-for="(t, i) in triés" :key="t.cle"
            class="border-b border-white/5 hover:bg-white/5"
            :title="titre(t)"
          >
            <td class="px-2 py-2 text-white">{{ i + 1 }}</td>
            <td class="px-1 py-2 text-center">
              <button
                class="text-[11px] font-mono rounded px-1 transition-colors"
                :class="journalComptes?.[t.signal_id ?? ''] ? 'bg-teal-500/20 text-teal-300 hover:bg-teal-500/30' : 'text-white/30 hover:text-white'"
                :title="journalComptes?.[t.signal_id ?? ''] ? `${journalComptes[t.signal_id ?? '']} note(s) — ouvrir le journal` : 'Ouvrir le journal de bord du trade'"
                @click="journalSignal = t"
              >{{ journalComptes?.[t.signal_id ?? ''] ?? '+' }}</button>
            </td>
            <td class="px-2 py-2 font-bold text-white">{{ t.symbole }}</td>
            <td class="px-2 py-2 text-center">
              <span class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
                :class="t.univers === 'action' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-amber-500/10 text-amber-300 border-amber-500/30'"
              >{{ t.univers === 'action' ? 'Action' : 'Crypto' }}</span>
            </td>
            <td class="px-2 py-2 text-right font-mono" :class="(t.classement ?? 0) >= 7 ? 'text-blue-300' : 'text-white'">{{ t.classement ?? '—' }}/10</td>
            <td class="px-2 py-2 text-white">{{ dateHeure(t.ouvert_le) }}</td>
            <td class="px-2 py-2 text-right font-mono text-white" :title="t.ferme_le ? `Fermé le ${dateHeure(t.ferme_le)}` : ''">{{ duree(t) }}</td>
            <td class="px-2 py-2 text-right font-mono text-white">{{ fmt(t.entree) }}</td>
            <td class="px-2 py-2 text-right font-mono text-red-400">{{ fmt(t.stop) }}</td>
            <td class="px-2 py-2 text-right font-mono text-white">{{ t.qty.toFixed(2) }}</td>
            <td class="px-2 py-2 text-right font-mono text-white/80">{{ t.montant.toFixed(0).replace('.', ',') }} $</td>
            <td class="px-2 py-2 text-right font-mono" :class="t.r1_encaisse ? 'text-emerald-300' : 'text-white/40'">{{ t.r1_encaisse ? fmt(t.r1_encaisse) : '—' }}</td>
            <td class="px-2 py-2 text-right font-mono text-amber-300">{{ t.trailing_final ? fmt(t.trailing_final) : '—' }}</td>
            <td class="px-2 py-2 text-right font-mono text-white/80">{{ t.sommet ? fmt(t.sommet) : '—' }}</td>
            <td class="px-2 py-2 text-right font-mono text-white">{{ t.prix_sortie ? fmt(t.prix_sortie) : '—' }}</td>
            <td class="px-2 py-2 text-center">
              <span class="badge" :class="t.verdict === 'TS' ? 'badge-green' : 'badge-red'">{{ t.verdict === 'TS' ? '🏁 TS' : '❌ SL' }}</span>
            </td>
            <td class="px-2 py-2 text-right font-mono font-bold" :class="classe(t.r_realise)">{{ t.r_realise === null ? '—' : rFmt(t.r_realise) }}</td>
            <td class="px-2 py-2 text-right font-mono font-bold" :class="classe(t.pl_dollars)">{{ dollars(t.pl_dollars) }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <JournalBordModal
      :ouvert="journalSignal !== null"
      :signal-id="journalSignal?.signal_id ?? ''"
      :titre="journalSignal ? `${journalSignal.symbole} — ${journalSignal.verdict ?? ''} ${journalSignal.r_realise !== null ? rFmt(journalSignal.r_realise) : ''}` : ''"
      @fermer="journalSignal = null"
      @note="charger()"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { http } from '@/services/http.client'
import JournalBordModal from '@/components/common/JournalBordModal.vue'

interface TradeFerme {
  cle: string; signal_id: string | null
  symbole: string; univers: 'crypto' | 'action'; classement: number | null
  ouvert_le: number; ferme_le: number | null
  entree: number; stop: number; r1: number; qty: number; montant: number
  r1_encaisse: number | null; trailing_final: number | null; sommet: number | null
  prix_sortie: number | null; verdict: string | null
  r_realise: number | null; pl_dollars: number
}

const trades = ref<TradeFerme[]>([])
const journalComptes = ref<Record<string, number>>({})
const journalSignal = ref<TradeFerme | null>(null)
const triColonne = ref('')
const triDir = ref<'asc' | 'desc'>('desc')

/// Synthèse : WR = part de trades à R > 0 · ΣR · Σ$ (réalisé, non composé —
/// le capital composé vit sur la carte).
const wr = computed(() => {
  const n = trades.value.length
  if (!n) return 0
  return (trades.value.filter(t => (t.r_realise ?? 0) > 0).length / n) * 100
})
const sommeR = computed(() => {
  const rs = trades.value.map(t => t.r_realise).filter((r): r is number => r !== null)
  return rs.length ? rs.reduce((a, b) => a + b, 0) : null
})
const sommeDollars = computed(() => trades.value.reduce((a, t) => a + t.pl_dollars, 0))

function valeurTri(t: TradeFerme, col: string): number | string | null {
  if (col === 'duree') return t.ferme_le ? t.ferme_le - t.ouvert_le / 1000 : null
  if (col === 'classement') return t.classement
  const v = (t as unknown as Record<string, unknown>)[col]
  return typeof v === 'number' || typeof v === 'string' ? v : null
}
const triés = computed(() => {
  const liste = [...trades.value]
  if (!triColonne.value) return liste
  const sens = triDir.value === 'asc' ? 1 : -1
  return liste.sort((a, b) => {
    const va = valeurTri(a, triColonne.value)
    const vb = valeurTri(b, triColonne.value)
    if (va === null && vb === null) return 0
    if (va === null) return 1
    if (vb === null) return -1
    if (typeof va === 'number' && typeof vb === 'number') return (va - vb) * sens
    return String(va).localeCompare(String(vb)) * sens
  })
})
function trier(col: string) {
  if (triColonne.value === col) {
    triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    triColonne.value = col
    triDir.value = 'desc'
  }
}

async function charger() {
  try {
    const h = await http.get('/api/rockets/historique')
    trades.value = h.data.trades ?? []
  } catch { trades.value = [] }
  try {
    const j = await http.get<Record<string, number>>('/api/journal/comptes')
    journalComptes.value = j.data ?? {}
  } catch { journalComptes.value = {} }
}

function duree(t: TradeFerme): string {
  if (!t.ferme_le) return '—'
  const s = Math.max(0, Math.floor((t.ferme_le - t.ouvert_le / 1000)))
  if (s < 60) return `${s} s`
  if (s < 3600) return `${Math.floor(s / 60)} mn`
  if (s < 86400) return `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} mn`
  return `${Math.floor(s / 86400)} j ${Math.floor((s % 86400) / 3600)} h`
}
function titre(t: TradeFerme): string {
  const lignes = [`${t.symbole} — ${t.verdict ?? '—'} · ${t.r_realise !== null ? rFmt(t.r_realise) : ''} · ${dollars(t.pl_dollars)}`]
  lignes.push(`Entrée ${fmt(t.entree)} · Invalidation ${fmt(t.stop)} · R1 ${fmt(t.r1)} · Qté ${t.qty.toFixed(2)}`)
  if (t.r1_encaisse) lignes.push(`Neutralisé à ${fmt(t.r1_encaisse)} (50 % vendus)`)
  if (t.sommet) lignes.push(`Sommet ${fmt(t.sommet)}${t.trailing_final ? ` · trailing final ${fmt(t.trailing_final)} (marge sommet→sortie : ${(((t.sommet - (t.prix_sortie ?? t.trailing_final)) / (t.entree - t.stop))).toFixed(2)}R)` : ''}`)
  if (t.ferme_le) lignes.push(`Fermé le ${dateHeure(t.ferme_le)}`)
  return lignes.join('\n')
}
function classe(v: number | null): string {
  if (v === null || v === 0) return 'text-white'
  return v > 0 ? 'text-emerald-400' : 'text-red-400'
}
function rFmt(r: number): string {
  return `${r >= 0 ? '+' : ''}${r.toFixed(2)}R`
}
function dollars(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2).replace('.', ',')} $`
}
function fmt(v: number): string {
  return v >= 1 ? v.toFixed(2) : v.toFixed(4)
}
function dateHeure(ts: number): string {
  const d = new Date(ts < 1e12 ? ts * 1000 : ts)
  return `${String(d.getDate()).padStart(2, '0')}/${String(d.getMonth() + 1).padStart(2, '0')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

let minuteur: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void charger()
  minuteur = setInterval(charger, 30_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>
