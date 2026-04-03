<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">📦 Données Historiques</h1>
      <div class="flex items-center gap-3">
        <span v-if="derniereMaj" class="text-xs text-gray-400">MAJ {{ derniereMaj }}</span>
        <button
          class="px-4 py-1.5 rounded-lg bg-blue-500/20 text-blue-400 text-sm font-semibold hover:bg-blue-500/30 transition disabled:opacity-50"
          :disabled="enImportMt5"
          @click="importerMt5"
        >
          {{ enImportMt5 ? '⏳ Import MT5…' : '📥 Importer depuis MT5' }}
        </button>
      </div>
    </div>

    <!-- Collecte globale -->
    <div class="glass-card p-5 flex flex-wrap items-end gap-4">
      <div class="flex items-center gap-2">
        <label class="text-sm text-gray-400 shrink-0">Mois d'historique :</label>
        <select v-model="moisSelectionne" class="bg-white border border-white/20 rounded-lg px-3 py-1.5 text-sm text-black">
          <option v-for="m in [1, 3, 6, 12, 24]" :key="m" :value="m">{{ m }} mois</option>
        </select>
      </div>
      <div class="flex flex-col gap-1">
        <div class="flex items-center gap-2">
          <span class="text-xs text-gray-400">Assets :</span>
          <button class="text-xs text-blue-400 hover:underline" @click="tousAssetsSelectionnes ? assetsSelectionnes = [] : assetsSelectionnes = TOUS_ASSETS.slice()">
            {{ tousAssetsSelectionnes ? 'Tout décocher' : 'Tout cocher' }}
          </button>
        </div>
        <div class="flex flex-wrap gap-2">
          <label
            v-for="a in TOUS_ASSETS"
            :key="a"
            class="flex items-center gap-1 cursor-pointer select-none text-xs px-2 py-1 rounded-lg border transition"
            :class="assetsSelectionnes.includes(a)
              ? 'border-blue-500/50 bg-blue-500/10 text-blue-400'
              : 'border-white/10 bg-white/5 text-gray-400'"
          >
            <input type="checkbox" class="hidden" :value="a" v-model="assetsSelectionnes" />
            {{ a }}
          </label>
        </div>
      </div>
      <!-- Sélecteur Timeframes -->
      <div class="flex flex-col gap-1">
        <span class="text-xs text-gray-400">Timeframes :</span>
        <div class="flex flex-wrap gap-2">
          <label
            v-for="tf in TOUS_TF"
            :key="tf"
            class="flex items-center gap-1 cursor-pointer select-none text-xs px-2 py-1 rounded-lg border transition"
            :class="tfsSelectionnes.includes(tf)
              ? 'border-emerald-500/50 bg-emerald-500/10 text-emerald-400'
              : 'border-white/10 bg-white/5 text-gray-400'"
          >
            <input type="checkbox" class="hidden" :value="tf" v-model="tfsSelectionnes" />
            {{ tf }}
          </label>
        </div>
      </div>
      <div class="flex flex-col gap-2">
        <button
          class="px-4 py-2 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 transition disabled:opacity-50"
          :disabled="enCollecte || tfsSelectionnes.length === 0 || assetsSelectionnes.length === 0"
          @click="lancerCollecte"
        >
          {{ enCollecte ? '⏳ Collecte en cours…' : '⬇ Lancer la collecte' }}
        </button>
        <span v-if="messageCollecte" class="text-sm" :class="erreurCollecte ? 'text-red-400' : 'text-emerald-400'">
          {{ messageCollecte }}
        </span>
      </div>
    </div>

    <!-- Résultats du dernier import MT5 -->
    <div v-if="messageImportMt5" class="glass-card p-4 flex items-center gap-3">
      <span :class="erreurImportMt5 ? 'text-red-400' : 'text-blue-400'" class="text-sm font-semibold">{{ messageImportMt5 }}</span>
      <span v-if="!erreurImportMt5 && statsImportMt5" class="text-xs text-gray-400">
        ({{ statsImportMt5.total_bougies.toLocaleString() }} lues · {{ statsImportMt5.total_inseres.toLocaleString() }} insérées)
      </span>
    </div>

    <!-- Résultats de la dernière collecte -->
    <div v-if="resultatsCollecte.length > 0" class="glass-card p-5 space-y-2">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Résultats collecte</h2>
      <div class="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4">
        <div
          v-for="r in resultatsCollecte"
          :key="r.asset + r.timeframe"
          class="rounded-lg px-3 py-2 text-xs"
          :class="r.erreur ? 'bg-red-500/10 border border-red-500/20' : 'bg-emerald-500/10 border border-emerald-500/20'"
        >
          <p class="font-bold text-white">{{ r.asset }} {{ r.timeframe }}</p>
          <p v-if="r.erreur" class="text-red-400 truncate">{{ r.erreur }}</p>
          <p v-else class="text-emerald-400">
            {{ r.inseres }} nouveaux / {{ r.fetched }} reçus
          </p>
        </div>
      </div>
    </div>

    <!-- Grille de couverture -->
    <div class="glass-card p-5">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">Couverture par asset × timeframe</h2>
      <div v-if="chargement" class="text-gray-400 text-sm animate-pulse text-center py-8">Chargement…</div>
      <div v-else-if="couverture.length === 0" class="text-gray-500 text-sm text-center py-8">
        Aucune donnée — lancez une collecte pour remplir la base.
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
              <div v-if="ligne.estCrypto" class="flex items-center justify-end gap-2">
                <div class="w-20 h-1.5 rounded-full bg-white/10 overflow-hidden">
                  <div
                    class="h-full rounded-full transition-all"
                    :class="ligne.pct >= 80 ? 'bg-emerald-400' : ligne.pct >= 40 ? 'bg-yellow-400' : 'bg-red-400'"
                    :style="{ width: ligne.pct + '%' }"
                  />
                </div>
                <span class="text-xs whitespace-nowrap" :class="ligne.pct >= 80 ? 'text-emerald-400' : ligne.pct >= 40 ? 'text-yellow-400' : 'text-red-400'">{{ ligne.pct }}%</span>
              </div>
              <div v-else class="flex items-center justify-end gap-1.5">
                <span class="w-2 h-2 rounded-full shrink-0" :class="ligne.ageDays <= 2 ? 'bg-emerald-400' : ligne.ageDays <= 7 ? 'bg-yellow-400' : 'bg-red-400'" />
                <span class="text-xs" :class="ligne.ageDays <= 2 ? 'text-emerald-400' : ligne.ageDays <= 7 ? 'text-yellow-400' : 'text-red-400'">{{ ligne.fraicheurLabel }}</span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { CouvertureDonnees, ResultatCollecteItem } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'

const assetsStore = useAssetsStore()
const TOUS_ASSETS = computed(() => assetsStore.assets.map(a => a.id))
const TOUS_TF = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']

const couverture = ref<CouvertureDonnees[]>([])
const chargement = ref(false)
const derniereMaj = ref<string | null>(null)
const enCollecte = ref(false)
const messageCollecte = ref<string | null>(null)
const erreurCollecte = ref(false)
const resultatsCollecte = ref<ResultatCollecteItem[]>([])
const enImportMt5 = ref(false)
const messageImportMt5 = ref<string | null>(null)
const erreurImportMt5 = ref(false)
const statsImportMt5 = ref<{ total_bougies: number; total_inseres: number } | null>(null)
const moisSelectionne = ref(6)
const assetsSelectionnes = ref<string[]>([])
const tfsSelectionnes = ref<string[]>(['M5', 'M15', 'H1', 'H4'])
const tousAssetsSelectionnes = computed(() =>
  TOUS_ASSETS.value.length > 0 && TOUS_ASSETS.value.every(a => assetsSelectionnes.value.includes(a))
)

// Assets crypto (24/7) vs marchés régulés (≈5/7 jours, sessions limitées)
const ASSETS_CRYPTO = new Set(['BTC', 'ETH', 'SOL', 'BNB', 'XRP', 'ADA', 'DOGE', 'AVAX', 'LINK', 'DOT'])

// Bougies attendues par mois — crypto 24/7 uniquement (non-crypto → fraîcheur)
const bougiesParMoisCrypto: Record<string, number> = {
  M1: 43200, M5: 8640, M15: 2880, M30: 1440,
  H1: 720, H4: 180, D1: 30, W1: 4,
}

function bougiesAttendues(tf: string, mois: number): number {
  return (bougiesParMoisCrypto[tf] ?? 1) * mois
}

const TF_ORDRE: Record<string, number> = {
  M1: 0, M5: 1, M15: 2, M30: 3, H1: 4, H4: 5, D1: 6, W1: 7,
}

const lignesEnrichies = computed(() => {
  const lignes = couverture.value
    .filter(c => TOUS_ASSETS.value.includes(c.asset))
    .map(c => {
      const estCrypto = ASSETS_CRYPTO.has(c.asset)
      const pct = estCrypto
        ? Math.min(100, Math.round((c.count / bougiesAttendues(c.timeframe, moisSelectionne.value)) * 100))
        : 0
      const dateMin = c.min_ts ? new Date(c.min_ts * 1000).toLocaleDateString('fr-FR') : '—'
      const dateMax = c.max_ts ? new Date(c.max_ts * 1000).toLocaleDateString('fr-FR') : '—'
      const ageDays = c.max_ts ? Math.floor((Date.now() / 1000 - c.max_ts) / 86400) : 999
      const fraicheurLabel = ageDays === 0 ? "Aujourd'hui" : ageDays === 1 ? 'Hier' : `${ageDays}j`
      return { ...c, estCrypto, pct, dateMin, dateMax, ageDays, fraicheurLabel }
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

async function chargerCouverture() {
  chargement.value = true
  try {
    const res = await apiService.obtenirCouvertureDonnees()
    couverture.value = res.couverture
    derniereMaj.value = new Date().toLocaleTimeString('fr-FR')
  } catch {
    couverture.value = []
  } finally {
    chargement.value = false
  }
}

async function importerMt5() {
  enImportMt5.value = true
  messageImportMt5.value = null
  erreurImportMt5.value = false
  statsImportMt5.value = null
  try {
    const res = await apiService.importerMt5()
    statsImportMt5.value = { total_bougies: res.total_bougies, total_inseres: res.total_inseres }
    if (res.message) {
      messageImportMt5.value = `ℹ️ ${res.message}`
    } else {
      messageImportMt5.value = `✅ Import MT5 terminé — ${res.resultats.length} fichier(s) traité(s)`
    }
    await chargerCouverture()
  } catch (err: unknown) {
    erreurImportMt5.value = true
    messageImportMt5.value = `❌ Erreur MT5 : ${err instanceof Error ? err.message : 'inconnue'}`
  } finally {
    enImportMt5.value = false
  }
}

async function lancerCollecte() {
  enCollecte.value = true
  messageCollecte.value = null
  erreurCollecte.value = false
  resultatsCollecte.value = []
  try {
    const assets = assetsSelectionnes.value
    const res = await apiService.collecterDonnees({
      assets,
      timeframes: tfsSelectionnes.value,
      mois: moisSelectionne.value,
    })
    resultatsCollecte.value = res.resultats
    messageCollecte.value = `✅ ${res.total_inseres.toLocaleString()} nouvelles bougies insérées`
    await chargerCouverture()
  } catch (err: unknown) {
    erreurCollecte.value = true
    messageCollecte.value = `❌ Erreur : ${err instanceof Error ? err.message : 'inconnue'}`
  } finally {
    enCollecte.value = false
  }
}

onMounted(async () => {
  await assetsStore.chargerAssets()
  assetsSelectionnes.value = TOUS_ASSETS.value.slice()
  await chargerCouverture()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
