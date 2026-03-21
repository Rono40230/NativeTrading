<template>
  <div class="glass-card p-6 space-y-6">
    <div class="flex items-center justify-between">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Assets actifs</h2>
      <span class="text-xs text-gray-500">{{ nbActifs }} / {{ tous.length }} activés</span>
    </div>

    <div v-for="cat in CATEGORIES" :key="cat.type" class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wider" :class="cat.couleur">
        {{ cat.label }}
      </p>
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
        <label
          v-for="a in cat.assets"
          :key="a.id"
          class="flex items-center gap-2 cursor-pointer rounded-lg border px-3 py-2 transition select-none"
          :class="estActif(a.id)
            ? 'border-emerald-500/40 bg-emerald-500/10'
            : 'border-white/20 bg-white/[0.03] hover:bg-white/[0.07] hover:border-white/30'"
        >
          <input
            type="checkbox"
            class="hidden"
            :checked="estActif(a.id)"
            :disabled="enCours === a.id"
            @change="basculer(a)"
          />
          <span class="w-3 h-3 rounded-sm border flex items-center justify-center shrink-0 transition"
            :class="estActif(a.id) ? 'bg-emerald-500 border-emerald-500' : 'border-white/30'">
            <svg v-if="estActif(a.id)" class="w-2.5 h-2.5 text-white" fill="currentColor" viewBox="0 0 12 12">
              <path d="M10 3L5 8.5 2 5.5" stroke="white" stroke-width="1.5" fill="none" stroke-linecap="round"/>
            </svg>
          </span>
          <div class="min-w-0">
            <p class="font-mono text-xs font-bold text-white truncate">{{ a.id }}</p>
            <p class="text-[10px] text-gray-400 truncate">{{ a.nom }}</p>
          </div>
          <span v-if="enCours === a.id" class="ml-auto text-[10px] text-gray-500">…</span>
        </label>
      </div>
    </div>

    <p v-if="erreur" class="text-red-400 text-xs">{{ erreur }}</p>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { AssetInfo } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'

const assetsStore = useAssetsStore()
const tous = ref<AssetInfo[]>([])
const enCours = ref<string | null>(null)
const erreur = ref('')

const nbActifs = computed(() => tous.value.filter(a => a.actif).length)

function estActif(id: string) {
  return tous.value.find(a => a.id === id)?.actif ?? false
}

const CATEGORIES = computed(() => [
  {
    type: 'crypto', label: '🪙 Crypto (Binance)', couleur: 'text-yellow-400',
    assets: tous.value.filter(a => a.type === 'crypto'),
  },
  {
    type: 'metal', label: '🥇 Métaux (IB)', couleur: 'text-amber-400',
    assets: tous.value.filter(a => a.type === 'metal'),
  },
  {
    type: 'forex', label: '💱 Forex (IB)', couleur: 'text-blue-400',
    assets: tous.value.filter(a => a.type === 'forex'),
  },
  {
    type: 'indice', label: '📈 Indices (IB)', couleur: 'text-purple-400',
    assets: tous.value.filter(a => a.type === 'indice'),
  },
])

async function basculer(a: AssetInfo) {
  enCours.value = a.id
  erreur.value = ''
  try {
    if (a.actif) {
      await apiService.supprimerAsset(a.id)
    } else {
      await apiService.ajouterAsset(a.id, a.nom, a.type as AssetInfo['type'], a.source ?? 'binance')
    }
    // Recharger la liste complète (actifs + inactifs) pour garder toutes les cartes visibles
    tous.value = await apiService.obtenirAssets(true)
    await assetsStore.chargerAssets()
  } catch (e: unknown) {
    erreur.value = (e as Error).message ?? 'Erreur'
  } finally {
    enCours.value = null
  }
}

onMounted(async () => {
  tous.value = await apiService.obtenirAssets(true)
})
</script>

<style scoped>
.glass-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.75rem;
  backdrop-filter: blur(12px);
}
</style>
