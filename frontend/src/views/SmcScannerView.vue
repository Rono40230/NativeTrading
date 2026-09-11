<template>
  <!-- 🔭 Scanner SMC v2 (11/09) — la salle de marché : tous les assets en
       même temps, un mini-graphique par carte (OB + zones achat/vente +
       niveaux des setups vivants, prix temps réel). Sélecteur AUTO par
       défaut : chaque carte montre le TF de son meilleur setup annoncé
       (règle A : force d'abord, alignement H1/H4 départage — repli M15 si
       le vivier est vide). Carte cliquable → grand graphique. -->
  <div class="h-full flex flex-col gap-3 overflow-hidden bg-white/5 rounded-xl px-4 py-3">
    <div class="flex items-center gap-3 shrink-0 flex-wrap">
      <h1 class="text-xl font-bold text-white">🔭 Scanner SMC</h1>
      <span class="text-xs text-white">le vivier complet, chaque asset sur son meilleur setup</span>
      <div class="ml-auto flex items-center gap-2">
        <label class="flex items-center gap-1.5 text-xs text-white cursor-pointer select-none"
               title="Ne montrer que les setups dans le sens H1+H4 (lecture seule — aucun filtrage moteur)">
          <input type="checkbox" v-model="alignesSeuls" class="accent-emerald-400 w-3.5 h-3.5" />
          Alignés HTF
        </label>
        <select
          class="bg-black/30 border border-white/15 rounded-lg px-2 py-1.5 text-xs text-white"
          v-model="mode"
          title="Auto : chaque carte choisit le TF de son meilleur setup (force, puis alignement MTF)"
        >
          <option value="auto">Auto · meilleur setup</option>
          <option value="M5">M5</option>
          <option value="M15">M15</option>
          <option value="M30">M30</option>
        </select>
        <button
          class="text-xs px-2.5 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors"
          :disabled="chargement"
          @click="charger"
        >{{ chargement ? '⏳ …' : '↻' }}</button>
      </div>
    </div>

    <div v-if="erreur" class="text-xs text-red-400 shrink-0">{{ erreur }}</div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1 custom-scrollbar">
      <div class="grid grid-cols-2 gap-3 content-start">
        <section
          v-for="a in cartes" :key="a.nom"
          class="rounded-xl border bg-white/[0.03] p-3 flex flex-col gap-2 cursor-pointer transition-colors hover:border-blue-400/40"
          :class="a.aligne ? 'border-blue-500/30' : 'border-white/10'"
          @click="ouvrirGraphique(a)"
        >
          <div class="flex items-center gap-2">
            <span class="font-mono font-bold text-white text-sm">{{ a.nom }}</span>
            <span class="badge" :class="classeTendance(a.mtf?.h1)" title="Tendance H1 (f_htf Pine)">H1 {{ fleche(a.mtf?.h1) }}</span>
            <span class="badge" :class="classeTendance(a.mtf?.h4)" title="Tendance H4">H4 {{ fleche(a.mtf?.h4) }}</span>
            <span
              class="ml-auto text-[9px] font-bold px-1.5 py-0.5 rounded-full border"
              :class="mode === 'auto' ? 'bg-blue-500/10 text-blue-300 border-blue-500/30' : 'bg-white/5 text-white border-white/10'"
              :title="mode === 'auto' ? `Auto : meilleur setup annoncé (force puis alignement) — repli M15` : 'TF verrouillé manuellement'"
            >{{ mode === 'auto' ? `AUTO · ${a.tfAuto}` : mode }}</span>
          </div>

          <MiniChartSmc
            :asset="a.nom" :tf="a.tf"
            :setups="a.vivants.map(s => ({ direction: s.direction, entree: s.entree, sl: s.sl, tp1: s.tps[0] ?? s.entree }))"
            @ouvrir="ouvrirGraphique(a)"
          />

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
          <p v-else class="text-[11px] text-white py-1">Aucun setup en formation — zone calme{{ mode === 'auto' ? ' (vue M15)' : '' }}.</p>

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
      <p v-if="!cartes.length && !chargement" class="text-center text-white text-sm py-10">
        Aucun asset armé — le périmètre vit dans la modale Timeframe/Asset de la carte SMC.
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import { useSettingsStore } from '@/stores/settings.store'
import MiniChartSmc from '@/components/chart/MiniChartSmc.vue'

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

const router = useRouter()
const settingsStore = useSettingsStore()

const vivants = ref<SetupVivant[]>([])
const journal = ref<LigneJournal[]>([])
const mtf = ref<Trends>({})
const mode = ref<'auto' | 'M5' | 'M15' | 'M30'>('auto')
const alignesSeuls = ref(false)
const chargement = ref(false)
const erreur = ref('')
let minuteur: ReturnType<typeof setInterval> | null = null

type Align = 'aligne' | 'mixte' | 'oppose' | 'neutre'
const RANG_ALIGN: Record<Align, number> = { aligne: 3, mixte: 2, oppose: 1, neutre: 0 }

interface Carte {
  nom: string
  mtf: { h1: number; h4: number } | undefined
  vivants: SetupVivant[]
  clotures: LigneJournal[]
  tfAuto: string
  tf: string
  aligne: boolean
}

function alignementSetup(s: SetupVivant, t?: { h1: number; h4: number }): Align {
  if (!t || t.h1 === 0 || t.h4 === 0) return 'neutre'
  const up = t.h1 > 0 && t.h4 > 0
  const down = t.h1 < 0 && t.h4 < 0
  if (s.direction === 'Long') return up ? 'aligne' : down ? 'oppose' : 'mixte'
  return down ? 'aligne' : up ? 'oppose' : 'mixte'
}

/// Règle A (décision 11/09) : le meilleur setup annoncé = force décroissante,
/// à égalité le plus aligné H1/H4. Repli M15 sans vivier.
function meilleurTf(nom: string, setups: SetupVivant[]): string {
  const t = mtf.value[nom]
  const classe = [...setups].sort((a, b) =>
    b.force - a.force || RANG_ALIGN[alignementSetup(b, t)] - RANG_ALIGN[alignementSetup(a, t)])
  return classe[0]?.tf ?? 'M15'
}

const cartes = computed<Carte[]>(() => {
  const noms = new Set<string>([...Object.keys(mtf.value), ...vivants.value.map(s => s.asset)])
  return [...noms].sort().map(nom => {
    const vivs = vivants.value
      .filter(s => s.asset === nom && s.statut !== 'Dissipe')
      .filter(s => !alignesSeuls.value || alignementSetup(s, mtf.value[nom]) === 'aligne')
    const tfAuto = meilleurTf(nom, vivants.value.filter(s => s.asset === nom && s.statut !== 'Dissipe'))
    const tf = mode.value === 'auto' ? tfAuto : mode.value
    return {
      nom,
      mtf: mtf.value[nom],
      vivants: vivs,
      clotures: journal.value.filter(j => j.asset === nom && j.issue !== null).slice(0, 6),
      tfAuto,
      tf,
      aligne: vivs.some(s => alignementSetup(s, mtf.value[nom]) === 'aligne'),
    }
  })
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

function ouvrirGraphique(a: Carte) {
  settingsStore.definirAsset(a.nom)
  settingsStore.definirTimeframe(a.tf)
  router.push('/smc/graphiques')
}

function alignement(s: SetupVivant): Align {
  return alignementSetup(s, mtf.value[s.asset])
}
function classeAlignement(a: Align): string {
  return {
    aligne: 'border-l-emerald-500',
    mixte: 'border-l-amber-500',
    oppose: 'border-l-red-500',
    neutre: 'border-l-white/20',
  }[a]
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

onMounted(() => {
  void charger()
  minuteur = setInterval(() => void charger(), 45_000)
})
onUnmounted(() => { if (minuteur) clearInterval(minuteur) })
</script>

<style scoped>
.badge { @apply text-[9px] font-bold px-1.5 py-0.5 rounded-full border; }
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255,255,255,.15); border-radius: 10px; }
</style>
