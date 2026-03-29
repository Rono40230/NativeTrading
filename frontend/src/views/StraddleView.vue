<template>
  <div class="flex flex-col gap-5">
    <!-- En-tête -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-xl font-bold text-white">⚡ Straddle — Créneaux de volatilité</h1>
        <p class="text-sm text-gray-400 mt-1">
          Le LLM analyse l'historique OHLCV et identifie les créneaux récurrents de forte volatilité bidirectionnelle.
          <RouterLink to="/pnl" class="text-yellow-400 hover:underline ml-1">→ P&amp;L</RouterLink>
        </p>
      </div>
    </div>

    <!-- Panneau de lancement -->
    <div class="glass-card p-4 flex flex-wrap gap-4 items-end">
      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Asset</label>
        <AppSelect v-model="asset" :options="optionsAssets" />
      </div>

      <div class="flex flex-col gap-1">
        <label class="text-xs text-gray-400 uppercase tracking-wider">Période d'analyse</label>
        <AppSelect v-model="periode" :options="optionsPeriode" />
      </div>

      <button class="btn-primary disabled:opacity-50" :disabled="chargement" @click="analyser">
        {{ chargement ? '⏳ Analyse LLM…' : '🔍 Analyser' }}
      </button>

      <button class="btn-secondary" @click="chargerCreneaux">
        🔄 Actualiser la liste
      </button>
    </div>

    <!-- Message résultat analyse -->
    <div
      v-if="dernierResultat"
      class="rounded-lg px-4 py-3 text-sm border"
      :class="dernierResultat.nb_retenus > 0
        ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-300'
        : 'bg-amber-500/10 border-amber-500/30 text-amber-300'"
    >
      <span v-if="dernierResultat.nb_retenus > 0">
        ✅ {{ dernierResultat.nb_retenus }} créneau(x) identifié(s) sur {{ dernierResultat.nb_analyses }} bougies analysées
      </span>
      <span v-else-if="dernierResultat.message">
        ⚠️ {{ dernierResultat.message }}
      </span>
      <span v-else>
        ⚠️ Aucun créneau retenu sur {{ dernierResultat.nb_analyses }} bougies —
        conviction insuffisante ou volatilité trop faible sur cet asset.
      </span>
    </div>

    <!-- Tableau des créneaux (composant dédié) -->
    <StraddleCreneauxTable
      :creneaux="creneaux"
      :asset="asset"
      :filtre-statut="filtreStatut"
      :chargement-liste="chargementListe"
      :chargement-precision="chargementPrecision"
      @update:filtre-statut="filtreStatut = $event"
      @changer-statut="changerStatut"
      @charger-precision="chargerPrecision"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import AppSelect from '@/components/common/AppSelect.vue'
import StraddleCreneauxTable from '@/components/common/StraddleCreneauxTable.vue'
import { apiService } from '@/services/api.service'
import type { StraddleCreneau } from '@/services/api.types'
import { useAssetsStore } from '@/stores/assets.store'
import { useAlerteStore } from '@/stores/alerte.store'

const assetsStore = useAssetsStore()
const alerteStore = useAlerteStore()

const assetsDisponibles = computed(() => {
  const liste = assetsStore.assets
  if (liste.length === 0) return ['XAUUSD', 'XAGUSD', 'EURUSD', 'GBPUSD', 'USDJPY', 'BTC', 'ETH']
  return liste.map(a => a.id)
})

const optionsAssets = computed(() =>
  assetsDisponibles.value.map(a => ({ label: a, value: a }))
)

const optionsPeriode = [
  { label: '3 mois', value: '3m' },
  { label: '6 mois', value: '6m' },
  { label: '1 an', value: '1a' },
  { label: '2 ans', value: '2a' },
]

const asset = ref('XAUUSD')
const periode = ref('6m')
const chargement = ref(false)
const chargementListe = ref(false)
const creneaux = ref<StraddleCreneau[]>([])
const dernierResultat = ref<{ nb_analyses: number; nb_retenus: number; message?: string } | null>(null)
const filtreStatut = ref<'tous' | 'a_tester' | 'valide' | 'invalide'>('tous')
const chargementPrecision = ref<Record<number, boolean>>({})

async function chargerPrecision(c: StraddleCreneau) {
  chargementPrecision.value[c.id] = true
  try {
    const r = await apiService.analyserPrecisionCreneau(c.id, {
      asset: c.asset,
      jour_semaine: c.jour_semaine,
      heure_debut: c.heure_debut,
      heure_fin: c.heure_fin,
    })

    if (r.timing_optimal) {
      const idx = creneaux.value.findIndex(x => x.id === c.id)
      if (idx !== -1) {
        creneaux.value[idx] = {
          ...creneaux.value[idx],
          timing_optimal: r.timing_optimal ?? null,
          fenetre_entree: r.fenetre_entree ?? null,
          whipsaw_minutes: r.whipsaw_minutes ?? null,
        }
      }
    } else if (r.message) {
      alerteStore.afficherErreur(r.message)
    }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Précision: ${(e as Error).message}`)
  } finally {
    chargementPrecision.value[c.id] = false
  }
}

async function chargerCreneaux() {
  chargementListe.value = true
  try {
    creneaux.value = await apiService.getStraddleCreneaux()
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Straddle: ${(e as Error).message}`)
  } finally {
    chargementListe.value = false
  }
}

async function analyser() {
  chargement.value = true
  dernierResultat.value = null
  try {
    const res = await apiService.analyserStraddle(asset.value, periode.value)
    creneaux.value = res.creneaux
    dernierResultat.value = {
      nb_analyses: res.nb_analyses,
      nb_retenus: res.nb_retenus,
      message: res.message,
    }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Analyse Straddle échouée: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

async function changerStatut(c: StraddleCreneau, statut: string) {
  try {
    await apiService.patchStraddleCreneau(c.id, { statut })
    const idx = creneaux.value.findIndex(x => x.id === c.id)
    if (idx !== -1) {
      creneaux.value[idx] = {
        ...creneaux.value[idx],
        statut: statut as StraddleCreneau['statut'],
      }
    }
  } catch (e: unknown) {
    alerteStore.afficherErreur(`Mise à jour échouée: ${(e as Error).message}`)
  }
}

onMounted(() => {
  void chargerCreneaux()
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-primary { @apply px-5 py-2 rounded-lg bg-yellow-600 hover:bg-yellow-500 text-white text-sm font-semibold transition-all; }
.btn-secondary { @apply px-4 py-2 rounded-lg bg-gray-700 hover:bg-gray-600 text-white text-sm transition-all; }
</style>
