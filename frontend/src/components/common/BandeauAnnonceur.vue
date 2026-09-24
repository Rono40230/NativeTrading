<template>
  <!-- BANDEAU ANNONCEUR (refonte cockpit 24/09) : le panneau d'annonciation
       (Korry) d'un avion, en tête du dashboard. Dark cockpit — tout va
       bien = lampes éteintes et discrètes (compteur vert faible) ; un
       problème = la lampe s'allume (ambre = dégradé, rouge = panne).
       Deux groupes : lampes systèmes (6 Korry gravés) + alertes prix
       (master caution). Les sessions de marchés vivent dans la rangée
       SESSIONS MONDIALES (MarketClocks). -->
  <div class="glass-card px-3 py-1.5 flex items-center gap-3 flex-wrap">

    <!-- ── Groupe 1 : annunciateur systèmes (lampes Korry gravées) ─────── -->
    <div class="flex items-center gap-1.5 flex-wrap">
      <PopoverInfo texte="SERVEUR — le backend Rust de l'app répond à la sonde (toutes les 30 s). OK = il tourne.">
        <div class="korry" :class="backendOk ? '' : 'ko'">
          <span class="k-label">SERVEUR <span class="k-info">ⓘ</span></span>
          <span class="k-fenetre">{{ backendOk ? 'OK' : 'KO' }}</span>
        </div>
      </PopoverInfo>
      <PopoverInfo texte="EA MT5 — le flux de bougies M1 envoyé par l'EA Axi (MetaTrader 5) : X/Y = symboles suivis dont la dernière bougie a moins de 120 s. Ambre = silence de l'EA.">
        <div class="korry" :class="mt5Ok === null ? 'attente' : mt5Ok ? '' : 'degrade'">
          <span class="k-label">EA MT5 <span class="k-info">ⓘ</span></span>
          <span class="k-fenetre">{{ mt5Compteur }} frais</span>
        </div>
      </PopoverInfo>
      <PopoverInfo texte="BYBIT — le WebSocket temps réel crypto/métaux : bougies insérées depuis minuit (Paris). Rouge = flux coupé.">
        <div class="korry" :class="btcPrix ? '' : 'ko'">
          <span class="k-label">BYBIT <span class="k-info">ⓘ</span></span>
          <span class="k-fenetre">{{ bybitFlux }} bougies</span>
        </div>
      </PopoverInfo>
      <PopoverInfo texte="IA LOCALE — Ollama : appels LLM passés aujourd'hui (ranker rockets, catalyseur news, analyses). Rouge = hors ligne.">
        <div class="korry" :class="ollamaOk === null ? 'attente' : ollamaOk ? '' : 'ko'">
          <span class="k-label">IA LOCALE <span class="k-info">ⓘ</span></span>
          <span class="k-fenetre">{{ llmAppels }} appel{{ llmAppels > 1 ? 's' : '' }}</span>
        </div>
      </PopoverInfo>
      <PopoverInfo :texte="tiingoTitre || 'ACTIONS US — veille Tiingo : univers couvert en bougies (backfill).'">
        <div class="korry" :class="tiingoOk === null ? 'attente' : tiingoOk ? '' : 'degrade'">
          <span class="k-label">ACTIONS US <span class="k-info">ⓘ</span></span>
          <span class="k-fenetre">{{ tiingoAvancement }} univers</span>
        </div>
      </PopoverInfo>
      <PopoverInfo texte="PRESSE — le collecteur d'articles (cycle 30 min) : taille de la bibliothèque. Ambre = collecte arrêtée.">
        <div class="korry" :class="presseOk ? '' : 'degrade'">
          <span class="k-label">PRESSE <span class="k-info">ⓘ</span></span>
          <span class="k-fenetre">{{ presseCompact }} articles</span>
        </div>
      </PopoverInfo>
    </div>

    <span class="w-px self-stretch bg-white/10 hidden md:block" />

    <!-- ── Groupe 3 : alertes prix (master caution) ────────────────────── -->
    <PopoverInfo
      :titre="alertes.length ? `${alertes.length} alerte(s) prix active(s)` : 'Aucune alerte prix active'"
      :texte="alertes.length
        ? alertes.map(a => `${a.sens === 'en_dessous' ? '🔻' : '🔺'} ${a.asset} ${a.prix}`).join('\n') + '\nGestion complète dans Graphiques.'
        : 'Créer une alerte : page Graphiques → clic droit sur le prix.'"
    >
      <button
        class="voyant cursor-pointer"
        :class="alertes.length ? 'attention pulse-ambre' : 'eteint'"
        @click.stop="router.push('/smc/graphiques')"
      >{{ alertes.length ? `⚠ ${alertes.length} ALERT${alertes.length > 1 ? 'ES' : 'E'}` : '◌ ALERTES' }}</button>
    </PopoverInfo>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import PopoverInfo from './PopoverInfo.vue'
import { alertesApi, type AlertePrix } from '@/services/api.alertes'

defineProps<{
  backendOk: boolean
  btcPrix: number | null
  ollamaOk: boolean | null
}>()

const router = useRouter()

// ── Sondes systèmes (ex-DashboardSystemStatus, 30 s) ────────────────────
const mt5Ok = ref<boolean | null>(null)
const mt5Compteur = ref('0/0')
const tiingoOk = ref<boolean | null>(null)
const tiingoAvancement = ref('…')
const tiingoTitre = ref('')
const presseOk = ref(false)
const presseTotal = ref(0)
const bybitFlux = ref('…')
const llmAppels = ref(0)

const presseCompact = computed(() =>
  presseTotal.value >= 1000 ? `${(presseTotal.value / 1000).toFixed(1).replace('.', ',')}k` : String(presseTotal.value))

async function sonder() {
  try {
    const r = await http.get('/api/mt5/statut')
    mt5Ok.value = !!r.data?.connecte
    const syms: { age_s: number }[] = r.data?.symboles ?? []
    mt5Compteur.value = `${syms.filter(x => x.age_s >= 0 && x.age_s < 120).length}/${syms.length}`
  } catch { mt5Ok.value = false }
  try {
    const r = await http.get('/api/rockets/actions/backfill/etat')
    const d = r.data as { univers_avec_bougies?: number; univers_total?: number; progression_pct?: number }
    tiingoOk.value = true
    tiingoAvancement.value = `${d.univers_avec_bougies ?? 0}/${d.univers_total ?? 0}`
    tiingoTitre.value = `Veille actions — backfill ${d.progression_pct ?? 0} % de l'univers (volume réel Tiingo).`
  } catch { tiingoOk.value = false; tiingoTitre.value = 'Endpoint veille actions injoignable' }
  try {
    const r = await http.get('/api/presse/briefs')
    presseOk.value = Array.isArray(r.data) ? r.data.length > 0 : !!r.data
  } catch { presseOk.value = false }
  try {
    const r = await http.get('/api/presse/articles', { params: { page: 1 } })
    presseTotal.value = r.data?.total ?? 0
  } catch { presseTotal.value = 0 }
  try {
    const r = await http.get('/api/data/coverage')
    bybitFlux.value = `+${Math.round((r.data?.bougies_aujourd_hui ?? 0) / 1000)}k`
  } catch { bybitFlux.value = '—' }
  try {
    const r = await http.get('/api/ia/status')
    llmAppels.value = r.data?.appels_jour ?? 0
  } catch { llmAppels.value = 0 }
}

// ── Alertes prix (ex-tuile Graphiques, 60 s) ────────────────────────────
const alertes = ref<AlertePrix[]>([])

let poll: ReturnType<typeof setInterval> | null = null

async function chargerAlertes() {
  try { alertes.value = await alertesApi.lister() } catch { alertes.value = [] }
}

let pollAlertes: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  void sonder()
  poll = setInterval(sonder, 30_000)
  // Alertes : rafraîchies doucement (60 s) — le son Telegram prévient en
  // temps réel, le voyant n'est que le rappel visuel.
  void chargerAlertes()
  pollAlertes = setInterval(chargerAlertes, 60_000)
})
onUnmounted(() => {
  if (poll !== null) clearInterval(poll)
  if (pollAlertes !== null) clearInterval(pollAlertes)
})
</script>

<style scoped>
/* Lampes Korry (annunciateur) — dark cockpit : normal = éteint/discret
   (libellé gravé faible, compteur vert sombre dans sa fenêtre) ; dégradé
   = lampe allumée ambre ; panne = rouge pulsant. */
.korry {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 2px 5px 3px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.03);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}
.k-label {
  font-size: 8px;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  text-align: center;
  color: rgba(255, 255, 255, 0.5);
  font-family: ui-monospace, monospace;
  white-space: nowrap;
}
.k-info { color: rgba(255, 255, 255, 0.25); letter-spacing: 0; }
.k-fenetre {
  font-size: 9px;
  font-weight: 700;
  text-align: center;
  font-family: ui-monospace, monospace;
  background: #020409;
  border-radius: 2px;
  padding: 1px 2px;
  color: rgba(52, 211, 153, 0.75);
  white-space: nowrap;
}
.korry.degrade {
  background: rgba(245, 158, 11, 0.14);
  border-color: rgba(251, 191, 36, 0.55);
  animation: korry-pulse 1.6s ease-in-out infinite;
}
.korry.degrade .k-label { color: #fcd34d; }
.korry.degrade .k-fenetre { color: #fbbf24; }
.korry.ko {
  background: rgba(239, 68, 68, 0.18);
  border-color: rgba(248, 113, 113, 0.6);
  animation: korry-pulse 1.2s ease-in-out infinite;
}
.korry.ko .k-label { color: #fecaca; }
.korry.ko .k-fenetre { color: #f87171; }
.korry.attente .k-label { animation: korry-pulse 1.4s ease-in-out infinite; }
@keyframes korry-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.55; }
}

/* Master caution alertes (bouton) + BTC. */
.voyant {
  @apply px-1.5 py-0.5 rounded font-bold font-mono border whitespace-nowrap text-[10px]
         bg-white/[0.04] border-white/10 text-white/60;
}
.voyant.eteint  { @apply text-white/40; }
.voyant.attention { @apply text-amber-200 bg-amber-500/25 border-amber-400/60; }

.pulse-ambre { animation: korry-pulse 1.4s ease-in-out infinite; }
</style>
