<template>
  <!-- COLONNE-INSTRUMENT v3 (redesign cockpit 26/09, phase 1 — squelette) :
       GAUCHE le radar rond (stratégies à scanner) puis la jauge, plaque-nom
       cliquable → page définition (sous la jauge) ; DROITE la colonne de
       blocs de commande gravés (PARAMÈTRES, LABO, + carte Assets selon la
       stratégie) — contenus vivants en phases 2-3 ; BAS l'écran courbe
       pleine largeur. Clic carte → page de la stratégie. La pastille
       Telegram reste au coin. Un seul point de décision : surAction. -->
  <div
    class="relative flex flex-col gap-1.5 min-w-0 rounded-xl border p-2 cursor-pointer transition-all hover:brightness-110 hover:shadow-[0_0_18px_rgba(255,255,255,0.06)]"
    :class="teinte"
    :title="`Ouvrir la page ${nom}`"
    @click="router.push(route)"
  >

    <!-- Pastille Telegram : commande du canal, coin de la carte (owner 25/09) -->
    <PopoverInfo titre="Telegram — messages d'imminence" :texte="titreTelegram">
      <button
        class="absolute -top-2 -right-2 z-10 w-[28px] h-[28px] rounded-full flex items-center justify-center border-2 shadow-lg transition-all hover:scale-110 disabled:opacity-50"
        :class="notifications
          ? 'bg-emerald-500 border-emerald-200/50 shadow-emerald-500/40'
          : 'bg-red-500 border-red-200/50 shadow-red-500/40'"
        :disabled="bascule"
        :aria-label="notifications ? 'Telegram activé — cliquer pour couper' : 'Telegram coupé — cliquer pour activer'"
        @click.stop="basculerTelegram"
      >
        <svg viewBox="0 0 24 24" class="w-[16px] h-[16px]" aria-hidden="true">
          <path fill="#fff" d="M9.04 15.51l-.38 5.36c.54 0 .78-.23 1.06-.5l2.55-2.44 5.28 3.87c.97.53 1.66.25 1.92-.9L23.9 4.6c.31-1.42-.5-1.98-1.45-1.63L2.7 10.3c-1.39.54-1.37 1.32-.24 1.67l5.05 1.57L19.5 6.2c.55-.36 1.05-.16.64.2z" />
        </svg>
      </button>
    </PopoverInfo>

    <!-- Rang instrument : [radar + jauge] | colonne de blocs -->
    <div class="flex gap-2 flex-1 min-h-0">

      <!-- Zone gauche -->
      <div class="flex items-start gap-1 shrink-0">
        <!-- Radar ANIMÉ (phase 4, owner 26/09) : aiguille qui balaye dans le
             sens horaire + blips qui apparaissent/disparaissent — purement
             décoratif. Clic → page Scanner. -->
        <PopoverInfo v-if="avecScanner" titre="Scanner" :texte="id === 'straddle'
    ? 'Annonces économiques à venir — les déclencheurs des passes straddle (10 prochains jours).'
    : 'Setups en formation et journal — page Scanner de la stratégie.'">
          <button
            class="w-11 h-11 rounded-full border border-white/15 bg-black/40 flex items-center justify-center shadow-[inset_0_1px_0_rgba(255,255,255,0.05)] transition-all hover:scale-110 cursor-pointer shrink-0 mt-2 overflow-hidden"
            :class="teinteBloc"
            aria-label="Ouvrir le scanner"
            @click.stop="surAction('scanner')"
          >
            <svg viewBox="0 0 40 40" class="w-8 h-8" aria-hidden="true">
              <!-- Grille statique -->
              <circle cx="20" cy="20" r="16" fill="none" stroke="rgba(52,211,153,0.35)" stroke-width="1.5" />
              <circle cx="20" cy="20" r="10" fill="none" stroke="rgba(52,211,153,0.25)" stroke-width="1" />
              <circle cx="20" cy="20" r="4" fill="none" stroke="rgba(52,211,153,0.2)" stroke-width="1" />
              <line x1="20" y1="4" x2="20" y2="36" stroke="rgba(52,211,153,0.2)" stroke-width="1" />
              <line x1="4" y1="20" x2="36" y2="20" stroke="rgba(52,211,153,0.2)" stroke-width="1" />
              <!-- Balayage rotatif : secteur + aiguille, rotation CSS 3 s -->
              <g class="radar-sweep">
                <path d="M20 20 L20 4 A16 16 0 0 1 31.3 8.7 Z" fill="rgba(52,211,153,0.18)" />
                <line x1="20" y1="20" x2="20" y2="5" stroke="#34d399" stroke-width="1.5" stroke-linecap="round" />
              </g>
              <!-- Blips : apparaissent quand le balayage passe dessus -->
              <circle class="radar-blip radar-blip-1" cx="27" cy="14" r="1.6" fill="#34d399" />
              <circle class="radar-blip radar-blip-2" cx="14" cy="26" r="1.3" fill="rgba(52,211,153,0.7)" />
              <circle class="radar-blip radar-blip-3" cx="25" cy="27" r="1.1" fill="rgba(52,211,153,0.5)" />
              <circle class="radar-blip radar-blip-4" cx="10" cy="15" r="1.2" fill="rgba(52,211,153,0.6)" />
            </svg>
          </button>
        </PopoverInfo>

        <!-- Jauge + plaque-nom cliquable (→ définition) -->
        <JaugeStrategie :id="id" :nom="nom" :icone="icone" class="min-w-0" @definition="surAction('definition')" />
      </div>

      <!-- Zone droite : blocs de commande — libellé : valeur, ligne par
           ligne (owner 26/09 : « je dois savoir à quoi ils correspondent ») -->
      <div class="flex-1 min-w-0 flex flex-col gap-2">
        <BlocCommande titre="Paramètres" :teinte="teinteBloc" @clic="surAction('parametres')">
          <div class="flex items-center justify-center gap-1.5 text-[10px] leading-snug">
            <span class="font-bold text-white/70">Capital</span>
            <span class="font-bold tabular-nums" :class="capital >= capitalDepart ? 'text-emerald-300' : 'text-red-300'">{{ capital.toLocaleString('fr-FR', { maximumFractionDigits: 0 }) }} $</span>
          </div>
          <div class="flex items-center justify-center gap-1.5 text-[10px] leading-snug">
            <span class="font-bold text-white/70">Risque</span>
            <span class="font-bold text-white tabular-nums">{{ risque }} %</span>
          </div>
        </BlocCommande>
        <BlocCommande titre="Labo" :teinte="teinteBloc" @clic="surAction('simulation')">
          <div class="flex items-center justify-center gap-1.5 text-[10px] leading-snug">
            <span class="font-bold text-white/70">Σ R</span>
            <span class="font-bold tabular-nums" :class="sommeR >= 0 ? 'text-emerald-300' : 'text-red-300'">
              {{ (sommeR >= 0 ? '+' : '−') + Math.abs(sommeR).toFixed(1).replace('.', ',') }} R</span>
          </div>
          <div class="flex items-center justify-center gap-1.5 text-[10px] leading-snug">
            <span class="font-bold text-white/70">WR</span>
            <span class="font-bold text-white tabular-nums">{{ tauxReussite.toFixed(0) }} %</span>
          </div>
        </BlocCommande>
        <BlocCommande v-if="carteAsset" :titre="carteAsset.titre" :teinte="teinteBloc" @clic="surAction(carteAsset.cle)">
          <p class="text-[10px] font-bold text-white text-center leading-snug">{{ resumeAssets }}</p>
        </BlocCommande>
      </div>

    </div>

    <!-- L'écran : l'histoire en $, toute la largeur -->
    <EcranCourbe :id="id" />

    <!-- Modales de réglages : portées par LA CARTE, centrées écran par
         ModaleCadre, au-dessus du bloc STRATÉGIES (phase 0). -->
    <SmcReglagesModales :ouverte="id === 'SMC' ? modaleSmc : null" @fermer="modaleSmc = null" />
    <StraddleReglagesModales :ouverte="id === 'straddle' ? modaleStraddle : null" @fermer="modaleStraddle = null" />
    <RocketsReglagesModales :ouverte="id === 'rockets' ? modaleRockets : null" @fermer="modaleRockets = null" />
    <KdjReglagesModales :ouverte="id === 'kdj_halftrend' ? modaleKdj : null" @fermer="modaleKdj = null" />

    <!-- Calendrier économique (radar Straddle — les annonces sont ses
         déclencheurs) : même modale que le badge CALENDRIER du bandeau. -->
    <ModaleCadre v-if="ouverteCalendrier" titre="📅 Annonces économiques — 10 prochains jours" large @fermer="ouverteCalendrier = false">
      <EconomicCalendar :jours="10" />
    </ModaleCadre>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import PopoverInfo from './PopoverInfo.vue'
import BlocCommande from './BlocCommande.vue'
import JaugeStrategie from './JaugeStrategie.vue'
import EcranCourbe from './EcranCourbe.vue'
import SmcReglagesModales, { type ModaleSmc } from './SmcReglagesModales.vue'
import StraddleReglagesModales, { type ModaleStraddle } from './StraddleReglagesModales.vue'
import RocketsReglagesModales, { type ModaleRockets } from './RocketsReglagesModales.vue'
import KdjReglagesModales, { type ModaleKdj } from './KdjReglagesModales.vue'
import ModaleCadre from './ModaleCadre.vue'
import EconomicCalendar from './EconomicCalendar.vue'
import { useAlerteStore } from '@/stores/alerte.store'
import { chargerAnalyse } from '@/composables/useAnalyses'

const props = defineProps<{
  id: string
  nom: string
  icone: string
  route: string
  teinte: string
}>()

const router = useRouter()
const alerteStore = useAlerteStore()

// ── Métier de la carte (même modèle pour toute stratégie future) ────────────
/// Stratégies disposant d'une page Scanner (radar rond à gauche).
const AVEC_SCANNER = new Set(['SMC', 'rockets', 'kdj_halftrend', 'straddle'])
const avecScanner = computed(() => AVEC_SCANNER.has(props.id))

/// Carte Assets spécifique (droite, sous Labo) : titre + clé d'action.
const CARTES_ASSET: Record<string, { titre: string; cle: string }> = {
  SMC: { titre: 'Assets & TF', cle: 'timeframes' },
  straddle: { titre: 'Assets & créneaux', cle: 'perimetre' },
  kdj_halftrend: { titre: 'Assets', cle: 'assets' },
}
const carteAsset = computed(() => CARTES_ASSET[props.id] ?? null)

/// Rétro-éclairage des blocs et du radar = teinte de la stratégie.
const TEINTES_BLOC: Record<string, string> = {
  SMC: 'hover:bg-blue-500/15 hover:border-blue-400/50',
  straddle: 'hover:bg-amber-500/15 hover:border-amber-400/50',
  rockets: 'hover:bg-orange-500/15 hover:border-orange-400/50',
  kdj_halftrend: 'hover:bg-cyan-500/15 hover:border-cyan-400/50',
}
const teinteBloc = computed(() => TEINTES_BLOC[props.id] ?? 'hover:bg-white/10 hover:border-white/30')

// ── Modales de réglages (portées par la carte, phase 0) ─────────────────────
const modaleSmc = ref<ModaleSmc | null>(null)
const modaleStraddle = ref<ModaleStraddle | null>(null)
const modaleRockets = ref<ModaleRockets | null>(null)
const modaleKdj = ref<ModaleKdj | null>(null)
const ouverteCalendrier = ref(false)

const ROUTES_DEFINITION: Record<string, string> = {
  SMC: '/smc/definition',
  straddle: '/straddle/definition',
  rockets: '/rockets/definition',
  kdj_halftrend: '/kdj/definition',
}
const ROUTES_SCANNER: Record<string, string> = {
  SMC: '/smc/scanner',
  rockets: '/rockets/scanner',
  kdj_halftrend: '/kdj/scanner',
}

/// Clé d'action d'un élément de la carte → page ou modale. Un seul point de
/// décision (plaque = définition, radar = scanner, blocs = paramètres/labo/
/// assets).
function surAction(cle: string) {
  if (cle === 'simulation') { router.push(`/simulation?strategie=${props.id}&verrouille=1`); return }
  if (cle === 'definition') { router.push(ROUTES_DEFINITION[props.id] ?? '/'); return }
  if (cle === 'scanner') {
    // Straddle n'a pas de page scanner : son radar ouvre le calendrier
    // économique (les annonces sont SES déclencheurs).
    if (props.id === 'straddle') { ouverteCalendrier.value = true; return }
    const cible = ROUTES_SCANNER[props.id]
    if (cible) router.push(cible)
    return
  }
  if (props.id === 'SMC') modaleSmc.value = cle as ModaleSmc
  else if (props.id === 'straddle') modaleStraddle.value = cle as ModaleStraddle
  else if (props.id === 'rockets') modaleRockets.value = cle as ModaleRockets
  else if (props.id === 'kdj_halftrend') modaleKdj.value = cle as ModaleKdj
}

// ── Pastille Telegram (drapeau du registre, PUT partiel) ────────────────────
const etat = ref('—')
const notifications = ref(false)
const bascule = ref(false)

/// Info-bulle de la pastille : la règle d'envoi complète — le drapeau ET
/// l'état (Observation = silencieux, décision 15/09 ; le clic pré-règle
/// le drapeau).
const titreTelegram = computed(() => [
  `État : ${notifications.value ? 'ACTIVÉS' : 'COUPÉS'} (clic pour ${notifications.value ? 'couper' : 'activer'}).`,
  "Condition complète d'envoi : réglage activé ET stratégie Officielle.",
  ...(etat.value !== 'Officielle' ? [`Ici état ${etat.value} → silencieux tant que la stratégie ne repasse pas Officielle.`] : []),
].join('\n'))

// L'envoi relit le drapeau à CHAQUE signal : effet immédiat, sans relance.
async function basculerTelegram() {
  bascule.value = true
  try {
    const res = await http.put<{ notifications: boolean }>(`/api/strategies/${props.id}`, {
      notifications: !notifications.value,
    })
    notifications.value = res.data.notifications
  } catch (e) {
    alerteStore.afficherErreur(`Telegram ${props.nom} : bascule échouée — ${(e as Error).message}`)
  }
  bascule.value = false
}

async function chargerRegistre() {
  try {
    const r = await http.get('/api/strategies')
    const s = (r.data as { id: string; etat: string; notifications: boolean; capital: number; risque_pct: number }[]).find(x => x.id === props.id)
    if (s) { etat.value = s.etat; notifications.value = s.notifications; capital.value = s.capital; risque.value = s.risque_pct }
  } catch { /* silencieux */ }
}

// ── Données vivantes des blocs (Paramètres + Labo + Assets) ────────────────
const capital = ref(0)
const capitalDepart = ref(1000)
const risque = ref(0)
const sommeR = ref(0)
const tauxReussite = ref(0)
const resumeAssets = ref('…')

async function chargerBlocs() {
  try {
    const a = await chargerAnalyse(props.id)
    if (a) {
      sommeR.value = a.r_distance_total ?? 0
      tauxReussite.value = (a.taux_reussite ?? 0) * 100
    }
  } catch { /* silencieux */ }
  try {
    const r = await http.get(`/api/strategies/${props.id}/capital`)
    capital.value = r.data?.capital_actuel ?? 0
    capitalDepart.value = r.data?.capital_depart ?? 1000
  } catch { /* silencieux */ }
  await chargerAssets()
}

/// Résumé des assets armés — endpoint propre à chaque stratégie.
async function chargerAssets() {
  try {
    if (props.id === 'SMC') {
      // Couples asset × TF : « 26 assets · 78 couples »
      const r = await http.get('/api/smc/couples')
      const d = r.data as { armes: Record<string, string[]> }
      // Ne compter que les couples réellement ARMÉS (TF non vide) — un
      // asset au tableau vide est désarmé, pas « armé à 0 couples ».
      const armes = Object.entries(d.armes ?? {}).filter(([, tfs]) => tfs.length > 0)
      const couples = armes.reduce((n, [, tfs]) => n + tfs.length, 0)
      resumeAssets.value = armes.length ? `${armes.length} assets · ${couples} couples` : 'aucun'
    } else if (props.id === 'straddle') {
      // Périmètre M1 : « 9 assets »
      const r = await http.get('/api/straddle/perimetre')
      const d = r.data as { assets: string[] }
      resumeAssets.value = d.assets?.length ? `${d.assets.length} assets` : 'aucun'
    } else if (props.id === 'kdj_halftrend') {
      // KDJ H1 : « 4 armés » (liste vide = tous)
      const r = await http.get('/api/config', { params: { cle: 'kdj_assets_armes' } })
      const liste = JSON.parse(r.data?.valeur ?? '[]') as string[]
      resumeAssets.value = liste.length === 0 ? 'tous' : `${liste.length} armés`
    }
  } catch { resumeAssets.value = '—' }
}

let pollBlocs: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void chargerRegistre()
  void chargerBlocs()
  pollBlocs = setInterval(chargerBlocs, 60_000)
})
onUnmounted(() => { if (pollBlocs !== null) clearInterval(pollBlocs) })
</script>

<style scoped>
/* ── Radar animé (phase 4, owner 26/09) ─────────────────────────────────── */

/* Aiguille + secteur : rotation continue dans le sens des aiguilles d'une
   montre, 3 s par tour — un vrai balayage de radar. */
.radar-sweep {
  transform-origin: 20px 20px;
  animation: radar-rotate 3s linear infinite;
}

@keyframes radar-rotate {
  from { transform: rotate(0deg); }
  to   { transform: rotate(360deg); }
}

/* Blips : apparaissent brièvement quand le balayage passe dessus, puis
   s'estompent — chaque blip a un délai différent pour un effet vivant. */
.radar-blip {
  opacity: 0;
  animation: radar-blip-pulse 3s ease-out infinite;
}
.radar-blip-1 { animation-delay: 0s; }
.radar-blip-2 { animation-delay: 0.8s; }
.radar-blip-3 { animation-delay: 1.5s; }
.radar-blip-4 { animation-delay: 2.2s; }

@keyframes radar-blip-pulse {
  0%       { opacity: 0; }
  5%       { opacity: 1; }
  40%      { opacity: 0.6; }
  70%,100% { opacity: 0; }
}
</style>
