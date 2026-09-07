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

    <!-- §16-b (07/09) — Créneaux IA en « file d'attente » : slots en test
         (stats live), propositions dédoublonnées (1 par actif), réserve
         dépliable. La boucle statue au bout de N tirages : VALIDÉ reste
         armé, RÉFUTÉ est désarmé — l'armement reste au propriétaire SEUL. -->
    <div class="mt-1 pt-1 border-t border-white/10 flex flex-col gap-1">
      <div class="flex items-center gap-2">
        <span class="text-[10px] font-bold uppercase tracking-wider text-white">🤖 Créneaux IA</span>
        <span class="text-[10px] text-white/70">{{ armes }}/{{ plafond }} armé(s)</span>
        <button
          class="ml-auto text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/50 text-white transition-colors disabled:opacity-40"
          :disabled="recalculEnCours"
          title="Recalcule les créneaux statistiques (24 mois) et re-note les non évalués par l'analyste"
          @click="recalculer"
        >{{ recalculEnCours ? '⏳' : '↻ Recalculer' }}</button>
      </div>

      <!-- Seuils propriétaires de la boucle de validation (kv) -->
      <div class="flex items-center gap-1.5 text-[9px] text-white/60 flex-wrap">
        <span>Verdict après</span>
        <input v-model.number="seuilsMin" type="number" min="1" max="52"
               class="w-9 bg-white/10 rounded px-1 text-white outline-none" @change="sauverSeuils">
        <span>tirages · réfutation si ΣR ≤</span>
        <input v-model.number="seuilsPlancher" type="number" step="0.5" min="-10" max="0"
               class="w-12 bg-white/10 rounded px-1 text-white outline-none" @change="sauverSeuils">
      </div>

      <!-- Bannière : verdicts rendus par la boucle ces 7 derniers jours -->
      <div v-if="verdicts.length"
           class="rounded-lg border border-indigo-500/30 bg-indigo-500/10 px-2 py-1.5 text-[10px] text-white/85 flex flex-col gap-0.5">
        <span class="uppercase font-semibold tracking-wide text-indigo-300">Verdict de la boucle (7 j)</span>
        <span v-for="v in verdicts" :key="`v-${v.asset}-${v.jour}-${v.heure}`">
          {{ libelle(v) }} :
          <span :class="v.verdict_test === 'valide' ? 'text-emerald-400 font-semibold' : 'text-red-400 font-semibold'">
            {{ v.verdict_test === 'valide' ? '✅ validé' : '❌ réfuté — désarmé' }}
          </span>
          ({{ v.occurrences }} tirages, Σ {{ fmtR(v.somme_r) }}R)
        </span>
        <!-- Slot libéré par une réfutation → remplacement en un clic
             (l'initiative reste le clic du propriétaire). -->
        <button v-if="aRemplacer && armes < plafond"
                class="mt-0.5 self-start text-[9px] px-1.5 py-0.5 rounded transition-colors disabled:opacity-40
                       bg-emerald-900/40 text-emerald-300 hover:bg-emerald-800/60 border border-emerald-500/30"
                :disabled="armageEnCours"
                @click="armerFile">↻ Remplacer par la prochaine carte</button>
      </div>

      <div v-if="!slots.length && !file.length" class="text-[10px] text-white/70">
        {{ chargementCreneaux ? 'Chargement…' : 'Aucune proposition — le calcul quotidien (4h) ou le bouton Recalculer les produit' }}
      </div>

      <template v-for="it in items" :key="it.cle">
        <div v-if="it.kind === 'hdr'" class="flex items-center gap-1.5">
          <span class="text-[9px] uppercase tracking-wider text-white/50 mt-0.5">{{ it.label }}</span>
          <!-- Armement en lot : remplit les slots libres avec les têtes de
               file (actifs non déjà armés) — un clic au lieu de N. -->
          <button v-if="it.action === 'armer-file' && armes < plafond"
                  class="ml-auto text-[9px] px-1.5 py-0.5 rounded transition-colors disabled:opacity-40
                         bg-emerald-900/40 text-emerald-300 hover:bg-emerald-800/60 border border-emerald-500/30"
                  :disabled="armageEnCours"
                  :title="'Remplit les slots libres avec les meilleures propositions d actifs non déjà armés (plafond ' + plafond + ')'"
                  @click="armerFile">
            {{ armageEnCours ? '⏳' : `🚀 Armer (${Math.min(file.length, plafond - armes)})` }}
          </button>
        </div>
        <div v-else
             class="rounded-lg border px-2 py-1.5 flex flex-col gap-1 cursor-help"
             :class="it.role === 'slot'
               ? (it.c.verdict_test === 'valide' ? 'bg-emerald-500/15 border-emerald-500/50' : 'bg-emerald-500/10 border-emerald-500/40')
               : it.role === 'file' ? 'bg-white/5 border-white/15' : 'bg-white/5 border-white/10 opacity-70'"
             :title="it.c.justification || 'Pas encore noté par l analyste'">
          <div class="flex items-center gap-1.5 flex-wrap">
            <span class="text-[11px] font-semibold text-white">{{ it.c.asset }}</span>
            <span class="text-[11px] text-white">{{ JOURS[it.c.jour - 1] }} {{ it.c.heure }}h–{{ it.c.heure + 1 }}h</span>
            <span class="text-[10px] font-mono text-white/80">vol ×{{ it.c.ratio.toFixed(2) }}</span>
            <span class="text-[10px] font-mono text-white/60">fiab. {{ Math.round(it.c.fiabilite * 100) }}%</span>
            <template v-if="it.role === 'slot'">
              <span class="text-[10px] font-mono" :class="it.c.somme_r >= 0 ? 'text-emerald-300' : 'text-red-300'">
                {{ it.c.occurrences }}/{{ cible(it.c) }} tirages · Σ {{ fmtR(it.c.somme_r) }}R
              </span>
              <span v-if="it.c.verdict_test === 'valide'"
                    class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border bg-emerald-500/15 text-emerald-300 border-emerald-500/40">✅ VALIDÉ — pilier</span>
              <span v-else-if="it.c.verdict_test === 'incertain'"
                    class="text-[9px] px-1.5 py-0.5 rounded-full border bg-amber-500/10 text-amber-300 border-amber-500/30">⏳ prolongé</span>
            </template>
            <span v-else-if="it.c.verdict_ia" class="text-[9px] font-semibold px-1.5 py-0.5 rounded-full border"
                  :class="it.c.verdict_ia === 'ARMER' ? 'bg-emerald-500/10 text-emerald-300 border-emerald-500/30' : 'bg-white/5 text-white/50 border-white/10'"
            >IA : {{ it.c.verdict_ia }} {{ it.c.conviction ?? '—' }}/100</span>
            <button
              class="ml-auto text-[9px] px-1.5 py-0.5 rounded transition-colors shrink-0"
              :class="it.c.arme
                ? 'bg-red-900/40 text-red-300 hover:bg-red-800/60 border border-red-500/30'
                : 'bg-emerald-900/40 text-emerald-300 hover:bg-emerald-800/60 border border-emerald-500/30'"
              :title="it.c.arme ? 'Désarmer ce créneau' : 'Armer — devient une annonce synthétique (2 jambes à l heure E, timer T-10 s, Observation). Toi seul armes.'"
              @click="basculer(it.c)"
            >{{ it.c.arme ? 'Désarmer' : 'Armer' }}</button>
          </div>
          <p v-if="it.c.justification && it.role !== 'reserve'" class="text-[9px] leading-snug text-white/70">{{ it.c.justification }}</p>
        </div>
      </template>

      <button v-if="reserve > 0 && !reserveOuverte"
              class="text-left text-[9px] text-white/50 hover:text-white/80 transition-colors"
              @click="reserveOuverte = true">
        📦 Réserve ({{ reserve }} autres cartes) ▸
      </button>
      <button v-else-if="reserveOuverte"
              class="text-left text-[9px] text-white/50 hover:text-white/80 transition-colors"
              @click="reserveOuverte = false">
        📦 Replier la réserve ▾
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
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
  occurrences: number; somme_r: number; verdict_test: string | null; conclut_le: number | null
}
interface RepCreneaux {
  slots: CreneauIa[]; file: CreneauIa[]; reserve: number; reserve_liste: CreneauIa[]
  verdicts: CreneauIa[]; armes: number; plafond: number
  seuils: { min: number; plancher_r: number }
}
/// Une entrée de rendu : titre de groupe ou carte (slot / file / réserve).
type Item =
  | { kind: 'hdr'; label: string; cle: string; action?: 'armer-file' }
  | { kind: 'carte'; c: CreneauIa; role: 'slot' | 'file' | 'reserve'; cle: string }

const JOURS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']

const agenda = ref<AgendaApi | null>(null)
const annonces = ref<AgendaApi['annonces']>([])
const passes = ref<AgendaApi['passes']>([])
const slots = ref<CreneauIa[]>([])
const file = ref<CreneauIa[]>([])
const reserve = ref(0)
const reserveListe = ref<CreneauIa[]>([])
const reserveOuverte = ref(false)
const verdicts = ref<CreneauIa[]>([])
const plafond = ref(3)
const seuilsMin = ref(4)
const seuilsPlancher = ref(-1.5)
const chargementCreneaux = ref(true)
const recalculEnCours = ref(false)

const armes = computed(() => slots.value.length)

/// Un réfuté a libéré un slot ces 7 derniers jours → proposer le remplacement.
const aRemplacer = computed(() => verdicts.value.some(v => v.verdict_test === 'refute'))
const armageEnCours = ref(false)

/// Armement en lot : remplit les slots libres (un clic propriétaire).
async function armerFile() {
  armageEnCours.value = true
  try {
    await http.post('/api/straddle/creneaux-ia/armer-file', null)
  } catch { /* plafond atteint ou file vide — le rechargement réaffiche la vérité */ }
  await chargerCreneaux()
  armageEnCours.value = false
}

/// Liste plate rendue : en-têtes de groupe + cartes (ordre slots → file → réserve).
const items = computed<Item[]>(() => {
  const out: Item[] = []
  if (slots.value.length) {
    out.push({ kind: 'hdr', label: '🎯 Slots en test', cle: 'h-slots' })
    for (const c of slots.value) out.push({ kind: 'carte', c, role: 'slot', cle: `s-${c.asset}-${c.jour}-${c.heure}` })
  }
  if (file.value.length) {
    out.push({ kind: 'hdr', label: '🃏 Prochaines cartes — 1 par actif', cle: 'h-file', action: 'armer-file' })
    for (const c of file.value) out.push({ kind: 'carte', c, role: 'file', cle: `f-${c.asset}-${c.jour}-${c.heure}` })
  }
  if (reserveOuverte.value && reserveListe.value.length) {
    out.push({ kind: 'hdr', label: `📦 Réserve (${reserve.value})`, cle: 'h-res' })
    for (const c of reserveListe.value) out.push({ kind: 'carte', c, role: 'reserve', cle: `r-${c.asset}-${c.jour}-${c.heure}` })
  }
  return out
})

/// Cible de tirages du créneau (les incertains sont prolongés de 2).
function cible(c: CreneauIa): number {
  return c.verdict_test === 'incertain' ? seuilsMin.value + 2 : seuilsMin.value
}

function libelle(c: CreneauIa): string {
  return `${c.asset} ${JOURS[c.jour - 1]} ${c.heure}h–${c.heure + 1}h`
}

function fmtR(r: number): string {
  return `${r >= 0 ? '+' : ''}${r.toFixed(2)}`
}

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
    const res = await http.get<RepCreneaux>('/api/straddle/creneaux-ia')
    slots.value = res.data.slots ?? []
    file.value = res.data.file ?? []
    reserve.value = res.data.reserve ?? 0
    reserveListe.value = res.data.reserve_liste ?? []
    verdicts.value = res.data.verdicts ?? []
    plafond.value = res.data.plafond ?? 3
    seuilsMin.value = res.data.seuils?.min ?? 4
    seuilsPlancher.value = res.data.seuils?.plancher_r ?? -1.5
  } catch {
    slots.value = []
    file.value = []
  }
  chargementCreneaux.value = false
}

async function sauverSeuils() {
  try {
    await http.put('/api/straddle/creneaux-ia/seuils', {
      min: Math.round(seuilsMin.value),
      plancher_r: seuilsPlancher.value,
    })
  } catch { /* hors bornes : le rechargement suivant réaffiche la vérité */ }
  await chargerCreneaux()
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
  } catch { /* plafond atteint ou créneau disparu */ }
  // Dans tous les cas : recharger réaffiche slots/file/réserve à la vérité.
  await chargerCreneaux()
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
