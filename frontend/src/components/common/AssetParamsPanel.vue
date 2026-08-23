<template>
  <div class="asset-params-panel">
    <!-- Entête -->
    <div class="panel-header">
      <h3 class="panel-title">Paramètres de sizing par asset</h3>
      <div class="header-actions">
        <button class="btn-save" :disabled="store.saving" @click="sauvegarder">
          {{ store.saving ? 'Sauvegarde…' : '💾 Sauvegarder' }}
        </button>
      </div>
    </div>

    <!-- Feedback -->
    <div v-if="feedback" :class="['feedback-bar', feedback.type]">
      {{ feedback.msg }}
    </div>

    <!-- Tableau -->
    <div class="table-wrapper">
      <table class="params-table">
        <thead>
          <tr>
            <th>Asset</th>
            <th title="Valeur décimale d'un pip (ex: 0.0001)">Taille pip</th>
            <th title="Valeur monétaire d'1 pip en USD">Valeur pip ($)</th>
            <th title="Stop-Loss par défaut en pips">SL pips</th>
            <th title="Facteur de conversion pip → point MT5">pip→pt</th>
            <th class="col-computed" title="SL en points MT5 = SL pips × pip→pt">SL points</th>
            <th class="col-computed" title="Capital × risque% / 100">Investi ($)</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="section in sections" :key="section.id">
            <!-- En-tête de section -->
            <tr :class="['section-header', `section-header-${section.id}`]">
              <td colspan="8" class="section-label">
                {{ section.icon }} {{ section.label }}
              </td>
            </tr>
            <!-- Lignes de la section -->
            <tr
              v-for="row in section.rows"
              :key="row.asset"
              :class="`row-${section.id}`"
            >
              <td class="cell-asset">{{ row.asset }}</td>
              <td>
                <input
                  v-model.number="row.taille_pip"
                  type="number"
                  min="0.00001"
                  step="0.00001"
                  class="input-cell"
                />
              </td>
              <td class="cell-computed">{{ Number(row.valeur_pips).toFixed(1) }}</td>
              <td>
                <input
                  v-model.number="row.sl_pips"
                  type="number"
                  min="1"
                  step="1"
                  class="input-cell"
                />
              </td>
              <td>
                <input
                  v-model.number="row.pip_to_points"
                  type="number"
                  min="1"
                  step="10"
                  class="input-cell"
                />
              </td>
              <td class="cell-computed">
                {{ Math.round(row.sl_pips * row.pip_to_points) }}
              </td>
            </tr>
          </template>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted } from 'vue'
import { useAssetParamsStore } from '@/stores/assetParams.store'
import { useSettingsStore } from '@/stores/settings.store'
import type { AssetParams } from '@/services/api.types'

const store = useAssetParamsStore()
const settingsStore = useSettingsStore()
const editable   = reactive<AssetParams[]>([])

interface Feedback { type: 'ok' | 'err'; msg: string }
const feedback = ref<Feedback | null>(null)

// Catégorie de chaque asset
const ASSET_CATEGORY: Record<string, string> = {
  XAUUSD: 'metaux', XAGUSD: 'metaux',
  EURUSD: 'forex',  GBPUSD: 'forex',  USDCAD: 'forex',  AUDUSD: 'forex',
  EURGBP: 'forex',  GBPJPY: 'forex',  EURJPY: 'forex',  USDJPY: 'forex',
  CADJPY: 'forex',  NZDJPY: 'forex',  CHFJPY: 'forex',
  DAX:    'indices', SP500: 'indices',
  BTC:    'crypto',  ETH: 'crypto',
}

const SECTION_META = [
  { id: 'metaux',  label: 'Métaux',  icon: '🪙' },
  { id: 'forex',   label: 'Forex',   icon: '💱' },
  { id: 'indices', label: 'Indices', icon: '📈' },
  { id: 'crypto',  label: 'Crypto',  icon: '🔷' },
]

const sections = computed(() =>
  SECTION_META
    .map(meta => ({
      ...meta,
      rows: editable.filter(r => (ASSET_CATEGORY[r.asset] ?? 'forex') === meta.id),
    }))
    .filter(s => s.rows.length > 0)
)

onMounted(async () => {
  await store.charger()
  syncDepuisStore()
})

watch(() => store.liste, () => { syncDepuisStore() })

function syncDepuisStore() {
  editable.splice(0, editable.length, ...store.liste.map(p => ({ ...p })))
}



async function sauvegarder() {
  feedback.value = null
  const ok = await store.sauvegarder([...editable])
  feedback.value = ok
    ? { type: 'ok',  msg: '✅ Paramètres sauvegardés.' }
    : { type: 'err', msg: '❌ Échec de la sauvegarde.' }
  setTimeout(() => { feedback.value = null }, 3000)
}
</script>

<style src="./AssetParamsPanel.css" />
