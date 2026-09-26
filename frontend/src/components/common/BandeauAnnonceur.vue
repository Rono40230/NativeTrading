<template>
  <!-- BANDEAU ANNONCEUR (cockpit 26/09) : les lampes systèmes et le master
       caution ont quitté le bandeau — leur données vivent dans le pedestal
       COMMUNICATIONS & NAVIGATION (bloc Données/IA/Graphiques). Restent
       trois BADGES D'INSTRUMENTATION (owner 26/09), chacun résumant l'essentiel
       dans sa fenêtre digitale et ouvrant une modale avec le composant
       complet (calendrier 10 j, créneaux moyens, radar ATR). -->
  <div class="glass-card px-3 py-2 flex items-stretch justify-center gap-4 flex-wrap">

    <!-- ── Badge Calendrier économique ─────────────────────────────────── -->
    <PopoverInfo titre="Calendrier économique" :texte="texteCalendrier" class="flex-1 min-w-[280px]">
      <button
        class="korry w-full cursor-pointer"
        :class="annonceImminente ? 'lit-rouge' : annoncesJour.length || !etatCalendrier.source_prete ? 'lit-ambre' : ''"
        @click.stop="ouverte = 'calendrier'"
      >
        <span class="k-label">CALENDRIER <span class="k-info">ⓘ</span></span>
        <span class="k-fenetre" :class="etatCalendrier.source_prete && !annonceImminente ? '' : 'ko'">{{ resumeCalendrier }}</span>
      </button>
    </PopoverInfo>

    <!-- ── Badge Créneaux moyens de volatilité ─────────────────────────── -->
    <PopoverInfo titre="Créneaux moyen de volatilité sur 24 mois" :texte="texteCreneaux" class="flex-1 min-w-[280px]">
      <button
        class="korry w-full cursor-pointer"
        @click.stop="ouverte = 'creneaux'"
      >
        <span class="k-label">CRÉNEAUX <span class="k-info">ⓘ</span></span>
        <span class="k-fenetre k-fenetre-multi">{{ resumeCreneaux }}</span>
      </button>
    </PopoverInfo>

    <!-- ── Badge Radar ATR temps réel ──────────────────────────────────── -->
    <PopoverInfo titre="Radar ATR temps réel" :texte="texteRadar" class="flex-1 min-w-[280px]">
      <button
        class="korry w-full cursor-pointer"
        @click.stop="ouverte = 'radar'"
      >
        <span class="k-label">RADAR ATR <span class="k-info">ⓘ</span></span>
        <span class="k-fenetre k-fenetre-multi">
          <template v-if="topRadar.length">{{ topRadar.join(' · ') }}</template>
          <template v-else>{{ radarChargement ? 'calcul…' : '—' }}</template>
        </span>
      </button>
    </PopoverInfo>

    <!-- ── Modales : le composant complet de l'ancienne colonne latérale,
         monté uniquement à l'ouverture (pas de polling fantôme). ──────── -->
    <ModaleCadre v-if="ouverte === 'calendrier'" titre="📅 Calendrier économique — 10 prochains jours" large @fermer="ouverte = null">
      <p v-if="!etatCalendrier.source_prete" class="text-[11px] text-amber-300 bg-amber-500/10 border border-amber-500/30 rounded-lg px-2.5 py-1.5 mb-2">
        Source ForexFactory injoignable{{ etatCalendrier.dernier_fetch ? ` — dernière synchro : ${synchroCourt}` : '' }}.
        Relecture automatique toutes les 3 h jusqu'au retour (puis lundi/jeudi).
      </p>
      <EconomicCalendar :jours="10" />
    </ModaleCadre>
    <ModaleCadre v-if="ouverte === 'creneaux'" titre="⏰ Créneaux de volatilité moyen sur 24 mois" large @fermer="ouverte = null">
      <CreneauxVolatiliteBloc ouvert-defaut />
    </ModaleCadre>
    <ModaleCadre v-if="ouverte === 'radar'" titre="🌡️ Radar ATR temps réel" large @fermer="ouverte = null">
      <RadarAtrBloc ouvert-defaut />
    </ModaleCadre>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import { newsApi } from '@/services/api.news'
import type { AnnonceCalendrier, ReponsePatternsVolatilite } from '@/services/api.service'
import type { PatternHoraire } from '@/services/api.types.marche'
import { offsetParisHeures } from '@/utils/date'
import { fenetreDuJour, NOM_CLUSTER } from '@/composables/useVolatiliteAnalyse'
import { useRadarAtr } from '@/composables/useRadarAtr'
import PopoverInfo from './PopoverInfo.vue'
import ModaleCadre from './ModaleCadre.vue'
import EconomicCalendar from './EconomicCalendar.vue'
import CreneauxVolatiliteBloc from './CreneauxVolatiliteBloc.vue'
import RadarAtrBloc from './RadarAtrBloc.vue'

/// Modale ouverte (une seule à la fois).
const ouverte = ref<'calendrier' | 'creneaux' | 'radar' | null>(null)

// ── Badge Calendrier : les annonces à fort impact du jour ───────────────────
const annoncesProches = ref<AnnonceCalendrier[]>([])
const etatCalendrier = ref<{ dernier_fetch: string | null; nb_futurs: number; source_prete: boolean }>({ dernier_fetch: null, nb_futurs: 0, source_prete: true })

/// « hier 14:02 » / « 3 sep 09:00 » pour la bannière de la modale.
const synchroCourt = computed(() => {
  const iso = etatCalendrier.value.dernier_fetch
  if (!iso) return ''
  const d = new Date(iso)
  const opts: Intl.DateTimeFormatOptions = d.toDateString() === new Date().toDateString()
    ? { hour: '2-digit', minute: '2-digit' }
    : { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' }
  return new Intl.DateTimeFormat('fr-FR', opts).format(d)
})

/// Annonces du jour (Paris) — fort impact uniquement (même filtre que le bloc).
const annoncesJour = computed(() => {
  const offset = offsetParisHeures() * 3_600_000
  const maintenant = new Date()
  const jourParis = new Date(maintenant.getTime() + offset).toISOString().slice(0, 10)
  return annoncesProches.value.filter(a => a.impact === 'High' && new Date(a.date_heure).toISOString().slice(0, 10) === jourParis)
})

const resumeCalendrier = computed(() => {
  if (annonceImminente.value) {
    const min = Math.max(1, Math.round((annonceImminente.value.ts - Date.now()) / 60_000))
    return `⚠ ${annonceImminente.value.devise} dans ${min} min`
  }
  if (!etatCalendrier.value.source_prete && !annoncesJour.value.length) return 'source injoignable'
  return annoncesJour.value.length ? `${annoncesJour.value.length} annonce${annoncesJour.value.length > 1 ? 's' : ''} aujourd'hui` : 'aucune aujourd\'hui'
})

/// Annonce High imminente (≤ 15 min) — le badge clignote en rouge (owner
/// 26/09 : c'est l'UNIQUE alerte annonce ; les toasts ont été retirés).
/// Horloge locale 20 s : les événements sont connus à l'avance, la fenêtre
/// de 15 min bascule à la seconde près sans re-fetch.
const maintenant = ref(Date.now())
let horloge: ReturnType<typeof setInterval> | null = null
const annonceImminente = computed(() => {
  for (const a of annoncesProches.value) {
    if (a.impact !== 'High') continue
    const diff = new Date(a.date_heure).getTime() - maintenant.value
    if (diff > 0 && diff <= 15 * 60_000) {
      return { devise: a.devise, titre: a.titre, ts: new Date(a.date_heure).getTime() }
    }
  }
  return null
})

const texteCalendrier = computed(() =>
  annoncesJour.value.length
    ? 'Annonces à fort impact du jour (heure Paris) :\n' + annoncesJour.value.map(a => `${a.devise} — ${a.titre}`).join('\n') + '\n\nClic : les 10 prochains jours.'
    : 'Pas d\'annonces prévues aujourd\'hui.\n\nClic : les 10 prochains jours.')

async function chargerCalendrier() {
  etatCalendrier.value = await newsApi.etatCalendrier()
  annoncesProches.value = await apiService.obtenirCalendrier(2)
}

// ── Badge Créneaux : la meilleure fenêtre moyenne du jour ───────────────────
const patterns = ref<ReponsePatternsVolatilite[]>([])

const resumeCreneaux = computed(() => {
  if (!patterns.value.length) return '…'
  const jour = new Date(Date.now() + offsetParisHeures() * 3_600_000).getUTCDay()
  // La fenêtre du jour la plus représentée : chaque asset vote pour sa
  // meilleure fenêtre ; on retient celle qui rassemble le PLUS d'actifs
  // (une fenêtre « extrême » d'un seul actif ne résume pas le jour) —
  // l'intensité départage les égalités.
  const votes = new Map<string, { debut: number; fin: number; cluster: number; assets: { nom: string; atr: number }[] }>()
  for (const d of patterns.value) {
    const f = fenetreDuJour(d.patterns as PatternHoraire[], jour)
    if (!f) continue
    const cle = `${f.heureDebut}-${f.heureFin}-${f.cluster}`
    const v = votes.get(cle) ?? { debut: f.heureDebut, fin: f.heureFin, cluster: f.cluster, assets: [] }
    v.assets.push({ nom: d.asset, atr: f.atr ?? 0 })
    votes.set(cle, v)
  }
  const gagnante = [...votes.values()].sort((a, b) => b.assets.length - a.assets.length || b.cluster - a.cluster)[0]
  if (!gagnante) return '—'
  // Les 4 actifs sélectionnés par le tri (intensité ATR de leur fenêtre,
  // décroissant) — le reste compte en +N (owner 26/09).
  const tries = [...gagnante.assets].sort((a, b) => b.atr - a.atr)
  const top4 = tries.slice(0, 4).map(a => a.nom).join(' ')
  const reste = tries.length - 4
  const noms = reste > 0 ? `${top4} +${reste}` : top4
  return `${gagnante.debut}h–${gagnante.fin}h · ${NOM_CLUSTER[gagnante.cluster].toLowerCase()} · ${noms}`
})

const detailCreneaux = computed(() => {
  if (!patterns.value.length) return ''
  const jour = new Date(Date.now() + offsetParisHeures() * 3_600_000).getUTCDay()
  const votes = new Map<string, { debut: number; fin: number; cluster: number; assets: { nom: string; atr: number }[] }>()
  for (const d of patterns.value) {
    const f = fenetreDuJour(d.patterns as PatternHoraire[], jour)
    if (!f) continue
    const cle = `${f.heureDebut}-${f.heureFin}-${f.cluster}`
    const v = votes.get(cle) ?? { debut: f.heureDebut, fin: f.heureFin, cluster: f.cluster, assets: [] }
    v.assets.push({ nom: d.asset, atr: f.atr ?? 0 })
    votes.set(cle, v)
  }
  const g = [...votes.values()].sort((a, b) => b.assets.length - a.assets.length || b.cluster - a.cluster)[0]
  if (!g) return ''
  return [...g.assets].sort((a, b) => b.atr - a.atr)
    .map(a => `${a.nom} : ATR moyen ${a.atr.toFixed(1)}`).join('\n')
})

const texteCreneaux = computed(() =>
  `Fenêtre la plus volatile du jour (heure Paris), d'après les patterns moyens des 24 derniers mois — les 4 actifs les plus intenses sont affichés, l'ordre complet est ici :\n${detailCreneaux.value}\n\nClic : tous les créneaux moyens par actif.`)

async function chargerCreneaux() {
  try {
    patterns.value = await apiService.obtenirPatternsJourTousActifs()
  } catch { patterns.value = [] }
}

// ── Badge Radar ATR : top-5, cycle horaire (owner 26/09) ────────────────────
const { classement: classementRadar, chargement: radarChargement, demarrer: demarrerRadar } = useRadarAtr()
let arreterRadar: (() => void) | null = null

const topRadar = computed(() =>
  classementRadar.value.slice(0, 5).map(i => `${i.asset}·${i.tf} ${i.atr.toFixed(0)}%`))

const texteRadar = computed(() =>
  `Volatilité actuelle vs habitude de cet instant (jour × heure, 24 mois) — les 5 paires TF/asset les plus chaudes, remises à jour chaque heure.\n\nClic : le classement complet.`)

// ── Cycles : calendrier 30 min (les agendas bougent lentement), créneaux
//    1 h (cache serveur d'1 h), radar 1 h (spec owner). ─────────────────────
let cycleFin: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void chargerCalendrier()
  void chargerCreneaux()
  arreterRadar = demarrerRadar(3_600_000)
  horloge = setInterval(() => { maintenant.value = Date.now() }, 20_000)
  cycleFin = setInterval(() => {
    void chargerCalendrier()
    void chargerCreneaux()
  }, 1_800_000)
})
onUnmounted(() => {
  if (cycleFin !== null) clearInterval(cycleFin)
  if (horloge !== null) clearInterval(horloge)
  if (arreterRadar) arreterRadar()
})
</script>

<style scoped>
@keyframes macro-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.45; }
}
/* Badges Korry du bandeau (même langage que les anciennes lampes système) :
   normaux = discrets, allumés ambre quand il y a du neuf. Pleine largeur
   (owner 26/09) : flex-1, polices agrandies. */
.korry {
  display: flex;
  justify-content: center;
  flex-direction: column;
  gap: 2px;
  padding: 5px 12px 6px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.03);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}
.korry:hover { border-color: rgba(255, 255, 255, 0.3); }
.korry.lit-rouge {
  background: rgba(239, 68, 68, 0.18);
  border-color: rgba(248, 113, 113, 0.7);
  animation: macro-pulse 1.1s ease-in-out infinite;
}
.korry.lit-rouge .k-label { color: #fecaca; }
.korry.lit-ambre {
  background: rgba(245, 158, 11, 0.14);
  border-color: rgba(251, 191, 36, 0.55);
}
.k-label {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  text-align: center;
  color: rgba(255, 255, 255, 0.55);
  font-family: ui-monospace, monospace;
  white-space: nowrap;
}
.k-info { color: rgba(255, 255, 255, 0.25); letter-spacing: 0; }
.k-fenetre {
  font-size: 11px;
  font-weight: 700;
  text-align: center;
  font-family: ui-monospace, monospace;
  background: #020409;
  border-radius: 3px;
  padding: 2px 4px;
  color: rgba(52, 211, 153, 0.78);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Fenêtre multi-lignes (badge radar : les 5 entrées tiennent sur 2 lignes
   plutôt que de réduire la police ou tronquer) — APRÈS .k-fenetre :
   même spécificité, l'ordre décide (leçon récurrente du CSS scopé). */
.k-fenetre.ko { color: rgba(248, 113, 113, 0.85); }
.k-fenetre-multi {
  white-space: normal;
  overflow: visible;
  text-overflow: clip;
  line-height: 1.35;
}
</style>
