<template>
  <div class="glass-card p-4">
    <div class="flex items-center justify-between mb-3">
      <h2 class="text-xs uppercase font-bold text-white">Assets actifs</h2>
      <span class="text-xs text-gray-500">{{ nbActifs }} / {{ tous.length }} activés</span>
    </div>

    <!-- 4 colonnes côte à côte : Crypto | Métaux | Forex | Indices -->
    <div class="grid grid-cols-4 gap-3">
      <div v-for="cat in CATEGORIES" :key="cat.type">
        <p class="text-[11px] font-semibold uppercase tracking-wider mb-1.5" :class="cat.couleur">
          {{ cat.label }}
        </p>
        <div class="space-y-0.5">
          <label
            v-for="a in cat.assets"
            :key="a.id"
            class="flex items-center gap-1.5 cursor-pointer rounded border px-2 py-1 transition select-none"
            :class="estActif(a.id)
              ? 'border-emerald-500/40 bg-emerald-500/10'
              : 'border-white/10 bg-white/[0.02] hover:bg-white/[0.06] hover:border-white/20'"
          >
            <input
              type="checkbox"
              class="hidden"
              :checked="estActif(a.id)"
              :disabled="enCours === a.id"
              @change="basculer(a)"
            />
            <span
              class="w-2.5 h-2.5 rounded-sm border flex items-center justify-center shrink-0 transition"
              :class="estActif(a.id) ? 'bg-emerald-500 border-emerald-500' : 'border-white/30'"
            >
              <svg v-if="estActif(a.id)" class="w-2 h-2 text-white" fill="none" viewBox="0 0 12 12">
                <path d="M10 3L5 8.5 2 5.5" stroke="white" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            </span>
            <span class="font-mono text-[22px] font-bold text-white truncate leading-tight">{{ a.id }}</span>
            <span v-if="enCours === a.id" class="ml-auto text-[10px] text-gray-500">…</span>
          </label>
        </div>
      </div>
    </div>

    <p v-if="erreur" class="text-red-400 text-xs mt-2">{{ erreur }}</p>
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
    type: 'metal', label: '🥇 Métaux', couleur: 'text-amber-400',
    assets: tous.value.filter(a => a.type === 'metal'),
  },
  {
    type: 'forex', label: '💱 Forex', couleur: 'text-blue-400',
    assets: tous.value.filter(a => a.type === 'forex'),
  },
  {
    type: 'indice', label: '📈 Indices', couleur: 'text-purple-400',
    assets: tous.value.filter(a => a.type === 'indice'),
  },
])

async function basculer(a: AssetInfo) {
  enCours.value = a.id
  erreur.value = ''
  const ancienEtat = a.actif
  try {
    if (a.actif) {
      await apiService.supprimerAsset(a.id)
    } else {
      await apiService.ajouterAsset(a.id, a.nom, a.type as AssetInfo['type'], 'binance')
    }
    // Muter directement l'objet dans tous.value — pas de re-fetch, la carte reste toujours visible
    a.actif = !ancienEtat
    await assetsStore.chargerAssets()
  } catch (e: unknown) {
    erreur.value = (e as Error).message ?? 'Erreur'
    // Pas de mutation en cas d'erreur — état visuel inchangé
  } finally {
    enCours.value = null
  }
}

onMounted(async () => {
  tous.value = await apiService.obtenirAssets()
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
