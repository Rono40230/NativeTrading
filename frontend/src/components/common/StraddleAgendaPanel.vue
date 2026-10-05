<template>
  <div class="flex flex-col gap-1.5">
    <div v-if="annonces.length" class="flex flex-col gap-1">
      <div v-for="a in annonces.slice(0, 6)" :key="a.ts + a.titre"
           class="flex items-center gap-2 text-xs">
        <span :class="a.titre.startsWith('📯') ? 'text-emerald-400' : 'text-amber-400'">{{
          a.titre.startsWith('📯') ? '📯' : '📅' }}</span>
        <span class="text-white font-medium truncate">{{ a.titre || 'Annonce US' }}</span>
        <span class="text-white/70 text-[10px] truncate">{{ a.actifs.join(' ') }}</span>
        <span class="text-white">{{ heureLocale(a.ts) }}</span>
        <span class="ml-auto text-amber-300/90 font-mono text-[11px]">{{ compteARebours(a.ts) }}</span>
      </div>
    </div>
    <div v-else class="text-[11px] text-white">Aucune annonce ni événement armé à 7 jours</div>
    <div v-if="passes.length" class="text-[11px] text-emerald-400/80">
      {{ passes.length }} passe(s) en cours sur {{ [...new Set(passes.map(p => p.asset))].join(', ') }}
    </div>

    <!-- 28/09 (phase 3) — Créneaux ÉVÉNEMENT : chaque événement de la
         taxonomie × chaque asset du périmètre est une case armable. La
         boucle statue au fil des tirages : ΣR > 0 → valide (pilier),
         ΣR ≤ plancher ou 0 gagnant → réfuté (désarmé), sinon incertain
         prolongé. L'armement reste au propriétaire SEUL. -->
    <div class="mt-1 pt-1 border-t border-white/10 flex flex-col gap-1">
      <div class="flex items-center gap-2 flex-wrap">
        <span class="text-[10px] font-bold uppercase tracking-wider text-white">📯 Créneaux événements</span>
        <span class="text-[10px] text-white/70">{{ armes }}/{{ total }} armé(s)</span>
        <div class="ml-auto flex items-center gap-1.5">
          <button class="text-[9px] px-1.5 py-0.5 rounded bg-emerald-900/40 text-emerald-300 hover:bg-emerald-800/60 border border-emerald-500/30 transition-colors disabled:opacity-40"
                  :disabled="arm.enCours.value" @click="arm.toutArmer(true)">⚡ Tout armer</button>
          <button class="text-[9px] px-1.5 py-0.5 rounded bg-red-900/40 text-red-300 hover:bg-red-800/60 border border-red-500/30 transition-colors disabled:opacity-40"
                  :disabled="total === 0 || arm.enCours.value" @click="arm.toutArmer(false)">Tout désarmer</button>
        </div>
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
      <div v-if="verdictsRecents.length"
           class="rounded-lg border border-indigo-500/30 bg-indigo-500/10 px-2 py-1.5 text-[10px] text-white/85 flex flex-col gap-0.5">
        <span class="uppercase font-semibold tracking-wide text-indigo-300">Verdict de la boucle (7 j)</span>
        <span v-for="v in verdictsRecents" :key="`v-${v.ident}-${v.asset}`">
          {{ v.asset }} × {{ v.nom }} :
          <span :class="v.verdict_test === 'valide' ? 'text-emerald-400 font-semibold' : 'text-red-400 font-semibold'">
            {{ v.verdict_test === 'valide' ? '✅ validé' : '❌ réfuté — désarmé' }}
          </span>
          ({{ v.occurrences }} tirages, Σ {{ fmtR(v.somme_r) }}R)
        </span>
      </div>

      <div v-if="arm.chargement.value" class="text-[10px] text-white/70">Chargement…</div>
      <div v-else-if="!arm.evenements.value.length"
           class="text-[10px] text-white/70">Aucune case — le semis se fait au démarrage du backend.</div>

      <template v-else>
        <div v-for="ev in arm.evenements.value" :key="ev.ident"
             class="rounded-lg border px-2 py-1.5 flex flex-col gap-1"
             :class="evArmees(ev) ? 'bg-emerald-500/10 border-emerald-500/40' : 'bg-white/5 border-white/10 opacity-70'">
          <div class="flex items-center gap-1.5 flex-wrap">
            <span class="text-[11px] font-semibold text-white">{{ ev.nom }}</span>
            <span class="text-[10px] font-mono text-white/70">{{ ev.prochaine_heure_paris ?? '—' }} Paris</span>
            <span class="text-[10px] font-mono" :class="sommeEv(ev) >= 0 ? 'text-emerald-300' : 'text-red-300'">
              {{ tiragesEv(ev) }} tirage(s) · Σ {{ fmtR(sommeEv(ev)) }}R
            </span>
            <span class="ml-auto text-[9px] text-white/60">{{ evArmees(ev) }}/{{ ev.lignes.length }} armé(s)</span>
          </div>
          <div class="flex flex-wrap items-center gap-1">
            <button v-for="l in ev.lignes" :key="ev.ident + l.asset"
                    class="text-[9px] font-mono font-bold rounded px-1.5 py-[1px] border transition-colors"
                    :class="classeLigne(l)"
                    :title="titreLigne(ev, l)"
                    :disabled="arm.enCours.value"
                    @click="arm.basculer(l.asset, ev.ident)">
              {{ l.asset }}<template v-if="l.ratio"> ×{{ l.ratio.toFixed(1) }}</template>
              <span v-if="l.occurrences > 0" class="font-normal"> {{ l.occurrences }}t·{{ fmtR(l.somme_r) }}R</span>
              <span v-if="l.verdict_test === 'valide'"> ✅</span>
              <span v-else-if="l.verdict_test === 'refute'"> ❌</span>
              <span v-else-if="l.verdict_test === 'incertain'"> ⏳</span>
              <span v-if="l.hors_perimetre"> ⚠</span>
            </button>
          </div>
        </div>
      </template>

      <!-- Archive : créneaux statistiques remplacés le 28/09 — lignes et
           verdicts conservés, plus jamais armés. -->
      <button class="text-left text-[9px] text-white/50 hover:text-white/80 transition-colors"
              @click="archiveOuverte = !archiveOuverte">
        📦 Créneaux statistiques — remplacés le 28/09 · {{ arm.archive.value.total }} lignes archivées
        {{ archiveOuverte ? '▾' : '▸' }}
      </button>
      <div v-if="archiveOuverte && arm.archive.value.verdicts.length"
           class="flex flex-col gap-0.5 text-[9px] text-white/60 pl-3 border-l border-white/10">
        <span v-for="(v, i) in arm.archive.value.verdicts" :key="`a-${i}`">
          {{ v.asset }} {{ JOURS[v.jour - 1] }} {{ v.heure }}h :
          <span :class="v.verdict_test === 'valide' ? 'text-emerald-400' : 'text-red-400'">
            {{ v.verdict_test === 'valide' ? '✅ validé' : '❌ réfuté' }}
          </span>
          ({{ v.occurrences }} tirages, Σ {{ fmtR(v.somme_r) }}R)
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useAlerteStore } from '@/stores/alerte.store'
import { http } from '@/services/http.client'
import { useEvenementsArmement } from '@/composables/useEvenementsArmement'
import type { EvenementArmement, LigneArmement } from '@/composables/useEvenementsArmement'

interface AgendaApi {
  annonces: { ts: number; titre: string; devise: string; actifs: string[] }[]
  passes: { asset: string; direction: string }[]
}

const alerteStore = useAlerteStore()
const JOURS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']

const agenda = ref<AgendaApi | null>(null)
const annonces = ref<AgendaApi['annonces']>([])
const passes = ref<AgendaApi['passes']>([])
const archiveOuverte = ref(false)
const seuilsMin = ref(4)
const seuilsPlancher = ref(-1.5)

/// État d'armement événementiel partagé (composable).
const arm = useEvenementsArmement()

const total = computed(() => arm.evenements.value.reduce((n, ev) => n + ev.lignes.length, 0))
const armes = computed(() => arm.evenements.value.reduce(
  (n, ev) => n + ev.lignes.filter(l => l.arme).length, 0))

/// Verdicts conclus ces 7 derniers jours (bannière).
const ilYA7j = Math.floor(Date.now() / 1000) - 7 * 86_400
const verdictsRecents = computed(() =>
  arm.evenements.value.flatMap(ev =>
    ev.lignes
      .filter(l => l.verdict_test && l.verdict_test !== 'incertain' && (l.conclut_le ?? 0) > ilYA7j)
      .map(l => ({ ...l, ident: ev.ident, nom: ev.nom }))))

function evArmees(ev: EvenementArmement): number {
  return ev.lignes.filter(l => l.arme).length
}
function tiragesEv(ev: EvenementArmement): number {
  return ev.lignes.reduce((n, l) => n + l.occurrences, 0)
}
function sommeEv(ev: EvenementArmement): number {
  return ev.lignes.reduce((s, l) => s + l.somme_r, 0)
}

function classeLigne(l: LigneArmement): string {
  if (l.verdict_test === 'valide') return 'text-emerald-300 bg-emerald-500/15 border-emerald-400/50'
  if (l.verdict_test === 'refute') return 'text-red-300 bg-red-500/10 border-red-400/40 opacity-80'
  if (!l.arme) return 'text-white/50 bg-white/[0.03] border-white/10 hover:bg-white/10'
  return 'text-emerald-200 bg-emerald-500/10 border-emerald-400/30'
}

function titreLigne(ev: EvenementArmement, l: LigneArmement): string {
  const ratio = l.ratio ? ` · réactivité ×${l.ratio.toFixed(2)}` : ''
  const verdict = l.verdict_test
    ? ` · ${l.verdict_test === 'valide' ? 'VALIDÉ' : l.verdict_test === 'refute' ? 'RÉFUTÉ (désarmé)' : 'incertain (prolongé)'}`
    : ''
  const hors = l.hors_perimetre ? ' · HORS PÉRIMÈTRE : ignoré par le moteur' : ''
  return `${l.asset} × ${ev.nom}${ratio} · ${l.occurrences} tirage(s) · Σ${l.somme_r >= 0 ? '+' : ''}${l.somme_r.toFixed(2)}R${verdict}${hors}`
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

async function chargerArmement() {
  await arm.charger()
  seuilsMin.value = arm.seuils.value.min
  seuilsPlancher.value = arm.seuils.value.plancher_r
}

async function sauverSeuils() {
  try {
    await arm.sauverSeuils(seuilsMin.value, seuilsPlancher.value)
  } catch (e) {
    alerteStore.afficherErreur(`Seuils : ${(e as Error).message}`)
  }
}

async function charger() {
  try {
    const res = await http.get<AgendaApi>('/api/straddle/agenda')
    const d = res.data as AgendaApi
    annonces.value = d.annonces ?? []
    passes.value = d.passes ?? []
  } catch (e) {
    alerteStore.afficherErreur(`Agenda : ${(e as Error).message}`)
  }
}

let minuteur: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void charger()
  void chargerArmement()
  minuteur = setInterval(charger, 60_000)
})
onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })
</script>
