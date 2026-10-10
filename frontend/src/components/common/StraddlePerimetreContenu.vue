<template>
  <!-- 🕐 Matrice croisée UNIQUE (owner 10/10, option A) : lignes = créneaux
       événements, colonnes = assets. Le toggle d'en-tête de colonne
       ajoute/retire l'asset du périmètre ; chaque cellule arme/désarme SA
       case ; le toggle de ligne arme/désarme tout le créneau. Seuils,
       verdicts et archive vivent en bandeau bas — tout au même endroit. -->
  <div class="flex flex-col gap-2.5 min-w-0">
    <!-- Barre d'outils : compteur + balayages (le titre vit dans l'en-tête
         de la modale — owner 10/10). -->
    <div class="flex items-center gap-2 flex-wrap">
      <span class="text-[10px] text-white/60">{{ armes }}/{{ total }} case(s) armée(s)</span>
      <div class="ml-auto flex items-center gap-1.5">
        <button class="text-[10px] px-2 py-1 rounded-lg bg-emerald-600/20 text-emerald-300 font-semibold hover:bg-emerald-600/30 transition disabled:opacity-40"
                :disabled="armEnCours" @click="arm.toutArmer(true)">⚡ Tout armer</button>
        <button class="text-[10px] px-2 py-1 rounded-lg bg-red-900/30 text-red-300 font-semibold hover:bg-red-900/50 transition disabled:opacity-40"
                :disabled="total === 0 || armEnCours" @click="arm.toutArmer(false)">Tout désarmer</button>
      </div>
    </div>

    <div v-if="armChargement" class="text-[11px] text-white/70 py-3 text-center">Chargement…</div>
    <div v-else-if="!evenementsArmement.length" class="text-[11px] text-white/60 py-3 text-center">
      Aucun créneau — le semis se fait au démarrage du backend.
    </div>

    <!-- LA MATRICE -->
    <div v-else class="overflow-auto rounded-lg border border-white/10 max-h-[62vh]">
      <table class="text-[10px] border-collapse min-w-full">
        <thead>
          <tr class="bg-black/30">
            <th class="sticky left-0 z-10 bg-[#141a26] px-2 py-1.5 text-left text-white/60 font-semibold uppercase tracking-wide min-w-[150px]">
              Créneau · heure Paris
            </th>
            <th v-for="col in colonnes" :key="col" class="px-1 py-1 min-w-[74px]">
              <button class="w-full flex flex-col items-center gap-0.5 rounded px-1 py-1 border transition-colors disabled:opacity-50"
                      :class="dansPerimetre(col)
                        ? 'border-emerald-500/40 bg-emerald-500/10 hover:bg-emerald-500/20'
                        : 'border-white/10 bg-white/[0.03] hover:bg-white/10'"
                      :disabled="armEnCours || colonneEnCours === col"
                      :title="dansPerimetre(col)
                        ? `${col} surveillé — clic : RETIRER du tableau (ses cases sont supprimées ; le réajouter relance un test à zéro)`
                        : `${col} hors périmètre — clic : AJOUTER (toutes ses cases seront armées, nouveau test)`"
                      @click="basculerColonne(col)">
                <span class="font-mono font-bold" :class="dansPerimetre(col) ? 'text-white' : 'text-white/50'">
                  <span :class="couleurCategorie(col)">●</span> {{ col }}
                </span>
                <span class="text-[8px] text-white/60">{{ armeesColonne(col) }}/{{ lignesColonne(col) }}
                  <template v-if="!dansPerimetre(col)">⚠</template>
                </span>
              </button>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="ev in evenementsArmement" :key="ev.ident" class="border-t border-white/5">
            <td class="sticky left-0 z-10 bg-[#141a26] px-2 py-1">
              <div class="flex items-center gap-1.5">
                <button class="shrink-0 w-4 h-4 rounded border text-[9px] leading-none flex items-center justify-center transition disabled:opacity-40"
                        :class="armeesLigne(ev) === ev.lignes.length
                          ? 'bg-emerald-500/30 border-emerald-400/60 text-emerald-300'
                          : armeesLigne(ev) > 0
                            ? 'bg-amber-500/20 border-amber-400/50 text-amber-300'
                            : 'bg-white/[0.04] border-white/15 text-white/40 hover:bg-white/10'"
                        :disabled="armEnCours"
                        :title="armeesLigne(ev) === ev.lignes.length ? 'Tout désarmer ce créneau' : 'Tout armer ce créneau (nouveau test à zéro)'"
                        @click="arm.basculerEvenement(ev.ident, armeesLigne(ev) !== ev.lignes.length)">{{ armeesLigne(ev) === ev.lignes.length ? '×' : armeesLigne(ev) > 0 ? '◐' : '+' }}</button>
                <div class="flex flex-col leading-tight min-w-0">
                  <span class="font-semibold text-white truncate">{{ ev.calendrier ? '📅' : '📯' }} {{ ev.nom }}</span>
                  <span class="font-mono text-white/60 text-[9px]" :title="ev.calendrier
                    ? 'Type calendaire : l\'heure est celle de la prochaine annonce réelle de ce type'
                    : ''">{{ ev.prochaine_heure_paris ?? '—' }} Paris</span>
                </div>
                <span class="ml-auto font-mono text-[9px] shrink-0" :class="sommeLigne(ev) >= 0 ? 'text-emerald-300/80' : 'text-red-300/80'">
                  {{ tiragesLigne(ev) }}t · Σ{{ fmtR(sommeLigne(ev)) }}R
                </span>
              </div>
            </td>
            <td v-for="col in colonnes" :key="ev.ident + col" class="px-0.5 py-0.5 text-center">
              <button v-if="ligneDe(ev, col)" class="w-full rounded px-0.5 py-1 font-mono font-bold border transition-colors disabled:opacity-50"
                      :class="classeCellule(ligneDe(ev, col)!)"
                      :title="titreLigne(ev, ligneDe(ev, col)!)"
                      :disabled="armEnCours"
                      @click="arm.basculer(col, ev.ident)">
                <span class="text-[11px]">{{ glypheCellule(ligneDe(ev, col)!) }}</span>
                <span v-if="ligneDe(ev, col)!.occurrences > 0" class="block text-[8px] font-normal"
                      :class="ligneDe(ev, col)!.somme_r >= 0 ? 'text-emerald-300/80' : 'text-red-300/80'">
                  {{ ligneDe(ev, col)!.occurrences }}t {{ fmtR(ligneDe(ev, col)!.somme_r) }}
                </span>
              </button>
              <span v-else class="text-white/20">·</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- + Ajouter un asset au périmètre (assets actifs sans cases) -->
    <div v-if="ajoutables.length" class="flex items-center gap-1.5 flex-wrap">
      <span class="text-[10px] text-white/60">＋ Ajouter un asset (toutes ses cases armées) :</span>
      <button v-for="a in ajoutables" :key="a.id"
              class="text-[9px] font-mono rounded px-1.5 py-0.5 border border-white/10 bg-white/[0.03] text-white/60 hover:bg-emerald-500/20 hover:border-emerald-400/40 hover:text-white transition disabled:opacity-40"
              :disabled="armEnCours"
              @click="basculerColonne(a.id)">{{ a.id }}</button>
    </div>

    <!-- Bandeau bas : seuils · verdicts · archive · légende -->
    <div class="flex flex-col gap-1.5 pt-1 border-t border-white/10">
      <div class="flex items-center gap-1.5 text-[9px] text-white/60 flex-wrap">
        <span>Verdict après</span>
        <input v-model.number="seuilsMin" type="number" min="1" max="52"
               class="w-9 bg-white/10 rounded px-1 text-white outline-none" @change="sauverSeuils">
        <span>tirages · réfutation si ΣR ≤</span>
        <input v-model.number="seuilsPlancher" type="number" step="0.5" min="-10" max="0"
               class="w-12 bg-white/10 rounded px-1 text-white outline-none" @change="sauverSeuils">
      </div>

      <div class="flex items-center gap-3 flex-wrap">
        <button class="text-left text-[9px] text-white/50 hover:text-white/80 transition-colors"
                @click="archiveOuverte = !archiveOuverte">
          📦 Créneaux statistiques — remplacés le 28/09 · {{ arm.archive.value.total }} lignes
          {{ archiveOuverte ? '▾' : '▸' }}
        </button>
        <RouterLink to="/straddle"
                    class="ml-auto text-[10px] px-2.5 py-1 rounded-lg bg-yellow-500/20 text-yellow-400 font-semibold hover:bg-yellow-500/30 transition"
                    @click="$emit('fermer')">→ Agenda sur la page Straddle</RouterLink>
      </div>
      <div v-if="archiveOuverte && arm.archive.value.verdicts.length"
           class="flex flex-col gap-0.5 text-[9px] text-white/60 pl-3 border-l border-white/10 max-h-32 overflow-y-auto">
        <span v-for="(v, i) in arm.archive.value.verdicts" :key="`a-${i}`">
          {{ v.asset }} {{ JOURS[v.jour - 1] }} {{ v.heure }}h :
          <span :class="v.verdict_test === 'valide' ? 'text-emerald-400' : 'text-red-400'">
            {{ v.verdict_test === 'valide' ? '✅ validé' : '❌ réfuté' }}
          </span>
          ({{ v.occurrences }} tirages, Σ {{ fmtR(v.somme_r) }}R)
        </span>
      </div>

      <!-- 🤖 Analyse IA des créneaux (owner 10/10) : l'analyste local lit les
           compteurs de la matrice et croise événement × asset. -->
      <div class="rounded-lg border border-white/10 bg-white/[0.03] p-2.5 flex flex-col gap-1.5">
        <div class="flex items-center gap-2 flex-wrap">
          <p class="text-[10px] font-bold uppercase tracking-wider text-white">🤖 Analyse des créneaux</p>
          <span v-if="ia && !iaEnCours" class="text-[9px] font-mono px-1.5 py-0.5 rounded-full border font-bold"
                :class="ia.confiance >= 70 ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300' : 'border-white/20 bg-white/5 text-white'">{{ ia.confiance }}/100</span>
          <span v-if="ia && !iaEnCours && iaCache" class="text-[9px] text-white/40">cache du jour</span>
          <button class="ml-auto text-[10px] px-2.5 py-1 rounded-lg font-semibold transition-colors disabled:opacity-40"
                  :class="iaEnCours ? 'bg-white/10 text-white' : 'bg-teal-500/20 text-teal-300 hover:bg-teal-500/30'"
                  :disabled="iaEnCours"
                  @click="genererIa()">{{ iaEnCours ? '⏳ Analyse… (~1 min)' : ia ? '↻ Régénérer' : '⚡ Générer' }}</button>
        </div>
        <div v-if="ia" class="flex flex-col gap-1.5">
          <p class="text-[11px] text-white leading-relaxed">{{ ia.etat }}</p>
          <div v-if="ia.meilleurs_evenements.length" class="text-[10px] text-white/85">
            <span class="font-semibold text-emerald-300">📈 Événements à travailler :</span>
            <ul class="list-disc list-inside ml-1 space-y-0.5">
              <li v-for="(e, i) in ia.meilleurs_evenements" :key="'e' + i">{{ e }}</li>
            </ul>
          </div>
          <div v-if="ia.meilleurs_croisements.length" class="text-[10px] text-white/85">
            <span class="font-semibold text-teal-300">🎯 Meilleurs croisements asset × événement :</span>
            <ul class="list-disc list-inside ml-1 space-y-0.5">
              <li v-for="(c, i) in ia.meilleurs_croisements" :key="'c' + i">{{ c }}</li>
            </ul>
          </div>
          <p class="text-[9px] text-white/50">L'analyste lit les compteurs réels des cases (règle des 30 par case) — il propose, l'armement reste à votre clic.</p>
        </div>
        <p v-else-if="iaErreur" class="text-[10px] text-red-300">{{ iaErreur }}</p>
        <p v-else class="text-[10px] text-white/50">L'analyste local (Ollama) croisera les cases (asset × événement) pour dire quoi travailler — généré à la demande, conservé pour la journée.</p>
      </div>

      <p class="text-[8px] text-white/70 leading-snug">
        Cellule : clic = armer/désarmer la case (armer remet le test à zéro) · ✅ validé · ❌ réfuté (désarmé par la boucle) · ⏳ prolongé · ⚠ hors périmètre (armé mais ignoré par le moteur) · ×N = réactivité mesurée (tooltip).
        En-tête de colonne : ajouter/retirer l'asset du périmètre. Heures Paris, bascules été/hiver.
      </p>
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

const alerteStore = useAlerteStore()
const JOURS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']
const archiveOuverte = ref(false)
const seuilsMin = ref(4)
const seuilsPlancher = ref(-1.5)
const assetsApi = ref<AssetApi[]>([])
const colonneEnCours = ref('')

/// État d'armement événementiel partagé (composable).
const arm = useEvenementsArmement()
const evenementsArmement = arm.evenements
const armChargement = arm.chargement
const armEnCours = arm.enCours

/// Colonnes = assets ayant des cases (tri par catégorie puis nom).
const ORDRE_CATEGORIES: Record<AssetApi['type'], number> = { crypto: 0, metal: 1, forex: 2, indice: 3 }
const typeDe = (a: string): AssetApi['type'] => assetsApi.value.find(x => x.id === a)?.type ?? 'crypto'
const colonnes = computed(() => {
  const set = new Set<string>()
  for (const ev of evenementsArmement.value) for (const l of ev.lignes) set.add(l.asset)
  return [...set].sort((a, b) =>
    ORDRE_CATEGORIES[typeDe(a)] - ORDRE_CATEGORIES[typeDe(b)] || a.localeCompare(b))
})

/// Assets actifs SANS cases — candidats à l'ajout au périmètre.
const ajoutables = computed(() =>
  assetsApi.value.filter(a => a.actif !== false && !colonnes.value.includes(a.id)))

function couleurCategorie(a: string): string {
  const t = typeDe(a)
  if (t === 'crypto') return 'text-yellow-400'
  if (t === 'metal') return 'text-amber-400'
  if (t === 'forex') return 'text-blue-400'
  return 'text-purple-400'
}

/// Totaux.
const total = computed(() => evenementsArmement.value.reduce((n, ev) => n + ev.lignes.length, 0))
const armes = computed(() => evenementsArmement.value.reduce(
  (n, ev) => n + ev.lignes.filter(l => l.arme).length, 0))

function dansPerimetre(a: string): boolean {
  return arm.assets.value.includes(a)
}
function lignesColonne(a: string): number {
  return evenementsArmement.value.reduce((n, ev) => n + (ev.lignes.some(l => l.asset === a) ? 1 : 0), 0)
}
function armeesColonne(a: string): number {
  return evenementsArmement.value.reduce(
    (n, ev) => n + (ev.lignes.find(l => l.asset === a)?.arme ? 1 : 0), 0)
}
function armeesLigne(ev: EvenementArmement): number {
  return ev.lignes.filter(l => l.arme).length
}
function tiragesLigne(ev: EvenementArmement): number {
  return ev.lignes.reduce((n, l) => n + l.occurrences, 0)
}
function sommeLigne(ev: EvenementArmement): number {
  return ev.lignes.reduce((s, l) => s + l.somme_r, 0)
}
function ligneDe(ev: EvenementArmement, a: string): LigneArmement | undefined {
  return ev.lignes.find(l => l.asset === a)
}

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

/// Toggle de colonne = ajouter/retirer l'asset du PÉRIMÈTRE : l'ajout sème
/// et arme toutes ses cases (backend) ; le retrait les laisse armées mais
/// ignorées (⚠) — mêmes règles que l'ancien volet « Assets surveillés ».
async function basculerColonne(a: string) {
  colonneEnCours.value = a
  try {
    const cible = dansPerimetre(a)
      ? arm.assets.value.filter(x => x !== a)
      : [...arm.assets.value, a]
    await http.put('/api/straddle/perimetre', { assets: cible })
    await arm.charger()
  } catch (e) {
    alerteStore.afficherErreur(`Périmètre : ${(e as Error).message}`)
  }
  colonneEnCours.value = ''
}

function classeCellule(l: LigneArmement): string {
  if (l.verdict_test === 'valide') return 'text-emerald-300 bg-emerald-500/15 border-emerald-400/50'
  if (l.verdict_test === 'refute') return 'text-red-300 bg-red-500/10 border-red-400/40 opacity-80'
  if (!l.arme) return 'text-white/50 bg-white/[0.03] border-white/10 hover:bg-white/10'
  return 'text-emerald-200 bg-emerald-500/10 border-emerald-400/30'
}

function glypheCellule(l: LigneArmement): string {
  if (l.verdict_test === 'valide') return '✅'
  if (l.verdict_test === 'refute') return '❌'
  if (l.verdict_test === 'incertain') return '⏳'
  if (l.hors_perimetre) return '⚠'
  return l.arme ? '◉' : '·'
}

function titreLigne(ev: EvenementArmement, l: LigneArmement): string {
  const ratio = l.ratio ? ` · réactivité ×${l.ratio.toFixed(2)}` : ''
  const verdict = l.verdict_test
    ? ` · ${l.verdict_test === 'valide' ? 'VALIDÉ' : l.verdict_test === 'refute' ? 'RÉFUTÉ (désarmé)' : 'incertain (prolongé)'}`
    : ''
  const hors = l.hors_perimetre ? ' · HORS PÉRIMÈTRE : ignoré par le moteur' : ''
  return `${l.asset} × ${ev.nom}${ratio} · ${l.occurrences} tirage(s) · Σ${l.somme_r >= 0 ? '+' : ''}${l.somme_r.toFixed(2)}R${verdict}${hors}`
}

// ── Analyse IA des créneaux (à la demande, cache du jour backend) ─────────
interface AnalyseCreneaux {
  etat: string
  meilleurs_evenements: string[]
  meilleurs_croisements: string[]
  confiance: number
  generee_le: number
}
const ia = ref<AnalyseCreneaux | null>(null)
const iaEnCours = ref(false)
const iaErreur = ref('')
const iaCache = ref(false)

async function genererIa() {
  iaEnCours.value = true
  iaErreur.value = ''
  try {
    const r = await http.post<{ en_cache: boolean; analyse: AnalyseCreneaux }>('/api/evenements/ia', null, { timeout: 180_000 })
    ia.value = r.data.analyse
    iaCache.value = r.data.en_cache
  } catch {
    iaErreur.value = 'Analyste indisponible — vérifier que le serveur Ollama est démarré.'
  }
  iaEnCours.value = false
}

onMounted(async () => {
  try {
    const res = await http.get<AssetApi[]>('/api/assets')
    assetsApi.value = res.data ?? []
  } catch { /* catégories indisponibles — tri par nom */ }
  await arm.charger()
  seuilsMin.value = arm.seuils.value.min
  seuilsPlancher.value = arm.seuils.value.plancher_r
})
</script>
