<template>
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- Volet gauche : sélection des assets -->
      <div class="flex flex-col gap-3">
        <div class="flex items-center gap-2">
          <p class="text-xs font-bold text-white uppercase tracking-wider">Assets surveillés</p>
          <span class="text-[10px] text-white/50 ml-auto">{{ selection.length }} sélectionné(s)</span>
        </div>
        <p class="text-[10px] text-amber-300/80 border-l-2 border-amber-400/40 pl-2">
          Un asset ajouté reçoit automatiquement tous ses créneaux événements armés (balayage large 28/09).
        </p>
        <div v-for="cat in categories" :key="cat.type" class="flex flex-col gap-1">
          <p class="text-[11px] font-semibold uppercase tracking-wide" :class="cat.couleur">{{ cat.label }}</p>
          <div class="flex flex-wrap gap-1">
            <label
              v-for="a in cat.assets" :key="a"
              class="flex items-center gap-1.5 cursor-pointer rounded border px-2 py-1 text-xs transition select-none"
              :class="selection.includes(a)
                ? 'border-emerald-500/40 bg-emerald-500/10 text-white'
                : 'border-white/10 bg-white/[0.02] text-white/60 hover:bg-white/[0.06] hover:border-white/20'"
            >
              <input type="checkbox" class="hidden" :checked="selection.includes(a)" @change="basculerAsset(a)" />
              {{ a }}
            </label>
          </div>
        </div>
        <div class="flex items-center gap-2 mt-1">
          <button class="text-xs px-3 py-1.5 rounded-lg font-semibold border border-white/10 transition-colors disabled:opacity-40"
                  :class="modifie ? 'bg-emerald-600/20 text-emerald-300 hover:bg-emerald-600/30' : 'bg-white/5 text-white'"
                  :disabled="!modifie || enCours" @click="enregistrer">
            {{ enCours ? '⏳…' : '✅ Enregistrer le périmètre' }}
          </button>
          <span v-if="message" class="text-[11px]" :class="erreur ? 'text-red-400' : 'text-emerald-300'">{{ message }}</span>
        </div>
      </div>

      <!-- Volet droit : poste de commandement ÉVÉNEMENTS (28/09, phase 3).
           Chaque ligne = un événement de la taxonomie (heure DST-aware) ;
           les chips = une case armable (asset × événement) avec sa
           réactivité mesurée (×N) et l'état de son test. -->
      <div class="flex flex-col gap-2 min-w-0">
        <div class="flex items-center gap-2 flex-wrap">
          <p class="text-xs font-bold text-white uppercase tracking-wider">📯 Créneaux événements</p>
          <span class="text-[10px] text-white/60">{{ armes }}/{{ total }} armé(s)</span>
          <div class="ml-auto flex items-center gap-1.5">
            <button class="text-[10px] px-2 py-1 rounded-lg bg-emerald-600/20 text-emerald-300 font-semibold hover:bg-emerald-600/30 transition disabled:opacity-40"
                    :disabled="armEnCours" @click="arm.toutArmer(true)">
              ⚡ Tout armer
            </button>
            <button class="text-[10px] px-2 py-1 rounded-lg bg-red-900/30 text-red-300 font-semibold hover:bg-red-900/50 transition disabled:opacity-40"
                    :disabled="total === 0 || armEnCours" @click="arm.toutArmer(false)">
              Tout désarmer
            </button>
          </div>
        </div>

        <!-- Seuils propriétaires de la boucle de validation (kv) — déplacés
             de la page Straddle (owner 10/10 : un seul poste d'armement). -->
        <div class="flex items-center gap-1.5 text-[9px] text-white/60 flex-wrap">
          <span>Verdict après</span>
          <input v-model.number="seuilsMin" type="number" min="1" max="52"
                 class="w-9 bg-white/10 rounded px-1 text-white outline-none" @change="sauverSeuils">
          <span>tirages · réfutation si ΣR ≤</span>
          <input v-model.number="seuilsPlancher" type="number" step="0.5" min="-10" max="0"
                 class="w-12 bg-white/10 rounded px-1 text-white outline-none" @change="sauverSeuils">
        </div>

        <!-- Bannière : verdicts rendus par la boucle ces 7 derniers jours. -->
        <div v-if="verdictsRecents.length"
             class="rounded-lg border border-indigo-500/30 bg-indigo-500/10 px-2 py-1.5 text-[10px] text-white/85 flex flex-col gap-0.5">
          <span class="uppercase font-semibold tracking-wide text-indigo-300">Validés par la boucle (7 j)</span>
          <span v-for="v in verdictsRecents" :key="`v-${v.ident}-${v.asset}`">
            {{ v.asset }} × {{ v.nom }} :
            <span :class="v.verdict_test === 'valide' ? 'text-emerald-400 font-semibold' : 'text-red-400 font-semibold'">
              {{ v.verdict_test === 'valide' ? '✅ validé' : '❌ réfuté — désarmé' }}
            </span>
            ({{ v.occurrences }} tirages, Σ {{ fmtR(v.somme_r) }}R)
          </span>
        </div>

        <div v-if="armChargement" class="text-[11px] text-white/70 py-2 text-center">Chargement…</div>
        <div v-else-if="!evenementsArmement.length" class="text-[11px] text-white/60 py-2 text-center">
          Aucun créneau — le semis se fait au démarrage du backend.
        </div>

        <div v-else class="flex flex-col gap-1.5 overflow-y-auto max-h-[72vh] pr-1">
          <div v-for="ev in evenementsArmement" :key="ev.ident"
               class="rounded-lg border border-white/10 bg-black/20 px-2.5 py-1.5 flex flex-col gap-1">
            <div class="flex items-baseline justify-between gap-2 flex-wrap">
              <p class="text-[11px] font-bold text-white leading-snug">{{ ev.nom }}</p>
              <span class="text-[9px] font-mono text-white shrink-0">
                <span class="font-bold">{{ ev.prochaine_heure_paris ?? '—' }}</span> Paris
                <span class="text-white/50">· {{ ev.heure_locale }} {{ nomFuseau(ev.fuseau) }}</span>
              </span>
            </div>
            <div class="flex flex-wrap items-center gap-1">
              <button v-for="l in ev.lignes" :key="ev.ident + l.asset"
                      class="text-[9px] font-mono font-bold rounded px-1.5 py-[1px] border transition-colors"
                      :class="classeLigne(l)"
                      :title="titreLigne(ev, l)"
                      :disabled="armEnCours"
                      @click="arm.basculer(l.asset, ev.ident)">
                {{ l.asset }}<template v-if="l.ratio"> ×{{ l.ratio.toFixed(1) }}</template>
                <span v-if="l.verdict_test === 'valide'"> ✅</span>
                <span v-else-if="l.verdict_test === 'refute'"> ❌</span>
                <span v-else-if="l.verdict_test === 'incertain'"> ⏳</span>
                <span v-if="l.hors_perimetre"> ⚠</span>
              </button>
            </div>
          </div>
        </div>
        <p class="text-[8px] text-white/70 leading-snug">
          Clic sur une chip : armer/désarmer la case (asset × événement). ×N = réactivité mesurée (ATR des 3 premières minutes vs habitude).
          ✅ validé · ❌ réfuté (désarmé par la boucle) · ⏳ prolongé · ⚠ hors périmètre (armé mais ignoré par le moteur).
          Heures Paris avec bascules été/hiver.
        </p>
        <!-- Archive : créneaux statistiques remplacés le 28/09 — lignes et
             verdicts conservés, plus jamais armés (déplacée de la page). -->
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
        <RouterLink to="/straddle"
          class="self-start text-[10px] px-2.5 py-1.5 rounded-lg bg-yellow-500/20 text-yellow-400 font-semibold hover:bg-yellow-500/30 transition"
          @click="$emit('fermer')">→ Agenda complet sur la page Straddle</RouterLink>
      </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { http } from '@/services/http.client'
import { useAlerteStore } from '@/stores/alerte.store'
import { useEvenementsArmement } from '@/composables/useEvenementsArmement'
import type { EvenementArmement, LigneArmement } from '@/composables/useEvenementsArmement'

interface AssetApi { id: string; type: 'crypto' | 'metal' | 'forex' | 'indice'; actif?: boolean }

defineEmits<{ (e: 'fermer'): void }>()

const assets = ref<AssetApi[]>([])
const selection = ref<string[]>([])
const selectionInitiale = ref<string[]>([])
const enCours = ref(false)
const message = ref('')
const erreur = ref(false)

const alerteStore = useAlerteStore()
const JOURS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']
const archiveOuverte = ref(false)
const seuilsMin = ref(4)
const seuilsPlancher = ref(-1.5)

/// État d'armement événementiel partagé (composable).
const arm = useEvenementsArmement()
const evenementsArmement = arm.evenements
const armChargement = arm.chargement
const armEnCours = arm.enCours

const categories = computed(() => [
  { type: 'crypto', label: '🪙 Cryptos', couleur: 'text-yellow-400', assets: assets.value.filter(a => a.type === 'crypto').map(a => a.id) },
  { type: 'metal', label: '🥇 Métaux', couleur: 'text-amber-400', assets: assets.value.filter(a => a.type === 'metal').map(a => a.id) },
  { type: 'forex', label: '💱 Forex', couleur: 'text-blue-400', assets: assets.value.filter(a => a.type === 'forex').map(a => a.id) },
  { type: 'indice', label: '📈 Indices', couleur: 'text-purple-400', assets: assets.value.filter(a => a.type === 'indice').map(a => a.id) },
])

const modifie = computed(() =>
  JSON.stringify([...selection.value].sort()) !== JSON.stringify([...selectionInitiale.value].sort()))

/// VALIDÉS conclus ces 7 derniers jours (bannière — owner 10/10 : validés
/// seulement, les réfutés s'accumulaient en bruit ; leur état reste visible
/// dans la matrice, chips ❌ désarmées).
const ilYA7j = Math.floor(Date.now() / 1000) - 7 * 86_400
const verdictsRecents = computed(() =>
  evenementsArmement.value.flatMap(ev =>
    ev.lignes
      .filter(l => l.verdict_test === 'valide' && (l.conclut_le ?? 0) > ilYA7j)
      .map(l => ({ ...l, ident: ev.ident, nom: ev.nom }))))

function fmtR(r: number): string {
  return `${r >= 0 ? '+' : ''}${r.toFixed(2)}`
}

async function sauverSeuils() {
  try {
    await arm.sauverSeuils(seuilsMin.value, seuilsPlancher.value)
  } catch (e) {
    alerteStore.afficherErreur(`Seuils : ${(e as Error).message}`)
  }
}

/// Totaux d'armement toutes cases confondues.
const total = computed(() => evenementsArmement.value.reduce((n, ev) => n + ev.lignes.length, 0))
const armes = computed(() => evenementsArmement.value.reduce(
  (n, ev) => n + ev.lignes.filter(l => l.arme).length, 0))

function nomFuseau(fuseau: string): string {
  if (fuseau === 'America/New_York') return 'NY'
  if (fuseau === 'Europe/London') return 'Londres'
  return ''
}

/// Style d'une chip : validée = émeraude forte, réfutée = rouge, armée =
/// émeraude douce, désarmée = atténuée.
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

async function charger() {
  message.value = ''
  try {
    const [resA, resP] = await Promise.all([
      http.get<AssetApi[]>('/api/assets'),
      http.get<{ assets: string[] }>('/api/straddle/perimetre'),
    ])
    assets.value = (resA.data ?? []).filter(a => a.actif !== false)
    selection.value = [...resP.data.assets]
    selectionInitiale.value = [...resP.data.assets]
  } catch { /* modale vide */ }
}

function basculerAsset(a: string) {
  const i = selection.value.indexOf(a)
  if (i >= 0) selection.value.splice(i, 1)
  else selection.value.push(a)
}

async function enregistrer() {
  enCours.value = true
  message.value = ''
  try {
    await http.put('/api/straddle/perimetre', { assets: selection.value })
    selectionInitiale.value = [...selection.value]
    message.value = '✓ Périmètre enregistré — créneaux événements semés pour les nouveaux assets (appliqué ≤ 60 s)'
    erreur.value = false
    await arm.charger()
  } catch (e) {
    erreur.value = true
    message.value = `❌ ${(e as Error).message}`
  }
  enCours.value = false
}

onMounted(async () => {
  void charger()
  await arm.charger()
  seuilsMin.value = arm.seuils.value.min
  seuilsPlancher.value = arm.seuils.value.plancher_r
})
</script>
