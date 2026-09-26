<template>
  <!-- Interrupteurs Korry du pedestal COMMS & NAVIGATION (dessin owner
       25/09) : chaque accès est un rectangle gravé — icône + libellé
       gravé, rétro-éclairage teinté au survol, appuyé au clic. Les tuiles
       PRESSE et GRAPHIQUES portent une fenêtre digitale vivante (titres
       frais, alertes actives) — même contenu qu'avant, habillage métal.
       `ids` sélectionne les tuiles rendues (et donc les données
       chargées — pas de requête pour une tuile absente). -->
  <div class="flex flex-col gap-2 shrink-0">
    <div
      v-for="t in tuilesAffichees"
      :key="t.id"
      class="rounded-lg border p-2 flex flex-col gap-1.5 min-h-[168px] flex-1 shrink-0 cursor-pointer transition-all bg-white/[0.03] border-white/10 shadow-[inset_0_1px_0_rgba(255,255,255,0.05)] hover:shadow-[0_0_12px_rgba(255,255,255,0.08)]"
      :class="TEINTES[t.id]"
      @click="router.push(t.route)"
    >
      <div class="flex items-center gap-1.5">
        <span class="text-[15px] leading-none">{{ t.icone }}</span>
        <span class="text-[11px] font-extrabold uppercase tracking-[0.15em] text-white/75 truncate">{{ t.label }}</span>
      </div>

      <!-- 📰 Presse : 12 derniers titres (owner 25/09), en fenêtre digitale
           sur toile de fond journal -->
      <template v-if="t.id === 'presse'">
        <div class="relative rounded bg-black/30 px-1.5 py-1 min-h-0 flex-1 overflow-y-auto">
          <FondTheme theme="presse" />
          <div class="relative flex flex-col gap-0.5 h-full">
            <div v-if="!articles.length" class="text-[14px] text-white/70 leading-snug">Bibliothèque vide — le collecteur remplit au prochain cycle (30 min)</div>
            <div v-for="a in articles" :key="a.hash_titre" class="flex items-center gap-1.5 min-w-0">
              <span class="text-[14px] text-blue-400 font-mono shrink-0">{{ ageCourt(a) }}</span>
              <span class="text-[14px] text-white/90 truncate cursor-help" :title="a.resume_source || a.titre">{{ a.titre_fr || a.titre }}</span>
            </div>
          </div>
        </div>
      </template>

      <!-- 📈 Graphiques : alertes armées (format owner 25/09 : date de
           pose - asset - prix - boutons ; texte centré de marche à suivre
           quand vide), en fenêtre digitale sur toile courbe de prix. -->
      <template v-else-if="t.id === 'graphiques'">
        <div class="relative rounded bg-black/30 px-1.5 py-1 min-h-0 flex-1 overflow-hidden">
          <FondTheme theme="graphiques" />
          <div class="relative flex flex-col gap-1 h-full" :class="alertesActives.length ? '' : 'items-center justify-center text-center'">
            <div v-for="a in alertesActives" :key="a.id" class="flex items-center gap-1.5 text-[14px] min-w-0" :title="titreAlerte(a)">
              <span class="font-mono text-white/80 shrink-0">{{ datePose(a.cree_le) }}</span>
              <span class="font-semibold text-white truncate">{{ a.asset }}</span>
              <span class="font-mono text-amber-300">{{ formaterPrix(a.prix) }}</span>
              <button class="ml-auto text-[13px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/60 text-white transition-colors shrink-0" title="Ouvrir le graphique de cet asset" @click.stop="ouvrirGraphique(a.asset)">👁</button>
              <button class="text-[13px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-red-600/60 text-white transition-colors shrink-0" title="Supprimer l'alerte (bloc + graphique)" @click.stop="supprimerAlerte(a)">✕</button>
            </div>
            <div v-if="!alertesActives.length" class="text-[14px] text-white/70 leading-snug">
              Aucune alerte armée.<br>
              Allez sur la page Graphiques<br>
              → clic droit sur un prix<br>
              → pour poser une alarme
            </div>
          </div>
        </div>
      </template>

      <!-- 🧠 IA : productions du jour (textes owner 25/09), sur toile de
           fond cerveau -->
      <template v-else-if="t.id === 'ia'">
        <div class="relative rounded bg-black/30 px-1.5 py-1 min-h-0 flex-1 overflow-hidden">
          <FondTheme theme="ia" />
          <div class="relative flex flex-col gap-1 text-[14px] leading-snug">
            <p class="text-white">Conviction du jour : <span class="font-bold text-violet-200">{{ convictionJour.n }} signal{{ convictionJour.n > 1 ? 'aux' : '' }} noté{{ convictionJour.n > 1 ? 's' : '' }}</span></p>
            <p v-if="convictionJour.mediane !== null" class="text-white/85">
              Conviction /100 : <span class="font-bold text-violet-200">médiane {{ convictionJour.mediane }}</span>
              <span class="text-white/50"> · de {{ convictionJour.min }} à {{ convictionJour.max }}</span>
            </p>
            <p class="text-white/85">Travaux IA : <span class="font-bold text-violet-200">{{ appelsIA ?? '—' }}</span> depuis la relance de l'app</p>
            <p class="text-white/85">
              IA locale (Ollama) :
              <span :class="ollamaOk === false ? 'text-red-400 font-bold' : 'text-emerald-400 font-bold'">{{ ollamaOk === false ? 'hors ligne' : 'en ligne' }}</span>
            </p>
          </div>
        </div>
      </template>

      <!-- 📦 Données : la soute — symboles suivis et bougies du jour PAR
           SOURCE (textes owner 25/09), sur toile de fond base de données -->
      <template v-else-if="t.id === 'systeme'">
        <div class="relative rounded bg-black/30 px-1.5 py-1 min-h-0 flex-1 overflow-hidden">
          <FondTheme theme="donnees" />
          <div class="relative flex flex-col gap-1 text-[13px] leading-snug">
            <p class="text-white">Bybit : <span class="font-bold text-rose-200">{{ donnees.bybitSymboles }} symboles suivis.</span> <span class="font-bold text-rose-200">{{ donnees.bybit }}</span> aujourd'hui</p>
            <p class="text-white">EA Axi : <span class="font-bold text-rose-200">{{ donnees.eaSymboles }} symboles suivis.</span> <span class="font-bold text-rose-200">{{ donnees.ea }}</span> aujourd'hui</p>
            <p class="text-white">Actions US : <span class="font-bold text-rose-200">{{ donnees.actionsTotal }} actions suivies.</span> <span class="font-bold text-rose-200">{{ donnees.actionsAJour }}</span> à jour</p>
            <p class="text-white">Presse : <span class="font-bold text-rose-200">{{ donnees.presseTotal }}</span> articles notés en base. <span class="font-bold text-rose-200">{{ donnees.presse24h }}</span> ces dernières 24h</p>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import { presseApi, type ArticlePresse } from '@/services/api.presse'
import { alertesApi } from '@/services/api.alertes'
import { useAlerteStore } from '@/stores/alerte.store'
import { ciblerPremierSlot } from '@/utils/graphiques'
import type { AlertePrix } from '@/services/api.alertes'
import FondTheme from './FondTheme.vue'

/// Tuiles rendues par cette instance — le dashboard les place en deux
/// groupes depuis le 14/09 (presse entre calendrier et rapport, puis
/// graphiques/IA/données). Défaut : les quatre (comportement d'origine).
const props = withDefaults(defineProps<{ ids?: string[] }>(), {
  ids: () => ['presse', 'graphiques', 'ia', 'systeme'],
})
const affiche = (id: string) => props.ids.includes(id)

const router = useRouter()
const alerteStore = useAlerteStore()

/// Teinte de chaque interrupteur : rétro-éclairage au survol — la couleur
/// voyage jusqu'à la page ouverte.
const TEINTES: Record<string, string> = {
  presse: 'hover:bg-sky-500/15 hover:border-sky-400/50',
  graphiques: 'hover:bg-emerald-500/15 hover:border-emerald-400/50',
  ia: 'hover:bg-violet-500/15 hover:border-violet-400/50',
  systeme: 'hover:bg-rose-500/15 hover:border-rose-400/50',
}

const tuiles = [
  { id: 'presse', icone: '📰', label: 'Revue de presse', route: '/presse' },
  { id: 'graphiques', icone: '📈', label: 'Graphiques', route: '/smc/graphiques' },
  { id: 'ia', icone: '🧠', label: 'Fonctionnalités IA', route: '/ia' },
  { id: 'systeme', icone: '📦', label: 'Données', route: '/donnees' },
] as const

const tuilesAffichees = computed(() => tuiles.filter(t => affiche(t.id)))

// ── Presse : 3 derniers articles ─────────────────────────────────────────────
const articles = ref<ArticlePresse[]>([])

/// Âge court d'un article : « 4min », « 2h », « 3j ».
function ageCourt(a: ArticlePresse): string {
  const ts = Date.parse(a.publie_le) / 1000
  return Number.isFinite(ts) ? ageTs(ts) : ''
}

function ageTs(ts: number): string {
  const s = Math.max(0, Date.now() / 1000 - ts)
  if (s < 3600) return `${Math.max(1, Math.floor(s / 60))}mn`
  if (s < 86400) return `${Math.floor(s / 3600)}h`
  return `${Math.floor(s / 86400)}j`
}

// ── Graphiques : alertes prix actives (l'ancienne liste des slots de la
//    grille est retirée le 14/09 — décision propriétaire) ────────────────────
const alertesActives = ref<AlertePrix[]>([])

/// Ouvre la page Graphiques sur un asset précis (premier slot ciblé).
function ouvrirGraphique(asset: string, timeframe?: string) {
  ciblerPremierSlot(asset, timeframe)
  router.push('/smc/graphiques')
}

function titreAlerte(a: AlertePrix): string {
  return `${a.asset} — ${a.sens === 'en_dessous' ? 'descente sous' : 'montée au-dessus de'} ${formaterPrix(a.prix)}${a.note ? ` · ${a.note}` : ''}`
}

async function supprimerAlerte(a: AlertePrix) {
  try {
    await alertesApi.supprimer(a.id)
    alertesActives.value = alertesActives.value.filter(x => x.id !== a.id)
  } catch (e) {
    alerteStore.afficherErreur(`Suppression alerte : ${(e as Error).message}`)
  }
}

function formaterPrix(p: number | null): string {
  if (p === null) return '—'
  if (p >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(p)
  if (p >= 1) return p.toFixed(2)
  return p.toFixed(4)
}

// ── IA : plus d'aperçu (14/09) — la ligne « Modèle » est retirée, les
//    raccourcis vivent en onglets dans la page Fonctionnalités IA ────────────

// ── IA : notations conviction du jour + activité LLM ────────────────────────
interface SignalBref { llm_conviction?: number | null; cree_le?: number }
const convictionJour = ref<{ n: number; mediane: number | null; min: number | null; max: number | null }>({ n: 0, mediane: null, min: null, max: null })
const appelsIA = ref<number | null>(null)
const ollamaOk = ref<boolean | null>(null)

// ── Données : volumétrie des collections (la soute) ─────────────────────────
const donnees = ref<{
  bybitSymboles: number; bybit: string
  eaSymboles: number; ea: string
  actionsTotal: number; actionsAJour: number
  presseTotal: string; presse24h: number
}>({ bybitSymboles: 0, bybit: '…', eaSymboles: 0, ea: '…', actionsTotal: 0, actionsAJour: 0, presseTotal: '…', presse24h: 0 })

/// « +8,6k bougies » façon compteur du jour.
function compactJour(n: number): string {
  return n >= 1000 ? `+${(n / 1000).toFixed(1).replace('.', ',')}k bougies` : `+${n} bougie${n > 1 ? 's' : ''}`
}

/// Date de pose d'une alerte : « 25/09 14:32 ».
function datePose(ts: number): string {
  const d = new Date(ts * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getDate())}/${p(d.getMonth() + 1)} ${p(d.getHours())}:${p(d.getMinutes())}`
}

async function chargerTout() {
  // Chaque instance ne charge que les données des tuiles qu'elle rend.
  if (affiche('presse')) {
    try {
      const liste = await presseApi.articles({ page: 1 })
      articles.value = [...liste]
        .sort((a, b) => Date.parse(b.publie_le) - Date.parse(a.publie_le))
        .slice(0, 4)
    } catch { articles.value = [] }
  }

  if (affiche('graphiques')) {
    try {
      const alertes = await alertesApi.lister()
      alertesActives.value = alertes.filter(a => a.active)
      // Rattrapage (hérité de l'ancien bloc 🔔) : une alerte déclenchée ne
      // doit exister nulle part. Les graphs la suppriment en notifiant
      // (son + OS, poll 10 s) ; on nettoie ici les déclenchées de plus de
      // 2 minutes, fenêtre laissée aux charts pour la notification.
      const vieilles = alertes.filter(
        a => !a.active && a.declenchee_le && Date.now() / 1000 - a.declenchee_le > 120,
      )
      if (vieilles.length) {
        await Promise.all(vieilles.map(a => alertesApi.supprimer(a.id).catch(() => null)))
      }
    } catch { alertesActives.value = [] }
  }

  if (affiche('ia')) {
    try {
      const r = await http.get('/api/signaux', { params: { limit: 150 } })
      const debut = new Date(); debut.setHours(0, 0, 0, 0)
      const notes = (r.data as SignalBref[])
        .filter(s => s.llm_conviction != null && (s.cree_le ?? 0) >= debut.getTime() / 1000)
        .map(s => s.llm_conviction as number)
        .sort((a, b) => a - b)
      convictionJour.value = notes.length
        ? { n: notes.length, mediane: notes[Math.floor(notes.length / 2)], min: notes[0], max: notes[notes.length - 1] }
        : { n: 0, mediane: null, min: null, max: null }
    } catch { convictionJour.value = { n: 0, mediane: null, min: null, max: null } }
    try {
      const r = await http.get('/api/ia/status')
      appelsIA.value = r.data?.appels_jour ?? null
      ollamaOk.value = r.data?.ollama_disponible ?? null
    } catch { appelsIA.value = null; ollamaOk.value = null }
  }

  if (affiche('systeme')) {
    // Ventilation du flux du jour PAR SOURCE (le total global mélange
    // Bybit/MT5/actions — il ne veut rien dire par étiquette).
    interface LigneSource { source: string; bougies: number; symboles: number }
    try {
      const r = await http.get('/api/data/coverage')
      const parSource: LigneSource[] = r.data?.bougies_par_source ?? []
      const bybit = parSource.find(s => s.source === 'bybit_ws')
      const mt5 = parSource.find(s => s.source === 'mt5')
      donnees.value.bybitSymboles = bybit?.symboles ?? 0
      donnees.value.bybit = compactJour(bybit?.bougies ?? 0)
      donnees.value.ea = compactJour(mt5?.bougies ?? 0)
    } catch { donnees.value.bybit = '—'; donnees.value.ea = '—' }
    try {
      const r = await http.get('/api/mt5/statut')
      donnees.value.eaSymboles = (r.data?.symboles ?? []).length
    } catch { donnees.value.eaSymboles = 0 }
    try {
      const r = await http.get('/api/rockets/actions/backfill/etat')
      donnees.value.actionsTotal = r.data?.univers_total ?? 0
      donnees.value.actionsAJour = r.data?.univers_avec_bougies ?? 0
    } catch { donnees.value.actionsTotal = 0; donnees.value.actionsAJour = 0 }
    try {
      const r = await http.get('/api/presse/articles', { params: { page: 1 } })
      donnees.value.presseTotal = (r.data?.total ?? 0).toLocaleString('fr-FR')
      // 24 h comptées sur la page des plus récents (exact en dessous de
      // 50/jour — la taille de page).
      const limite = Date.now() - 86_400_000
      donnees.value.presse24h = (r.data?.articles ?? []).filter(
        (a: { publie_le: string }) => Date.parse(a.publie_le) >= limite,
      ).length
    } catch { donnees.value.presseTotal = '—'; donnees.value.presse24h = 0 }
  }
}

let poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void chargerTout()
  poll = setInterval(chargerTout, 60_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
