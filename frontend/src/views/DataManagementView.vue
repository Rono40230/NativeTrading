<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">📦 Données — Pilotage du pipeline</h1>

    <!-- ══ SECTION 1 — Contrôle des workers ══════════════════════════════════ -->
    <div class="glass-card p-5 space-y-4">
      <h2 class="text-sm font-semibold text-white uppercase tracking-wider">Workers d'ingestion</h2>

      <!-- Interrupteurs + statut — MT5 et workers dans la même rangée -->
      <div class="grid gap-4 grid-cols-1 md:grid-cols-2">

      <!-- Collecteur MT5/Axi (statut de l'EA — pas d'interrupteur : MT5
           ouvert = collecte, MT5 fermé = silence) -->
      <div class="rounded-xl border p-4 space-y-3"
           :class="mt5.connecte ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-white/10 bg-white/[0.02]'">
        <div class="flex items-center justify-between">
          <div>
            <p class="font-bold text-white">🖥️ Collecteur MT5 / Axi</p>
            <p class="text-xs text-white">Bougies M1 de ton broker — l'EA attaché dans MT5</p>
          </div>
          <span v-if="mt5.connecte" class="text-emerald-400 text-sm">● Connecté</span>
          <span v-else class="text-white text-sm">○ Silencieux (MT5 fermé ?)</span>
        </div>
        <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs" v-if="mt5.symboles?.length">
          <span v-for="s in mt5.symboles" :key="s.asset" class="text-white">
            {{ s.asset }} :
            <span v-if="s.age_s >= 0 && s.age_s < 120" class="text-emerald-400">il y a {{ s.age_s }}s</span>
            <span v-else class="text-white">sans prix (marché fermé ?)</span>
          </span>
        </div>
        <p v-else class="text-xs text-white">Aucun actif MT5 actif</p>
      </div>

      <!-- Tiingo Actions : veille quotidienne + rattrapage manuel (§7-3a :
           seul endpoint d'action conservé — déclencher un lot sans attendre
           le cycle de 65 min, précieux après l'ajout de pionniers narratifs) -->
      <div
        class="rounded-xl border p-4 space-y-3 transition"
        :class="tiingo ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-white/10 bg-white/[0.02]'"
      >
        <div class="flex items-center justify-between">
          <div>
            <p class="font-bold text-white">Tiingo Actions</p>
            <p class="text-xs text-white">Prix D1 volume réel — univers narratif + liquidité</p>
          </div>
          <span v-if="tiingo" class="text-emerald-400 text-sm">● À jour</span>
          <span v-else class="text-red-400 text-sm">○ Erreur</span>
        </div>
        <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
          <span class="text-white">{{ tiingo?.univers_avec_bougies ?? 0 }}/{{ tiingo?.univers_total ?? 0 }} couverts (périmètre actif)</span>
          <span class="text-white">backfill {{ tiingo?.progression_pct ?? 0 }} %</span>
          <span class="text-white">Rafraîchissement quotidien + tournante 65 min</span>
        </div>
        <div class="flex items-center gap-3">
          <button
            class="text-xs px-3 py-1.5 rounded-lg bg-white/10 hover:bg-blue-600/50 text-white transition-colors disabled:opacity-50"
            :disabled="rattrapageEnCours"
            title="Exécute immédiatement un lot de backfill (50 tickers max, borné par le quota Tiingo) — utile après l'ajout de pionniers narratifs"
            @click="rattraperBackfill"
          >{{ rattrapageEnCours ? '⏳ Lot en cours…' : '⚡ Rattraper maintenant' }}</button>
          <span v-if="rattrapageResultat" class="text-xs" :class="rattrapageResultat.erreur ? 'text-red-400' : 'text-emerald-300'">{{ libelleRattrapage }}</span>
        </div>
      </div>

      </div>

    </div>

    <!-- ══ SECTION 2 — Assets du pipeline ════════════════════════════════════ -->
    <div class="glass-card p-5">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-white uppercase tracking-wider">Assets du pipeline</h2>
        <div class="flex items-center gap-3">
          <!-- Garde fraîcheur : VERT quand le pipeline est sain (visible),
               ambre + noms des assets en alerte sinon -->
          <span class="text-[11px] px-2 py-0.5 rounded-full border" :class="assetsEnAlerte.length > 0 ? 'bg-amber-500/15 border-amber-500/30 text-amber-300' : 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300'">{{ assetsEnAlerte.length > 0 ? `⚠ fraîcheur : ${assetsEnAlerte.map(e => e.asset).join(', ')}` : 'fraîcheur ✓' }}</span>
          <button
            class="px-3 py-1 rounded-lg bg-emerald-500/20 text-emerald-400 text-xs font-semibold hover:bg-emerald-500/30 transition"
            @click="modaleAjout?.ouvrirModaleAsset()"
          >
            + Ajouter un asset
          </button>
          <span class="text-xs text-white">{{ nbAssetsActifs }} / {{ tous.length }} activés</span>
        </div>
      </div>
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <div v-for="cat in CATEGORIES" :key="cat.type">
          <p class="text-[11px] font-semibold uppercase tracking-wider mb-1.5" :class="cat.couleur">
            {{ cat.label }}
          </p>
          <div class="space-y-0.5">
            <label
              v-for="a in cat.assets"
              :key="a.id"
              class="flex items-center gap-1.5 cursor-pointer rounded border px-2 py-1 transition select-none"
              :class="a.actif
                ? 'border-emerald-500/40 bg-emerald-500/10'
                : 'border-white/10 bg-white/[0.02] hover:bg-white/[0.06] hover:border-white/20'"
            >
              <input
                type="checkbox"
                class="hidden"
                :checked="a.actif"
                :disabled="enCoursAsset === a.id"
                @change="basculerAsset(a)"
              />
              <span
                class="w-2.5 h-2.5 rounded-sm border flex items-center justify-center shrink-0 transition"
                :class="a.actif ? 'bg-emerald-500 border-emerald-500' : 'border-white/30'"
              >
                <svg v-if="a.actif" class="w-2 h-2 text-white" fill="none" viewBox="0 0 12 12">
                  <path d="M10 3L5 8.5 2 5.5" stroke="white" stroke-width="1.5" stroke-linecap="round"/>
                </svg>
              </span>
              <span class="font-mono text-sm font-bold text-white truncate">{{ a.id }}</span>
              <!-- Badge fraîcheur (garde 16/10) — tooltip CSS obligatoire (title natif inopérant, écart 27) -->
              <span v-if="anomaliesAsset(a.id).length" class="relative group shrink-0">
                <span class="text-[11px] text-amber-400 font-bold">⚠</span>
                <span
                  class="pointer-events-none absolute left-1/2 -translate-x-1/2 bottom-full mb-1.5 z-30 hidden group-hover:block whitespace-nowrap rounded-md border border-amber-500/40 bg-slate-900 px-2 py-1 text-[10px] font-normal text-amber-200 shadow-xl"
                >{{ anomaliesAsset(a.id).map(libelleAnomalie).join(' · ') }}</span>
              </span>
              <span
                class="ml-auto text-[10px] px-1.5 py-0.5 rounded font-semibold shrink-0"
                :class="badgeSource(a.source).classe"
              >
                {{ badgeSource(a.source).label }}
              </span>
              <span v-if="enCoursAsset === a.id" class="text-[10px] text-white">…</span>
            </label>
          </div>
        </div>
      </div>
      <p v-if="erreurAssets" class="text-red-400 text-xs mt-2">{{ erreurAssets }}</p>
      <p class="text-white text-[11px] mt-2">
        LE PIPELINE EST LA RÉFÉRENCE (owner 15/10) : décocher retire l'asset partout (armements en cascade, caches d'analyse rafraîchis) — les données sont conservées, réactiver remonte immédiatement. Cocher rend l'asset disponible partout, ARMÉ NULLE PART.
      </p>
    </div>

    <!-- Modale d'ajout (composant dédié) -->
    <ModaleAjoutAsset ref="modaleAjout" @cree="chargerTous" />

    <!-- Confirmation de retrait : impact réel de la cascade (doctrine 15/10) -->
    <ModaleConfirmation
      :ouverte="retraitCible !== null"
      titre="Retirer cet asset du pipeline ?"
      :sous-titre="retraitCible ? `${retraitCible.asset.id} — la référence unique de l'app` : ''"
      :lignes="retraitCible ? lignesRetrait(retraitCible.impact) : []"
      :avertissement="retraitCible && retraitCible.impact.positions_ouvertes > 0
        ? `${retraitCible.impact.positions_ouvertes} position(s) ouverte(s) sur cet asset : le rattrapage les clôturera au dernier prix connu.`
        : undefined"
      label-confirmer="Retirer partout"
      variant="danger"
      icone="📦"
      :en-cours="retraitEnCours"
      @confirmer="confirmerRetrait"
      @annuler="retraitCible = null"
    />

    <!-- ══ SECTION 4 — Couverture DB ═════════════════════════════════════════ -->
    <div class="glass-card p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-semibold text-white uppercase tracking-wider">Couverture par asset × timeframe</h2>
        <div class="flex items-center gap-2">
          <span
            v-if="bougiesAujourdHui !== null"
            class="text-xs text-emerald-400 px-2 py-0.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20 tabular-nums"
            title="Bougies insérées depuis minuit (Paris) — le compteur de flux journalier"
          >
            📈 +{{ bougiesAujourdHui.toLocaleString('fr-FR') }} bougies aujourd'hui
          </span>
          <span
            v-if="tailleDbGo"
            class="text-xs text-white px-2 py-0.5 rounded-lg bg-white/5 border border-white/10 tabular-nums"
            :title="`Taille de la base de données (${tailleDbOctets?.toLocaleString('fr-FR')} octets)`"
          >
            💾 DB : {{ tailleDbGo }}
          </span>
        </div>
      </div>
      <div v-if="chargement" class="text-white text-sm animate-pulse text-center py-8">Chargement…</div>
      <div v-else-if="couverture.length === 0" class="text-white text-sm text-center py-8">
        Aucune donnée — activez les workers (l'historique arrive avec le flux).
      </div>
      <!-- 4 colonnes (gabarit « Assets du pipeline ») : ~200 lignes asset × TF
           tiennent sur un écran ; dates et fraîcheur passent en infobulle. -->
      <div v-else class="grid gap-x-4 gap-y-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-4">
        <div v-for="colonne in colonnesCouverture" :key="colonne.type">
        <p class="text-[11px] font-semibold uppercase tracking-wider mb-1" :class="colonne.couleur">{{ colonne.label }}</p>
        <table class="w-full text-xs">
          <thead>
            <tr>
              <th class="text-left px-2 py-1.5 text-white">Asset</th>
              <th class="px-1 py-1.5 text-white">TF</th>
              <th class="px-2 py-1.5 text-white text-right">Bougies</th>
              <th class="px-2 py-1.5 text-white text-right">Densité</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="ligne in colonne.lignes"
              :key="ligne.asset + ligne.timeframe"
              class="border-t border-white/5 hover:bg-white/10 transition"
              :class="ligne.groupIndex % 2 === 1 ? 'bg-white/[0.04]' : ''"
              :title="`${ligne.asset} ${ligne.timeframe} — ${ligne.count.toLocaleString('fr-FR')} bougies\ndu ${ligne.dateMin} au ${ligne.dateMax}\n${ligne.fraicheurLabel}`"
            >
              <td class="px-2 py-1.5 font-bold text-white whitespace-nowrap">{{ ligne.asset }}</td>
              <td class="px-1 py-1.5 text-white text-center">{{ ligne.timeframe }}</td>
              <td class="px-2 py-1.5 text-right text-white font-mono tabular-nums">{{ ligne.count.toLocaleString('fr-FR') }}</td>
              <td class="px-2 py-1.5 text-right">
                <span class="tabular-nums font-semibold" :class="ligne.pct >= 80 ? 'text-emerald-400' : ligne.pct >= 40 ? 'text-yellow-400' : 'text-red-400'">{{ ligne.pct }}%</span>
              </td>
            </tr>
          </tbody>
        </table>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'
import ModaleAjoutAsset from '@/components/common/ModaleAjoutAsset.vue'
import ModaleConfirmation from '@/components/common/ModaleConfirmation.vue'
import { useAlerteStore } from '@/stores/alerte.store'
import type { AssetInfo } from '@/services/api.service'
import { apiService } from '@/services/api.service'
import type { CouvertureDonnees } from '@/services/api.service'
import { useFraicheurPipeline } from '@/composables/useFraicheurPipeline'
import type { WorkerConfig } from '@/services/api.worker'
import { useAssetsStore } from '@/stores/assets.store'

const assetsStore = useAssetsStore()
const alerteStore = useAlerteStore()
const TOUS_TF = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']

// ── Section 1 : workers ───────────────────────────────────────────────────────
const configWorker = ref<WorkerConfig | null>(null)
/// Veille actions Tiingo (statut seul — collecte quotidienne non pilotable).
const tiingo = ref<{ univers_avec_bougies: number; univers_total: number; progression_pct: number } | null>(null)

/// Rattrapage manuel du backfill Tiingo (POST /api/rockets/actions/backfill).
const rattrapageEnCours = ref(false)
const rattrapageResultat = ref<Record<string, unknown> | null>(null)
const libelleRattrapage = computed(() => {
  const r = rattrapageResultat.value ?? {}
  if (r.erreur) return String(r.erreur)
  return `+${r.nouveaux ?? 0} nouveaux · ${r.rafraichis ?? 0} rafraîchis · ${r.sans_donnees ?? 0} sans données`
})
async function rattraperBackfill() {
  rattrapageEnCours.value = true
  rattrapageResultat.value = null
  try {
    const res = await http.post('/api/rockets/actions/backfill', null, { timeout: 120000 })
    rattrapageResultat.value = res.data
    await chargerTiingo()
  } catch (e) {
    rattrapageResultat.value = { erreur: 'Lot échoué (voir quota/connexion)' }
  } finally {
    rattrapageEnCours.value = false
  }
}

async function chargerConfig() {
  try {
    configWorker.value = await apiService.getWorkerConfig()
  } catch {
    // Config par défaut affichée — le PUT réécrira la config serveur.
  }
}

async function chargerTiingo() {
  try {
    const r = await http.get('/api/rockets/actions/backfill/etat')
    tiingo.value = r.data
  } catch {
    tiingo.value = null
  }
}

// ── Section 2 : assets pipeline ───────────────────────────────────────────────
const tous = ref<AssetInfo[]>([])
const enCoursAsset = ref<string | null>(null)
const erreurAssets = ref('')

const nbAssetsActifs = computed(() => tous.value.filter(a => a.actif).length)

// ── Collecteur MT5 : statut (poll 30 s) ──────────────────────────────────────
const modaleAjout = ref<InstanceType<typeof ModaleAjoutAsset> | null>(null)

/// Retrait pipeline = confirmation avec l'impact réel (désarmements en
/// cascade + positions ouvertes confiées au rattrapage) — doctrine 15/10 :
/// le pipeline est LA référence, la cascade s'exécute côté backend.
interface ImpactRetrait {
  positions_ouvertes: number
  smc_couples: string[]
  kdj_arme: boolean
  straddle: boolean
}
const retraitCible = ref<{ asset: AssetInfo; impact: ImpactRetrait } | null>(null)
const retraitEnCours = ref(false)

async function demanderRetrait(a: AssetInfo) {
  try {
    const r = await http.get<ImpactRetrait>(`/api/assets/${a.id}/impact-retrait`)
    retraitCible.value = { asset: a, impact: r.data }
  } catch (e) {
    alerteStore.afficherErreur(`Impact du retrait ${a.id} : ${(e as Error).message}`)
  }
}

function lignesRetrait(i: ImpactRetrait): string[] {
  const l: string[] = []
  if (i.smc_couples.length) l.push(`SMC : ${i.smc_couples.length} couple(s) désarmé(s) (${i.smc_couples.join(', ')})`)
  if (i.kdj_arme) l.push('KDJ : retiré de la liste des assets armés')
  if (i.straddle) l.push('Straddle : retiré du périmètre — ses cases de matrice sont supprimées')
  if (!l.length) l.push("Aucun armement concerné — asset d'observation")
  return l
}

async function confirmerRetrait() {
  const cible = retraitCible.value
  if (!cible) return
  retraitEnCours.value = true
  try {
    await apiService.supprimerAsset(cible.asset.id)
    cible.asset.actif = false
    retraitCible.value = null
    await chargerTous()
    alerteStore.afficherSucces(`${cible.asset.id} retiré du pipeline — désarmé partout, données conservées`)
  } catch (e) {
    alerteStore.afficherErreur(`Retrait ${cible.asset.id} : ${(e as Error).message}`)
  }
  retraitEnCours.value = false
}

async function basculerAsset(a: AssetInfo) {
  if (a.actif) {
    await demanderRetrait(a)
    return
  }
  try {
    // Bascule totale 24/10 : MT5/Axi seule source — l'EA ne nourrit que les
    // assets mt5, un ajout binance serait un asset mort.
    // Ajout pipeline (doctrine 15/10) : disponible partout, armé nulle part.
    if (!a.symbol_mt5) {
      alerteStore.afficherErreur(`${a.id} : symbole MT5/Axi manquant — passe par « + Ajouter un asset » pour le renseigner`)
      return
    }
    await apiService.ajouterAsset(a.id, a.nom, a.type as AssetInfo['type'], 'mt5', undefined, a.symbol_mt5)
    a.actif = true
    await chargerTous()
  } catch (e) {
    alerteStore.afficherErreur(`Ajout ${a.id} : ${(e as Error).message}`)
  }
}

async function chargerTous() {
  try {
    tous.value = await apiService.obtenirAssets()
  } catch {
    tous.value = assetsStore.assets
  }
}

const mt5 = ref<{ connecte: boolean; symboles: { asset: string; age_s: number }[] }>({ connecte: false, symboles: [] })
async function chargerMt5() {
  try {
    const res = await http.get('/api/mt5/statut')
    mt5.value = res.data
  } catch { mt5.value = { connecte: false, symboles: [] } }
}

// Colonnes par CLASSE d'actif (crypto/métaux/forex/indices) — le broker
// (badge Bybit/MT5) reste indiqué par asset : XAU/XAG en métaux,
// NAS100/SP500/DAX en indices, même alimentés par MT5/Axi.
const CATEGORIES = computed(() => [
  { type: 'crypto', label: '🪙 Crypto', couleur: 'text-yellow-400', assets: tous.value.filter(a => a.type === 'crypto') },
  { type: 'metal', label: '🥇 Métaux', couleur: 'text-amber-400', assets: tous.value.filter(a => a.type === 'metal') },
  { type: 'forex', label: '💱 Forex', couleur: 'text-blue-400', assets: tous.value.filter(a => a.type === 'forex') },
  { type: 'indice', label: '📈 Indices', couleur: 'text-purple-400', assets: tous.value.filter(a => a.type === 'indice') },
])

function badgeSource(source?: string): { label: string; classe: string } {
  switch (source) {
    case 'binance':
      return { label: 'Bybit', classe: 'bg-yellow-500/15 text-yellow-300' }
    case 'mt5':
      return { label: 'MT5 / Axi', classe: 'bg-violet-500/15 text-violet-300' }
    default:
      return { label: '—', classe: 'bg-white/10 text-white' }
  }
}


// ── Import MT5 (bouton historique conservé) ───────────────────────────────────

/// Couverture en 4 colonnes PAR CATÉGORIE — même tri que le bloc « Assets
/// du pipeline » : Cryptos · Métaux · Forex · Indices (type de chaque asset).
const colonnesCouverture = computed(() => {
  const typeParAsset = new Map(tous.value.map(a => [a.id, a.type]))
  const parType: Record<string, typeof lignesEnrichies.value> = {
    crypto: [], metal: [], forex: [], indice: [],
  }
  for (const ligne of lignesEnrichies.value) {
    const type = typeParAsset.get(ligne.asset) ?? 'indice'
    parType[type]?.push(ligne)
  }
  return CATEGORIES.value
    .map(cat => ({ ...cat, lignes: parType[cat.type] ?? [] }))
    .filter(c => c.lignes.length > 0)
})

// ── Section 4 : couverture DB (auto-refresh 60 s) ─────────────────────────────
const couverture = ref<CouvertureDonnees[]>([])
const chargement = ref(false)

// ── Garde fraîcheur pipeline (16/10) : badge ⚠ par asset, poll 60 s ──────────
const { charger: chargerFraicheur, anomalies: anomaliesAsset, libelle: libelleAnomalie, enAlerte: assetsEnAlerte } = useFraicheurPipeline()

const ASSETS_CRYPTO = new Set(['BTC', 'ETH', 'SOL', 'BNB', 'XRP', 'ADA', 'DOGE', 'AVAX', 'LINK', 'DOT'])
const ASSETS_METAUX = new Set(['XAUUSD', 'XAGUSD', 'XPTUSD'])
const ASSETS_INDICES = new Set(['DAX', 'GER40', 'NAS100', 'SP500'])

/// Barres attendues PAR JOUR, calibrées le 16/10 sur la densité RÉELLE des
/// assets les mieux servis de chaque classe (crypto=BTC 1401 M1/j, forex=
/// majeures 1019, métaux=XAU/XAG 995-998, indice=NAS100 986 — GER40, plus
/// clairsemé à 925, apparaîtra < 100 %, c'est sa vérité). Le % devient une
/// DENSITÉ : 100 % = aucun trou dans l'historique réellement présent, la
/// profondeur se lit dans les colonnes de dates (plus de comparaison
/// arbitraire à 24 mois qui confondait rétention et trous).
const BARRES_PAR_JOUR: Record<string, Record<string, number>> = {
  crypto: { M1: 1400, M5: 280, M15: 95,  M30: 47, H1: 23.5, H4: 5.9, D1: 1,    W1: 0.143 },
  metal:  { M1: 995,  M5: 199, M15: 68,  M30: 34, H1: 17,   H4: 4.3, D1: 0.71, W1: 0.102 },
  forex:  { M1: 1015, M5: 203, M15: 68,  M30: 34, H1: 17,   H4: 4.3, D1: 0.71, W1: 0.102 },
  indice: { M1: 985,  M5: 197, M15: 68,  M30: 34, H1: 17,   H4: 4.3, D1: 0.71, W1: 0.102 },
}

function classeAsset(asset: string): string {
  if (ASSETS_CRYPTO.has(asset)) return 'crypto'
  if (ASSETS_METAUX.has(asset)) return 'metal'
  if (ASSETS_INDICES.has(asset)) return 'indice'
  return 'forex'
}

/// Barres attendues sur la durée RÉELLE de l'historique de cette série.
function bougiesAttendues(tf: string, asset: string, minTs: number, maxTs: number): number {
  const jours = Math.max((maxTs - minTs) / 86400, 1)
  return (BARRES_PAR_JOUR[classeAsset(asset)]?.[tf] ?? 1) * jours
}

const TF_ORDRE: Record<string, number> = {
  M1: 0, M5: 1, M15: 2, M30: 3, H1: 4, H4: 5, D1: 6, W1: 7,
}

const lignesEnrichies = computed(() => {
  // Ne montrer que les assets ACTIFS (cochés) et les timeframes CONFIGURÉS
  const idsActifs = new Set(tous.value.filter(a => a.actif).map(a => a.id))
  const tfsConfigures = new Set(configWorker.value?.timeframes ?? [])
  const lignes = couverture.value
    .filter(c => idsActifs.has(c.asset) && tfsConfigures.has(c.timeframe))
    .map(c => {
      const pct = Math.min(100, Math.round((c.count / bougiesAttendues(c.timeframe, c.asset, c.min_ts, c.max_ts)) * 100))
      const dateMin = c.min_ts ? new Date(c.min_ts * 1000).toLocaleDateString('fr-FR') : '—'
      const dateMax = c.max_ts ? new Date(c.max_ts * 1000).toLocaleDateString('fr-FR') : '—'
      // Fraîcheur = dernière CLÔTURE — une bougie D1 d'hier ou W1 de la
      // semaine passée est FRAÎCHE (elle n'est pas due). Horloge serveur
      // broker en avance → jamais d'étiquette négative.
      const ageSec = c.max_ts ? Math.floor(Date.now() / 1000 - c.max_ts) : 999 * 86400
      const ageDays = Math.floor(Math.max(0, ageSec) / 86400)
      const ageH = Math.floor(Math.max(0, ageSec) / 3600)
      const fraicheurLabel = ageSec < 0
        ? 'à l’instant'
        : ageH < 1 ? 'à l’instant'
        : ageH < 24 ? `il y a ${ageH} h`
        : ageDays === 1 ? 'hier'
        : `${ageDays} j`
      return { ...c, pct, dateMin, dateMax, ageDays, fraicheurLabel }
    })
    .sort((a, b) => {
      if (a.asset !== b.asset) return a.asset.localeCompare(b.asset)
      return (TF_ORDRE[a.timeframe] ?? 99) - (TF_ORDRE[b.timeframe] ?? 99)
    })

  const assetsVus: string[] = []
  return lignes.map(l => {
    if (!assetsVus.includes(l.asset)) assetsVus.push(l.asset)
    return { ...l, groupIndex: assetsVus.indexOf(l.asset) }
  })
})

const tailleDbOctets = ref<number | null>(null)
const bougiesAujourdHui = ref<number | null>(null)
const tailleDbGo = computed(() => {
  if (!tailleDbOctets.value) return null
  const go = tailleDbOctets.value / 1024 ** 3
  return go >= 1
    ? `${go.toLocaleString('fr-FR', { maximumFractionDigits: 2 })} Go`
    : `${(tailleDbOctets.value / 1024 ** 2).toLocaleString('fr-FR', { maximumFractionDigits: 0 })} Mo`
})

async function chargerCouverture() {
  chargement.value = true
  try {
    const res = await apiService.obtenirCouvertureDonnees()
    couverture.value = res.couverture
    tailleDbOctets.value = res.taille_db_octets ?? null
    bougiesAujourdHui.value = res.bougies_aujourd_hui ?? null
  } catch {
    couverture.value = []
  } finally {
    chargement.value = false
  }
}

// ── Cycle de vie : polls 30 s (statut workers) et 60 s (couverture) ──────────
let minuteurStatut: ReturnType<typeof setInterval> | null = null
let minuteurCouverture: ReturnType<typeof setInterval> | null = null

onMounted(async () => {
  await assetsStore.chargerAssets()
  try {
    tous.value = await apiService.obtenirAssets()
  } catch {
    tous.value = assetsStore.assets
  }
  await chargerConfig()
  await chargerTiingo()
  await chargerCouverture()
  void chargerFraicheur()
  void chargerMt5()
  minuteurStatut = setInterval(chargerTiingo, 30_000)
  minuteurCouverture = setInterval(() => { chargerCouverture(); chargerFraicheur() }, 60_000)
  minuteurMt5 = setInterval(chargerMt5, 30_000)
})

let minuteurMt5: ReturnType<typeof setInterval> | null = null
onUnmounted(() => {
  if (minuteurStatut) clearInterval(minuteurStatut)
  if (minuteurCouverture) clearInterval(minuteurCouverture)
  if (minuteurMt5) clearInterval(minuteurMt5)
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
