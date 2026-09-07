<template>
  <div class="flex flex-col gap-1.5">
    <div v-if="annonces.length" class="flex flex-col gap-1">
      <div v-for="a in annonces.slice(0, 6)" :key="a.ts"
           class="flex items-center gap-2 text-xs">
        <span class="text-amber-400">📅</span>
        <span class="text-white font-medium truncate">{{ a.titre || 'Annonce US' }}</span>
        <span class="text-white">{{ heureLocale(a.ts) }}</span>
        <span class="ml-auto text-amber-300/90 font-mono text-[11px]">{{ compteARebours(a.ts) }}</span>
      </div>
    </div>
    <div v-else class="text-[11px] text-white">Aucune annonce US forte à 7 jours</div>
    <div v-if="passes.length" class="text-[11px] text-emerald-400/80">
      {{ passes.length }} passe(s) en cours sur {{ [...new Set(passes.map(p => p.asset))].join(', ') }}
    </div>

    <!-- §16 (07/09) — Créneaux IA : propositions de l'analyste, armement
         propriétaire SEUL. Un créneau armé = annonce synthétique 🤖 (même
         rail, timer T-10 s, moteur unifié — Observation). -->
    <div class="mt-1 pt-1 border-t border-white/10 flex flex-col gap-1">
      <div class="flex items-center gap-2">
        <span class="text-[10px] font-bold uppercase tracking-wider text-white">🤖 Créneaux IA</span>
        <span class="text-[10px] text-white/70">{{ creneaux.filter(c => c.arme).length }}/{{ plafond }} armé(s)</span>
        <button
          class="ml-auto text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/50 text-white transition-colors disabled:opacity-40"
          :disabled="recalculEnCours"
          title="Recalcule les créneaux statistiques (24 mois) et re-note les non évalués par l'analyste"
          @click="recalculer"
        >{{ recalculEnCours ? '⏳' : '↻ Recalculer' }}</button>
      </div>
      <div v-if="!creneaux.length" class="text-[10px] text-white/70">
        {{ chargementCreneaux ? 'Chargement…' : 'Aucune proposition — le calcul quotidien (4h) ou le bouton Recalculer les produit' }}
      </div>
      <div
        v-for="c in creneaux" :key="`${c.asset}-${c.jour}-${c.heure}`"
        class="rounded-lg border px-2 py-1.5 flex flex-col gap-1 cursor-help"
        :class="c.arme ? 'bg-emerald-500/10 border-emerald-500/40' : 'bg-white/5 border-white/10'"
        :title="c.justification || 'Pas encore noté par l\'analyste'"
      >
        <div class="flex items-center gap-1.5 flex-wrap">
          <span class="text-[11px] font-semibold text-white">{{ c.asset }}</span>
          <span class="text-[11px] text-white">{{ JOURS[c.jour - 1] }} {{ c.heure }}h–{{ c.heure + 1 }}h</span>
          <span class="text-[10px] font-mono text-white/80">vol ×{{ c.ratio.toFixed(2) }}</span>
          <span class="text-[10px] font-mono text-white/60">fiab. {{ Math.round(c.fiabilite * 100) }}%</span>
          <span v-if="c.verdict_ia" class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
            :class="c.verdict_ia === 'ARMER' ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-white/5 text-white/70 border-white/10'"
          >IA : {{ c.verdict_ia }} {{ c.conviction ?? '—' }}/100</span>
          <button
            class="ml-auto text-[9px] px-1.5 py-0.5 rounded transition-colors"
            :class="c.arme
              ? 'bg-red-900/40 text-red-300 hover:bg-red-800/60 border border-red-500/30'
              : 'bg-emerald-900/40 text-emerald-300 hover:bg-emerald-800/60 border border-emerald-500/30'"
            :title="c.arme ? 'Désarmer ce créneau' : 'Armer — devient une annonce synthétique (2 jambes à l heure E, timer T-10 s, Observation). Toi seul armes.'"
            @click="basculer(c)"
          >{{ c.arme ? 'Désarmer' : 'Armer' }}</button>
        </div>
        <p v-if="c.justification" class="text-[9px] leading-snug text-white/70">{{ c.justification }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

interface AgendaApi {
  annonces: { ts: number; titre: string; devise: string; actifs: string[] }[]
  passes: { asset: string; direction: string }[]
}
interface CreneauIa {
  asset: string; jour: number; heure: number
  vol_pct: number; ratio: number; fiabilite: number; nb_semaines: number
  verdict_ia: string | null; conviction: number | null; justification: string | null
  arme: boolean
}

const JOURS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']

const agenda = ref<AgendaApi | null>(null)
const annonces = ref<AgendaApi['annonces']>([])
const passes = ref<AgendaApi['passes']>([])
const creneaux = ref<CreneauIa[]>([])
const plafond = ref(3)
const chargementCreneaux = ref(true)
const recalculEnCours = ref(false)

function heureLocale(ts: number): string {
  return new Intl.DateTimeFormat('fr-FR', { hour: '2-digit', minute: '2-digit' }).format(new Date(ts * 1000))
}

function compteARebours(ts: number): string {
  const d = ts - Math.floor(Date.now() / 1000)
  if (d <= 0) return 'en cours'
  const j = Math.floor(d / 86400)
  const h = Math.floor((d % 86400) / 3600)
  const m = Math.floor((d % 3600) / 60)
  if (j > 0) return `J-${j} ${h}h`
  if (h > 0) return `${h}h${String(m).padStart(2, '0')}`
  return `${m} min`
}

async function chargerCreneaux() {
  try {
    const res = await http.get<{ creneaux: CreneauIa[]; armes: number; plafond: number }>('/api/straddle/creneaux-ia')
    creneaux.value = res.data.creneaux ?? []
    plafond.value = res.data.plafond ?? 3
  } catch { creneaux.value = [] }
  chargementCreneaux.value = false
}

async function recalculer() {
  recalculEnCours.value = true
  try {
    await http.post('/api/straddle/creneaux-ia/calculer', null, { timeout: 180_000 })
    await chargerCreneaux()
  } catch { /* silencieux */ }
  recalculEnCours.value = false
}

async function basculer(c: CreneauIa) {
  try {
    await http.post(`/api/straddle/creneaux-ia/${c.arme ? 'ignorer' : 'armer'}`, {
      asset: c.asset, jour: c.jour, heure: c.heure,
    })
    await chargerCreneaux()
  } catch (e) {
    // plafond atteint ou créneau disparu — le prochain chargement réaffiche la vérité
    await chargerCreneaux()
  }
}

async function charger() {
  try {
    const res = await http.get<AgendaApi>('/api/straddle/agenda')
    const d = res.data as AgendaApi
    annonces.value = d.annonces ?? []
    passes.value = d.passes ?? []
  } catch { /* agenda indisponible */ }
}

let minuteur: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void charger()
  void chargerCreneaux()
  minuteur = setInterval(charger, 60_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>
