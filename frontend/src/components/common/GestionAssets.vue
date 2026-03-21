<template>
  <div class="glass-card p-6 space-y-5">
    <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Assets gérés</h2>

    <!-- Liste des assets actifs -->
    <div class="space-y-2">
      <div
        v-for="a in assetsStore.assets"
        :key="a.id"
        class="flex items-center justify-between rounded-lg bg-white/5 border border-white/10 px-3 py-2"
      >
        <div class="flex items-center gap-3">
          <span class="font-mono font-semibold text-white text-sm">{{ a.id }}</span>
          <span class="text-gray-400 text-sm">{{ a.nom }}</span>
          <span :class="COULEUR_TYPE[a.type]" class="text-[10px] uppercase font-semibold tracking-wide px-1.5 py-0.5 rounded">
            {{ a.type }}
          </span>
          <span class="text-[10px] text-gray-500 uppercase">{{ a.source }}</span>
        </div>
        <button
          class="text-red-400 hover:text-red-300 transition-colors text-xs px-2 py-1 rounded hover:bg-red-900/20"
          :disabled="suppressionEnCours === a.id"
          @click="demanderSuppression(a.id)"
        >
          {{ suppressionEnCours === a.id ? '…' : '✕ Retirer' }}
        </button>
      </div>
      <p v-if="!assetsStore.assets.length" class="text-gray-500 text-sm">Aucun asset configuré.</p>
    </div>

    <!-- Formulaire ajout -->
    <div class="border-t border-white/10 pt-4 space-y-3">
      <p class="text-xs text-gray-400 font-medium">Ajouter un asset</p>
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <input
          v-model="form.id"
          type="text"
          placeholder="Ticker (ex: AAPL)"
          maxlength="20"
          class="input-field uppercase"
          @input="form.id = form.id.toUpperCase()"
        />
        <input
          v-model="form.nom"
          type="text"
          placeholder="Nom (ex: Apple)"
          maxlength="60"
          class="input-field"
        />
        <select v-model="form.type" class="input-field">
          <option value="" disabled>Type</option>
          <option value="crypto">Crypto</option>
          <option value="metal">Métal</option>
          <option value="forex">Forex</option>
          <option value="indice">Indice</option>
        </select>
        <select v-model="form.source" class="input-field">
          <option value="binance">Binance</option>
          <option value="ib">IB Gateway</option>
        </select>
      </div>
      <div v-if="erreurAjout" class="text-red-400 text-xs">{{ erreurAjout }}</div>
      <button
        class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 rounded text-sm font-medium transition-colors"
        :disabled="ajoutEnCours || !formValide"
        @click="ajouter"
      >
        {{ ajoutEnCours ? 'Ajout…' : '＋ Ajouter' }}
      </button>
    </div>

    <!-- Modal confirmation suppression -->
    <Teleport v-if="confirmId" to="body">
      <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
        <div class="bg-gray-900 border border-white/10 rounded-xl p-6 space-y-4 max-w-sm w-full mx-4">
          <p class="text-white font-semibold">Retirer l'asset <span class="text-red-400">{{ confirmId }}</span> ?</p>
          <p class="text-gray-400 text-sm">L'historique des bougies est conservé. L'asset disparaîtra de toute l'application.</p>
          <div class="flex gap-3">
            <button
              class="flex-1 px-4 py-2 bg-red-600 hover:bg-red-500 rounded text-sm font-medium"
              @click="confirmerSuppression"
            >Confirmer</button>
            <button
              class="flex-1 px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded text-sm"
              @click="confirmId = null"
            >Annuler</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useAssetsStore } from '@/stores/assets.store'
import type { AssetInfo } from '@/services/api.service'

const assetsStore = useAssetsStore()

const COULEUR_TYPE: Record<AssetInfo['type'], string> = {
  crypto: 'text-yellow-400 bg-yellow-400/10',
  metal:  'text-amber-400 bg-amber-400/10',
  forex:  'text-blue-400 bg-blue-400/10',
  indice: 'text-purple-400 bg-purple-400/10',
}

const form = ref({ id: '', nom: '', type: '' as AssetInfo['type'] | '', source: 'binance' as 'binance' | 'ib' })
const ajoutEnCours = ref(false)
const erreurAjout = ref('')
const confirmId = ref<string | null>(null)
const suppressionEnCours = ref<string | null>(null)

const formValide = computed(
  () => form.value.id.length >= 2 && form.value.nom.trim().length > 0 && form.value.type !== '',
)

function demanderSuppression(id: string) {
  confirmId.value = id
}

async function confirmerSuppression() {
  if (!confirmId.value) return
  suppressionEnCours.value = confirmId.value
  confirmId.value = null
  try {
    await assetsStore.supprimerAsset(suppressionEnCours.value)
  } finally {
    suppressionEnCours.value = null
  }
}

async function ajouter() {
  if (!formValide.value || form.value.type === '') return
  ajoutEnCours.value = true
  erreurAjout.value = ''
  try {
    await assetsStore.ajouterAsset(form.value.id, form.value.nom.trim(), form.value.type, form.value.source)
    form.value = { id: '', nom: '', type: '', source: 'binance' }
  } catch (e: unknown) {
    erreurAjout.value = (e as Error).message ?? 'Erreur lors de l\'ajout.'
  } finally {
    ajoutEnCours.value = false
  }
}
</script>

<style scoped>
.input-field {
  @apply bg-gray-800 text-white text-sm rounded-lg px-3 py-2 border border-white/10
         focus:outline-none focus:ring-2 focus:ring-blue-500 w-full;
}
</style>
