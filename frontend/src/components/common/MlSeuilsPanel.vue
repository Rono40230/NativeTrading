<template>
  <div class="space-y-3 flex flex-col h-full">
    <div v-if="chargement" class="p-4 text-center text-gray-400 text-xs h-full flex items-center justify-center">
      Chargement…
    </div>
    <div v-else class="space-y-2 flex-1 flex flex-col justify-between">

      <!-- Rockets -->
      <div class="bg-black/20 rounded-lg border border-white/10 p-3 space-y-2 border-l-2 !border-l-orange-500">
        <div class="flex items-center justify-between">
          <div class="flex flex-col">
            <span class="font-bold text-orange-200 text-sm flex items-center gap-1"><span>🚀</span> Rockets</span>
          </div>
          <span class="text-emerald-400 font-bold">{{ (seuils.rockets * 100).toFixed(0) }}%</span>
        </div>
        <input
          type="range" min="0.30" max="0.90" step="0.05"
          v-model.number="seuils.rockets"
          @change="enregistrer"
          class="w-full accent-emerald-500 h-1 cursor-pointer"
        />
        <div class="flex justify-between text-[9px] text-gray-500">
          <span>30% permissif</span>
          <span>90% strict</span>
        </div>
      </div>

      <!-- Straddle -->
      <div class="bg-black/20 rounded-lg border border-white/10 p-3 space-y-2 border-l-2 !border-l-purple-500">
        <div class="flex items-center justify-between">
          <div class="flex flex-col">
            <span class="font-bold text-purple-200 text-sm flex items-center gap-1"><span>⚡</span> Straddle</span>
          </div>
          <span class="text-blue-400 font-bold">{{ (seuils.straddle * 100).toFixed(0) }}%</span>
        </div>
        <input
          type="range" min="0.50" max="0.95" step="0.05"
          v-model.number="seuils.straddle"
          @change="enregistrer"
          class="w-full accent-blue-500 h-1 cursor-pointer"
        />
        <div class="flex justify-between text-[9px] text-gray-500">
          <span>50% sélectif</span>
          <span>95% permissif</span>
        </div>
      </div>

      <!-- SMC -->
      <div class="bg-black/20 rounded-lg border border-white/10 p-3 space-y-2 border-l-2 !border-l-blue-500">
        <div class="flex items-center justify-between">
          <div class="flex flex-col">
            <span class="font-bold text-blue-200 text-sm flex items-center gap-1"><span>△</span> SMC</span>
          </div>
          <span class="text-violet-400 font-bold">{{ (seuils.smc * 100).toFixed(0) }}%</span>
        </div>
        <input
          type="range" min="0.30" max="0.90" step="0.05"
          v-model.number="seuils.smc"
          @change="enregistrer"
          class="w-full accent-violet-500 h-1 cursor-pointer"
        />
        <div class="flex justify-between text-[9px] text-gray-500">
          <span>30% permissif</span>
          <span>90% strict</span>
        </div>
      </div>

      <!-- Statut enregistrement auto -->
      <div class="pt-1 flex items-center justify-end h-4">
        <transition name="fade">
          <span v-if="message" class="text-[10px] font-bold" :class="messageOk ? 'text-emerald-400' : 'text-red-400'">
            {{ sauvegarde ? '⏳ Enregistrement...' : message }}
          </span>
        </transition>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'

const chargement = ref(true)
const sauvegarde = ref(false)
const message = ref('')
const messageOk = ref(true)

const seuils = ref({ rockets: 0.60, straddle: 0.75, smc: 0.60 })

async function chargerSeuil(cle: string): Promise<number | null> {
  try {
    const data = await apiService.obtenirConfig(cle)
    const val = parseFloat(data?.valeur ?? '')
    return isNaN(val) ? null : val
  } catch {
    return null
  }
}

async function chargerSeuils() {
  chargement.value = true
  const [r, s, m] = await Promise.all([
    chargerSeuil('seuil_confiance_rockets'),
    chargerSeuil('seuil_confiance_straddle'),
    chargerSeuil('seuil_confiance_smc'),
  ])
  if (r !== null) seuils.value.rockets = r
  if (s !== null) seuils.value.straddle = s
  if (m !== null) seuils.value.smc = m
  chargement.value = false
}

onMounted(() => {
  chargerSeuils()
})

function forcerSeuil(strategie: string, valeurEntiere: number) {
  let r = false
  if (strategie === 'ROCKETS')      { seuils.value.rockets = valeurEntiere / 100; r = true }
  else if (strategie === 'STRADDLE') { seuils.value.straddle = valeurEntiere / 100; r = true }
  else if (strategie === 'SMC')      { seuils.value.smc = valeurEntiere / 100; r = true }
  
  if (r) {
    enregistrer()
  }
}

defineExpose({ chargerSeuils, forcerSeuil })

async function enregistrer() {
  sauvegarde.value = true
  message.value = ''
  try {
    await Promise.all([
      apiService.sauvegarderConfig('seuil_confiance_rockets', String(seuils.value.rockets)),
      apiService.sauvegarderConfig('seuil_confiance_straddle', String(seuils.value.straddle)),
      apiService.sauvegarderConfig('seuil_confiance_smc', String(seuils.value.smc)),
    ])
    messageOk.value = true
    message.value = '✅ Enregistré'
    setTimeout(() => { message.value = '' }, 3000)
  } catch {
    messageOk.value = false
    message.value = '❌ Échec'
  } finally {
    sauvegarde.value = false
  }
}
</script>

<style scoped>
.glass-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.75rem;
  backdrop-filter: blur(12px);
}
</style>
