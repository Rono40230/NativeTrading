<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">📦 Données — Pilotage du pipeline</h1>

    <!-- ══ SECTION 1 — Contrôle des workers ══════════════════════════════════ -->
    <div class="glass-card p-5 space-y-4">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Workers d'ingestion</h2>

      <!-- Collecteur MT5/Axi (statut de l'EA — pas d'interrupteur : MT5
           ouvert = collecte, MT5 fermé = silence) -->
      <div class="rounded-xl border p-4 space-y-3"
           :class="mt5.connecte ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-white/10 bg-white/[0.02]'">
        <div class="flex items-center justify-between">
          <div>
            <p class="font-bold text-white">🖥️ Collecteur MT5 / Axi</p>
            <p class="text-xs text-gray-500">Bougies M1 de ton broker — l'EA attaché dans MT5</p>
          </div>
          <span v-if="mt5.connecte" class="text-emerald-400 text-sm">● Connecté</span>
          <span v-else class="text-gray-400 text-sm">○ Silencieux (MT5 fermé ?)</span>
        </div>
        <div class="flex flex-wrap gap-x-4 gap-y-1 text-xs" v-if="mt5.symboles?.length">
          <span v-for="s in mt5.symboles" :key="s.asset" class="text-gray-400">
            {{ s.asset }} :
            <span v-if="s.age_s >= 0 && s.age_s < 120" class="text-emerald-400">prix il y a {{ s.age_s }}s</span>
            <span v-else class="text-gray-600">sans prix (marché fermé ?)</span>
          </span>
        </div>
        <p v-else class="text-xs text-gray-600">Aucun actif MT5 actif</p>
      </div>

      <!-- Interrupteurs + statut -->
      <div class="grid gap-4 md:grid-cols-2">
        <div
          v-for="w in cartesWorkers"
          :key="w.nom"
          class="rounded-xl border p-4 space-y-3 transition"
          :class="w.config.actif ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-white/10 bg-white/[0.02]'"
        >
          <div class="flex items-center justify-between">
            <div>
              <p class="font-bold text-white">{{ w.nom }}</p>
              <p class="text-xs text-gray-500">{{ w.description }}</p>
            </div>
            <button
              class="px-4 py-1.5 rounded-lg text-sm font-semibold transition disabled:opacity-50"
              :class="w.config.actif
                ? 'bg-amber-500/20 text-amber-400 hover:bg-amber-500/30'
                : 'bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30'"
              :disabled="enEcritureConfig"
              @click="basculerWorker(w.cle)"
            >
              {{ w.config.actif ? '⏸ Pause' : '▶ Démarrer' }}
            </button>
          </div>
          <div class="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
            <span v-if="!w.config.actif" class="text-gray-400">⏸ Désactivé</span>
            <span v-else-if="w.statut?.connecte" class="text-emerald-400">● Connecté</span>
            <span v-else class="text-red-400">○ Déconnecté</span>
            <span class="text-gray-400">
              {{ w.statut?.nb_assets ?? 0 }} asset(s) suivis
            </span>
            <span class="text-gray-400">Dernière bougie : {{ fraicheur(w.statut?.derniere_bougie ?? null) }}</span>
            <span class="text-gray-500">{{ (w.statut?.bougies_inserees ?? 0).toLocaleString() }} bougies insérées</span>
          </div>
        </div>
      </div>

      <p v-if="messageConfig" class="text-sm" :class="erreurConfig ? 'text-red-400' : 'text-emerald-400'">
        {{ messageConfig }}
      </p>
    </div>

    <!-- ══ SECTION 2 — Assets du pipeline ════════════════════════════════════ -->
    <div class="glass-card p-5">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Assets du pipeline</h2>
        <div class="flex items-center gap-3">
          <button
            class="px-3 py-1 rounded-lg bg-emerald-500/20 text-emerald-400 text-xs font-semibold hover:bg-emerald-500/30 transition"
            @click="modaleAjout?.ouvrirModaleAsset()"
          >
            + Ajouter un asset
          </button>
          <span class="text-xs text-gray-500">{{ nbAssetsActifs }} / {{ tous.length }} activés</span>
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
              <span
                class="ml-auto text-[10px] px-1.5 py-0.5 rounded font-semibold shrink-0"
                :class="badgeSource(a.source).classe"
              >
                {{ badgeSource(a.source).label }}
              </span>
              <span v-if="enCoursAsset === a.id" class="text-[10px] text-gray-500">…</span>
            </label>
          </div>
        </div>
      </div>
      <p v-if="erreurAssets" class="text-red-400 text-xs mt-2">{{ erreurAssets }}</p>
      <p class="text-gray-500 text-[11px] mt-2">
        Décocher un asset l'exclut des workers à leur prochaine session/cycle (≤ 60 s) — les données sont conservées.
      </p>
    </div>

    <!-- Modale d'ajout (composant dédié) -->
    <ModaleAjoutAsset ref="modaleAjout" @cree="chargerTous" />

    <!-- ══ SECTION 3 — Backfill Dukascopy (remplace l'import CSV) ═════════════ -->
    <div class="glass-card p-4 space-y-2">
      <div class="flex items-center justify-between gap-3 flex-wrap">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">
          Backfill Dukascopy
        </h2>
        <span class="text-[11px] text-gray-500">
          Bouton ⬇ dans la table de couverture — télécharge l'historique depuis le datafeed
          public Dukascopy (rate-limité : ~2 min par mois, sans clé API).
        </span>
      </div>
      <div v-if="progressionBackfill" class="text-sm text-blue-400 flex items-center gap-2">
        <span class="inline-block w-3 h-3 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
        {{ progressionBackfill }}
      </div>
      <div
        v-if="resultatBackfill"
        class="text-sm whitespace-pre-line"
        :class="erreurBackfill ? 'text-red-400' : 'text-emerald-400'"
      >
        {{ resultatBackfill }}
      </div>
    </div>

    <!-- ══ SECTION 4 — Couverture DB ═════════════════════════════════════════ -->
    <div class="glass-card p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Couverture par asset × timeframe</h2>
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
            class="text-xs text-gray-400 px-2 py-0.5 rounded-lg bg-white/5 border border-white/10 tabular-nums"
            :title="`Taille de la base de données (${tailleDbOctets?.toLocaleString('fr-FR')} octets)`"
          >
            💾 DB : {{ tailleDbGo }}
          </span>
        </div>
      </div>
      <div v-if="chargement" class="text-gray-400 text-sm animate-pulse text-center py-8">Chargement…</div>
      <div v-else-if="couverture.length === 0" class="text-gray-500 text-sm text-center py-8">
        Aucune donnée — activez les workers ou lancez un backfill Dukascopy (⬇) pour remplir la base.
      </div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr>
            <th class="text-left px-3 py-2 text-gray-400">Asset</th>
            <th class="px-3 py-2 text-gray-400">TF</th>
            <th class="px-3 py-2 text-gray-400 text-right">Bougies</th>
            <th class="px-3 py-2 text-gray-400 text-right">Depuis</th>
            <th class="px-3 py-2 text-gray-400 text-right">Jusqu'à</th>
            <th class="px-3 py-2 text-gray-400 text-right">Statut</th>
            <th class="px-3 py-2 text-gray-400 text-right" title="Télécharger l'historique depuis Dukascopy">⬇</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="ligne in lignesEnrichies"
            :key="ligne.asset + ligne.timeframe"
            class="border-t border-white/5 hover:bg-white/10 transition"
            :class="ligne.groupIndex % 2 === 1 ? 'bg-white/[0.04]' : ''"
          >
            <td class="px-3 py-2 font-bold text-white">{{ ligne.asset }}</td>
            <td class="px-3 py-2 text-gray-300 text-center">{{ ligne.timeframe }}</td>
            <td class="px-3 py-2 text-right text-white font-mono">{{ ligne.count.toLocaleString() }}</td>
            <td class="px-3 py-2 text-right text-gray-300 text-xs">{{ ligne.dateMin }}</td>
            <td class="px-3 py-2 text-right text-gray-300 text-xs">{{ ligne.dateMax }}</td>
            <td class="px-3 py-2 text-right">
              <div class="flex items-center justify-end gap-2">
                <div class="w-20 h-1.5 rounded-full bg-white/10 overflow-hidden shrink-0">
                  <div
                    class="h-full rounded-full transition-all"
                    :class="ligne.pct >= 80 ? 'bg-emerald-400' : ligne.pct >= 40 ? 'bg-yellow-400' : 'bg-red-400'"
                    :style="{ width: ligne.pct + '%' }"
                  />
                </div>
                <span class="text-xs whitespace-nowrap tabular-nums" :class="ligne.pct >= 80 ? 'text-emerald-400' : ligne.pct >= 40 ? 'text-yellow-400' : 'text-red-400'">{{ ligne.pct }}%</span>
                <span class="text-xs whitespace-nowrap text-gray-500">{{ ligne.fraicheurLabel }}</span>
              </div>
            </td>
            <td class="px-3 py-2 text-right">
              <button
                :disabled="enBackfill"
                class="text-blue-400 hover:text-blue-200 text-sm transition disabled:opacity-30"
                :title="enBackfill
                  ? 'Backfill en cours…'
                  : `Télécharger ${MOIS_RETENTION} mois d'historique ${ligne.asset} ${ligne.timeframe} depuis Dukascopy`"
                @click="backfillDukascopy(ligne.asset, ligne.timeframe)"
              >
                {{ enBackfill && cibleBackfill === `${ligne.asset}-${ligne.timeframe}` ? '⏳' : '⬇' }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'
import ModaleAjoutAsset from '@/components/common/ModaleAjoutAsset.vue'
import type { AssetInfo } from '@/services/api.service'
import { apiService } from '@/services/api.service'
import type { CouvertureDonnees } from '@/services/api.service'
import type { WorkerConfig, WorkerStatus } from '@/services/api.worker'
import { useAssetsStore } from '@/stores/assets.store'

const assetsStore = useAssetsStore()
const TOUS_TF = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']

// ── Section 1 : workers ───────────────────────────────────────────────────────
const configWorker = ref<WorkerConfig | null>(null)
const statutWorkers = ref<WorkerStatus | null>(null)
const enEcritureConfig = ref(false)
const messageConfig = ref<string | null>(null)
const erreurConfig = ref(false)
/// Politique fixe (décisions propriétaire 2026-08-15) : rétention/stockage 2 ans.
const MOIS_RETENTION = 24

const cartesWorkers = computed(() => [
  {
    nom: 'Bybit WS',
    description: 'Crypto + métaux — WebSocket temps réel',
    cle: 'actif_bybit' as const,
    config: {
      actif: configWorker.value?.actif_bybit ?? false,
    },
    statut: statutWorkers.value?.bybit,
  },
])

function fraicheur(ts: number | null): string {
  if (!ts) return '—'
  const secondes = Math.floor(Date.now() / 1000) - ts
  if (secondes < 90) return "à l'instant"
  if (secondes < 3600) return `il y a ${Math.floor(secondes / 60)} min`
  if (secondes < 86_400) return `il y a ${Math.floor(secondes / 3600)} h`
  return `il y a ${Math.floor(secondes / 86_400)} j`
}

async function chargerConfig() {
  try {
    configWorker.value = await apiService.getWorkerConfig()
  } catch {
    // Config par défaut affichée — le PUT réécrira la config serveur.
  }
}

async function chargerStatutWorkers() {
  try {
    statutWorkers.value = await apiService.getWorkerStatus()
  } catch {
    statutWorkers.value = null
  }
}

async function basculerWorker(cle: 'actif_bybit') {
  const c = configWorker.value
  if (!c || enEcritureConfig.value) return
  enEcritureConfig.value = true
  messageConfig.value = null
  try {
    configWorker.value = await apiService.putWorkerConfig({ [cle]: !c[cle] })
    messageConfig.value = `Worker Bybit ${configWorker.value[cle] ? 'activé' : 'mis en pause'} — effet sous 60 s max`
    erreurConfig.value = false
    await chargerStatutWorkers()
  } catch (err: unknown) {
    erreurConfig.value = true
    messageConfig.value = `❌ Erreur : ${err instanceof Error ? err.message : 'inconnue'}`
  } finally {
    enEcritureConfig.value = false
  }
}

// ── Section 2 : assets pipeline ───────────────────────────────────────────────
const tous = ref<AssetInfo[]>([])
const enCoursAsset = ref<string | null>(null)
const erreurAssets = ref('')

const nbAssetsActifs = computed(() => tous.value.filter(a => a.actif).length)

// ── Collecteur MT5 : statut (poll 30 s) ──────────────────────────────────────
const modaleAjout = ref<InstanceType<typeof ModaleAjoutAsset> | null>(null)

async function basculerAsset(a: AssetInfo) {
  try {
    if (a.actif) {
      await apiService.supprimerAsset(a.id)
    } else {
      const source = a.source === 'mt5' ? 'mt5' : a.type === 'crypto' || a.type === 'metal' ? 'binance' : 'dukascopy'
      await apiService.ajouterAsset(a.id, a.nom, a.type as AssetInfo['type'], source, undefined, undefined, a.symbol_mt5 || undefined)
    }
    a.actif = !a.actif
    await chargerTous()
  } catch (e: unknown) {
    /* silencieux : le re-chargement réaligne l'état */
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

const CATEGORIES = computed(() => [
  { type: 'crypto', label: '🪙 Crypto (Bybit)', couleur: 'text-yellow-400', assets: tous.value.filter(a => a.type === 'crypto') },
  { type: 'metal', label: '🥇 Métaux (Bybit)', couleur: 'text-amber-400', assets: tous.value.filter(a => a.type === 'metal') },
  { type: 'forex', label: '💱 Forex (Dukascopy)', couleur: 'text-blue-400', assets: tous.value.filter(a => a.type === 'forex') },
  { type: 'mt5', label: '🖥️ Broker MT5 / Axi (temps réel)', couleur: 'text-violet-400', assets: tous.value.filter(a => a.source === 'mt5') },
  { type: 'indice', label: '📈 Indices (Dukascopy)', couleur: 'text-purple-400', assets: tous.value.filter(a => a.type === 'indice' && a.source !== 'mt5') },
])

function badgeSource(source?: string): { label: string; classe: string } {
  switch (source) {
    case 'binance':
      return { label: 'Bybit', classe: 'bg-yellow-500/15 text-yellow-300' }
    case 'dukascopy':
      return { label: 'Dukascopy', classe: 'bg-sky-500/15 text-sky-300' }
    case 'mt5':
      return { label: 'MT5 / Axi', classe: 'bg-violet-500/15 text-violet-300' }
    default:
      return { label: '—', classe: 'bg-white/10 text-gray-400' }
  }
}


// ── Section 3 : backfill Dukascopy (bouton ⬇ de la table de couverture) ──────
const enBackfill = ref(false)
const cibleBackfill = ref<string | null>(null)
const progressionBackfill = ref<string | null>(null)
const resultatBackfill = ref<string | null>(null)
const erreurBackfill = ref(false)

/**
 * Télécharge l'historique d'un asset × timeframe depuis le datafeed public
 * Dukascopy, mois par mois (chaque appel = ~22 fichiers quotidiens espacés
 * de 4 s à cause du rate limit). L'instrument Dukascopy est résolu côté
 * serveur via la colonne `assets.datafeed_dukascopy`.
 */
async function backfillDukascopy(asset: string, timeframe: string) {
  if (enBackfill.value) return
  enBackfill.value = true
  cibleBackfill.value = `${asset}-${timeframe}`
  resultatBackfill.value = null
  erreurBackfill.value = false
  progressionBackfill.value = `Préparation du backfill ${asset} ${timeframe}…`

  const nbMois = MOIS_RETENTION
  const maintenant = new Date()
  let totalBougies = 0
  let totalInserees = 0
  let totalErreurs = 0
  const avertissements: string[] = []

  try {
    for (let i = 0; i < nbMois; i++) {
      // Le mois courant est envoyé tel quel : le serveur s'arrête à hier.
      const date = new Date(maintenant.getFullYear(), maintenant.getMonth() - i, 1)
      const annee = date.getFullYear()
      const mois = date.getMonth() + 1 // 1-indexé côté API
      progressionBackfill.value =
        `Téléchargement ${asset} ${timeframe} — mois ${i + 1}/${nbMois} ` +
        `(${mois.toString().padStart(2, '0')}/${annee}) — ${totalInserees.toLocaleString()} bougies`

      const res = await apiService.backfillDukascopyMois({ asset, timeframe, annee, mois })
      totalBougies += res.bougies
      totalInserees += res.inserees
      totalErreurs += res.erreurs.length
      if (res.avertissement && !avertissements.includes(res.avertissement)) {
        avertissements.push(res.avertissement)
      }
    }

    if (totalInserees > 0) {
      erreurBackfill.value = false
      resultatBackfill.value =
        `✅ ${asset} ${timeframe} : ${totalInserees.toLocaleString()} bougies insérées ` +
        `(${totalBougies.toLocaleString()} M1 téléchargées sur ${nbMois} mois)` +
        (totalErreurs > 0 ? ` — ${totalErreurs} jour(s) en erreur (rate limit), relancer pour compléter` : '')
    } else {
      erreurBackfill.value = true
      resultatBackfill.value =
        `⚠️ ${asset} ${timeframe} : aucune bougie insérée sur ${nbMois} mois`
    }
    if (avertissements.length > 0) {
      resultatBackfill.value += `\n${avertissements.join('\n')}`
    }
    await chargerCouverture()
  } catch (err: unknown) {
    erreurBackfill.value = true
    const message = err instanceof Error ? err.message : 'Erreur inconnue'
    const detail = /timeout/i.test(message)
      ? ' (délai dépassé — le rate limit Dukascopy est peut-être actif, relancer dans quelques minutes)'
      : ''
    resultatBackfill.value =
      `❌ Backfill ${asset} ${timeframe} interrompu après ${totalInserees.toLocaleString()} bougies : ${message}${detail}`
    // On rafraîchit quand même : les mois déjà traités sont en base.
    await chargerCouverture()
  } finally {
    enBackfill.value = false
    cibleBackfill.value = null
    progressionBackfill.value = null
  }
}

// ── Import MT5 (bouton historique conservé) ───────────────────────────────────

// ── Section 4 : couverture DB (auto-refresh 60 s) ─────────────────────────────
const couverture = ref<CouvertureDonnees[]>([])
const chargement = ref(false)

const ASSETS_CRYPTO = new Set(['BTC', 'ETH', 'SOL', 'BNB', 'XRP', 'ADA', 'DOGE', 'AVAX', 'LINK', 'DOT'])
const ASSETS_METAUX = new Set(['XAUUSD', 'XAGUSD'])
const ASSETS_INDICES = new Set(['DAX', 'NAS100', 'SP500'])

/// Bougies attendues par mois, selon le calendrier de cotation de la classe :
/// - crypto : 24/7 ;
/// - métaux : ~23 h × 5 j/7 (historique spot, break quotidien) ;
/// - forex : 24 h × 5 j/7 ;
/// - indices : ~8,5 h × 5 j/7 (~22 jours ouvrés/mois).
/// Heuristiques indicatives — le % mesure un ordre de couverture, pas un audit.
const BOUGIES_PAR_MOIS: Record<string, Record<string, number>> = {
  crypto: { M1: 43200, M5: 8640, M15: 2880, M30: 1440, H1: 720, H4: 180, D1: 30, W1: 4 },
  metal:  { M1: 29500, M5: 5900, M15: 1970, M30: 985,  H1: 493, H4: 123, D1: 22, W1: 4 },
  forex:  { M1: 30860, M5: 6170, M15: 2060, M30: 1030, H1: 514, H4: 129, D1: 22, W1: 4 },
  indice: { M1: 11200, M5: 2240, M15: 750,  M30: 375,  H1: 125, H4: 31,  D1: 21, W1: 4 },
}

function classeAsset(asset: string): string {
  if (ASSETS_CRYPTO.has(asset)) return 'crypto'
  if (ASSETS_METAUX.has(asset)) return 'metal'
  if (ASSETS_INDICES.has(asset)) return 'indice'
  return 'forex'
}

function bougiesAttendues(tf: string, mois: number, asset: string): number {
  return (BOUGIES_PAR_MOIS[classeAsset(asset)]?.[tf] ?? 1) * mois
}

const TF_ORDRE: Record<string, number> = {
  M1: 0, M5: 1, M15: 2, M30: 3, H1: 4, H4: 5, D1: 6, W1: 7,
}

const lignesEnrichies = computed(() => {
  // Ne montrer que les assets ACTIFS (cochés) et les timeframes CONFIGURÉS
  const idsActifs = new Set(tous.value.filter(a => a.actif).map(a => a.id))
  const tfsConfigures = new Set(configWorker.value?.timeframes ?? [])
  const moisReference = MOIS_RETENTION
  const lignes = couverture.value
    .filter(c => idsActifs.has(c.asset) && tfsConfigures.has(c.timeframe))
    .map(c => {
      const pct = Math.min(100, Math.round((c.count / bougiesAttendues(c.timeframe, moisReference, c.asset)) * 100))
      const dateMin = c.min_ts ? new Date(c.min_ts * 1000).toLocaleDateString('fr-FR') : '—'
      const dateMax = c.max_ts ? new Date(c.max_ts * 1000).toLocaleDateString('fr-FR') : '—'
      const ageDays = c.max_ts ? Math.floor((Date.now() / 1000 - c.max_ts) / 86400) : 999
      const fraicheurLabel = ageDays === 0 ? "Aujourd'hui" : ageDays === 1 ? 'Hier' : `${ageDays}j`
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
  await chargerStatutWorkers()
  await chargerCouverture()
  void chargerMt5()
  minuteurStatut = setInterval(chargerStatutWorkers, 30_000)
  minuteurCouverture = setInterval(chargerCouverture, 60_000)
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
