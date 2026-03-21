<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">📦 Données Historiques</h1>
      <div class="flex items-center gap-3">
        <span v-if="derniereMaj" class="text-xs text-gray-400">MAJ {{ derniereMaj }}</span>
        <button class="btn-sm" :disabled="chargement" @click="chargerCouverture">
          {{ chargement ? '⏳' : '🔄' }} Actualiser
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
            <th class="px-3 py-2 text-gray-400 text-right">Couverture (obj. {{ moisSelectionne }} mois)</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="ligne in lignesEnrichies"
            :key="ligne.asset + ligne.timeframe"
            class="border-t border-white/5 hover:bg-white/5 transition"
          >
            <td class="px-3 py-2 font-bold text-white">{{ ligne.asset }}</td>
            <td class="px-3 py-2 text-gray-300 text-center">{{ ligne.timeframe }}</td>
            <td class="px-3 py-2 text-right text-white font-mono">{{ ligne.count.toLocaleString() }}</td>
            <td class="px-3 py-2 text-right text-gray-300 text-xs">{{ ligne.dateMin }}</td>
            <td class="px-3 py-2 text-right text-gray-300 text-xs">{{ ligne.dateMax }}</td>
            <td class="px-3 py-2 text-right">
              <div class="flex items-center justify-end gap-2">
                <div class="w-20 h-1.5 rounded-full bg-white/10 overflow-hidden">
                  <div
                    class="h-full rounded-full transition-all"
                    :class="ligne.pct >= 80 ? 'bg-emerald-400' : ligne.pct >= 40 ? 'bg-yellow-400' : 'bg-red-400'"
                    :style="{ width: ligne.pct + '%' }"
                  />
                </div>
                <span class="text-xs text-right whitespace-nowrap" :class="ligne.pct >= 80 ? 'text-emerald-400' : ligne.pct >= 40 ? 'text-yellow-400' : 'text-red-400'">
                  {{ ligne.pct }}%
                  <span class="text-gray-500 font-normal">/ {{ moisSelectionne }}m</span>
                </span>
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
const moisSelectionne = ref(6)
const assetsSelectionnes = ref<string[]>([])
const tfsSelectionnes = ref<string[]>(['M5', 'M15', 'H1', 'H4'])
const tousAssetsSelectionnes = computed(() =>
  TOUS_ASSETS.value.length > 0 && TOUS_ASSETS.value.every(a => assetsSelectionnes.value.includes(a))
)

// Calcul estimé du nombre de bougies attendu pour 6 mois selon TF
const bougiesParMoisParTf: Record<string, number> = {
  M1: 43200, M5: 8640, M15: 2880, M30: 1440,
  H1: 720, H4: 180, D1: 30, W1: 4,
}

const lignesEnrichies = computed(() =>
  couverture.value.map(c => {
    const attendu = (bougiesParMoisParTf[c.timeframe] ?? 1) * moisSelectionne.value
    const pct = Math.min(100, Math.round((c.count / attendu) * 100))
    const dateMin = c.min_ts ? new Date(c.min_ts * 1000).toLocaleDateString('fr-FR') : '—'
    const dateMax = c.max_ts ? new Date(c.max_ts * 1000).toLocaleDateString('fr-FR') : '—'
    return { ...c, pct, dateMin, dateMax }
  })
)

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
.btn-sm { @apply px-3 py-1.5 rounded-lg bg-white/10 text-gray-300 text-sm hover:bg-white/20 transition disabled:opacity-50; }
</style>
