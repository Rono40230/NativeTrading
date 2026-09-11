<template>
  <!-- 🔭 Scanner SMC (11/09) — le vivier des setups en formation, par asset,
       avec la confirmation multi-TF du propriétaire enfin visible : tendances
       H1/H4 (f_htf Pine, bougies clôturées) à côté de chaque vivier. Le
       liseré dit l'alignement : vert = setup dans le sens du HTF, ambre =
       HTF mitigé, rouge = contre-courant. Les setups dissipés (journal,
       grisés) servent la chasse aux faux négatifs et l'étude §3.2. -->
  <div class="h-full flex flex-col gap-3 overflow-hidden bg-white/5 rounded-xl px-4 py-3">
    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-xl font-bold text-white">🔭 Scanner SMC</h1>
      <span class="text-xs text-white">setups en formation · confirmation H1/H4 · journal des dissipés</span>
      <button
        class="ml-auto text-xs px-2.5 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors"
        :disabled="chargement"
        @click="charger"
      >{{ chargement ? '⏳ …' : '↻ Rafraîchir' }}</button>
    </div>

    <div v-if="erreur" class="text-xs text-red-400">{{ erreur }}</div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1 custom-scrollbar">
      <div class="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(350px,1fr))] content-start">
        <section
          v-for="a in assets" :key="a.nom"
          class="rounded-xl border bg-white/[0.03] p-3 flex flex-col gap-2"
          :class="classeCarte(a)"
        >
          <div class="flex items-center gap-2">
            <span class="font-mono font-bold text-white text-sm">{{ a.nom }}</span>
            <span class="badge" :class="classeTendance(a.mtf?.h1)" title="Tendance H1 (f_htf Pine, bougies clôturées)">H1 {{ fleche(a.mtf?.h1) }}</span>
            <span class="badge" :class="classeTendance(a.mtf?.h4)" title="Tendance H4">H4 {{ fleche(a.mtf?.h4) }}</span>
            <span class="ml-auto text-[10px] text-white">{{ a.vivants.length }} en formation</span>
          </div>

          <!-- Vivier -->
          <div v-if="a.vivants.length" class="flex flex-col gap-1.5">
            <div
              v-for="s in a.vivants" :key="s.cle"
              class="rounded-lg border-l-[3px] bg-black/20 px-2.5 py-1.5 border border-white/5"
              :class="classeAlignement(alignement(s))"
            >
              <div class="flex items-center gap-2 text-xs">
                <span class="font-mono font-bold text-white">{{ s.tf }}</span>
                <span :class="s.direction === 'Long' ? 'text-emerald-400' : 'text-red-400'">{{ s.direction === 'Long' ? '▲' : '▼' }} {{ s.direction }}</span>
                <span class="font-mono font-bold text-white">{{ s.force }}/10</span>
                <span
                  class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
                  :class="s.statut === 'EnFormation' ? 'bg-amber-500/10 text-amber-300 border-amber-500/30'
                        : s.statut === 'Confirme' ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
                        : 'bg-white/5 text-white border-white/10'"
                >{{ s.statut === 'EnFormation' ? '⏳ en formation' : s.statut === 'Confirme' ? '✓ confirmé' : 'dissipé' }}</span>
                <span class="ml-auto text-white text-[10px]" :title="`Vérité à la clôture : ${heure(s.cloture_barre)}`">⚡ {{ heure(s.cloture_barre) }}</span>
              </div>
              <div class="text-[10px] text-white font-mono mt-0.5">
                E {{ fmt(s.entree) }} · SL {{ fmt(s.sl) }} · TP1 {{ fmt(s.tps[0]) }}
              </div>
            </div>
          </div>
          <p v-else class="text-[11px] text-white py-1">Aucun setup en formation — zone calme.</p>

          <!-- Journal récent (clôturés) -->
          <div v-if="a.clotures.length" class="mt-auto pt-1.5 border-t border-white/5 flex flex-col gap-0.5">
            <p class="text-[9px] uppercase tracking-wide text-white">Journal récent</p>
            <div v-for="j in a.clotures" :key="j.cle" class="flex items-center gap-2 text-[10px] text-white">
              <span class="font-mono">{{ j.tf }}</span>
              <span>{{ j.direction === 'Long' ? '▲' : '▼' }}</span>
              <span class="font-mono">{{ j.force_max }}/10</span>
              <span :class="j.issue === 'signal' ? 'text-emerald-400' : 'text-white/60'">
                {{ j.issue === 'signal' ? '✓ signal' : '✗ dissipé' }}
              </span>
              <span class="ml-auto">{{ heure(j.annonce_le) }}</span>
            </div>
          </div>
        </section>
      </div>
      <p v-if="!assets.length && !chargement" class="text-center text-white text-sm py-10">
        Aucun asset armé — le périmètre vit dans la modale Timeframe/Asset de la carte SMC.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'

interface SetupVivant {
  strategie: string; asset: string; tf: string; direction: string
  force: number; entree: number; sl: number; tps: number[]
  cle: string; debut_barre: number; cloture_barre: number; ts_annonce: number
  statut: string
}
interface LigneJournal {
  cle: string; asset: string; tf: string; direction: string
  force_max: number; annonce_le: number; fin: number | null
  issue: string | null; signal_id: string | null
}
type Trends = Record<string, { h1: number; h4: number }>

const vivants = ref<SetupVivant[]>([])
const journal = ref<LigneJournal[]>([])
const mtf = ref<Trends>({})
const chargement = ref(false)
const erreur = ref('')

interface CarteAsset {
  nom: string
  mtf: { h1: number; h4: number } | undefined
  vivants: SetupVivant[]
  clotures: LigneJournal[]
}

const assets = computed<CarteAsset[]>(() => {
  const noms = new Set<string>([...Object.keys(mtf.value), ...vivants.value.map(s => s.asset)])
  return [...noms].sort().map(nom => ({
    nom,
    mtf: mtf.value[nom],
    vivants: vivants.value.filter(s => s.asset === nom && s.statut !== 'Dissipe'),
    clotures: journal.value
      .filter(j => j.asset === nom && j.issue !== null)
      .slice(0, 6),
  }))
})

async function charger() {
  chargement.value = true
  erreur.value = ''
  try {
    const [formation, trends, hist] = await Promise.all([
      http.get<SetupVivant[]>('/api/setups-formation', { params: { strategie: 'SMC' } }),
      http.get<Trends>('/api/smc/mtf'),
      http.get<LigneJournal[]>('/api/smc/setups-journal', { params: { limite: 200 } }),
    ])
    vivants.value = formation.data.filter(s => s.strategie === 'SMC')
    mtf.value = trends.data
    journal.value = hist.data
  } catch (e: unknown) {
    erreur.value = 'Lecture impossible — ' + ((e as { message?: string }).message ?? 'erreur inconnue')
  } finally {
    chargement.value = false
  }
}
onMounted(charger)

type Align = 'aligne' | 'mixte' | 'oppose' | 'neutre'
function alignement(s: SetupVivant): Align {
  const t = mtf.value[s.asset]
  if (!t || t.h1 === 0 || t.h4 === 0) return 'neutre'
  const up = t.h1 > 0 && t.h4 > 0
  const down = t.h1 < 0 && t.h4 < 0
  if (s.direction === 'Long') return up ? 'aligne' : down ? 'oppose' : 'mixte'
  return down ? 'aligne' : up ? 'oppose' : 'mixte'
}

function classeAlignement(a: Align): string {
  return {
    aligne: 'border-l-emerald-500',
    mixte: 'border-l-amber-500',
    oppose: 'border-l-red-500',
    neutre: 'border-l-white/20',
  }[a]
}
function classeCarte(a: CarteAsset): string {
  return a.vivants.some(s => alignement(s) === 'aligne')
    ? 'border-blue-500/30'
    : 'border-white/10'
}
function classeTendance(t: number | undefined): string {
  if (t === undefined) return 'bg-white/5 text-white border-white/10'
  if (t > 0) return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
  if (t < 0) return 'bg-red-500/10 text-red-400 border-red-500/30'
  return 'bg-white/5 text-white border-white/10'
}
function fleche(t: number | undefined): string {
  return t === undefined || t === 0 ? '—' : t > 0 ? '▲' : '▼'
}
function fmt(v: number): string {
  return v >= 1000 ? v.toFixed(1) : v >= 1 ? v.toFixed(2) : v.toFixed(4)
}
function heure(ts: number): string {
  const d = new Date(ts < 1e12 ? ts * 1000 : ts)
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}
</script>

<style scoped>
.badge { @apply text-[9px] font-bold px-1.5 py-0.5 rounded-full border; }
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255,255,255,.15); border-radius: 10px; }
</style>
