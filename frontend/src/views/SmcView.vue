<template>
  <div class="flex flex-col gap-4">
    <!-- En-tête + contrôle Signal Engine -->
    <DashboardSystemStatus
      :backend-ok="backendOk"
      :btc-prix="btcPrix"
      :ib-gateway-ok="false"
      :ml-pret="false"
      :engine-actif="engineActif"
      :engine-secondes="engineSecondes"
      :engine-signaux24h="engineSignaux24h"
      :engine-chargement="engineChargement"
      @engine-demarrer="engineDemarrer"
      @engine-arreter="engineArreter"
    />

    <!-- Analyse SMC manuelle -->
    <div class="glass-card p-4">
      <p class="text-xs font-semibold uppercase tracking-widest text-gray-400 mb-3">📊 Analyse manuelle</p>
      <SMCAnalyzerView embedded />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import DashboardSystemStatus from '@/components/common/DashboardSystemStatus.vue'
import SMCAnalyzerView from '@/views/SMCAnalyzerView.vue'
import { useSignalEngine } from '@/composables/useSignalEngine'
import { apiService } from '@/services/api.service'

const backendOk = ref(false)
const btcPrix = ref<number | null>(null)

const {
  actif: engineActif,
  secondesRestantes: engineSecondes,
  signaux24h: engineSignaux24h,
  chargement: engineChargement,
  demarrer: engineDemarrer,
  arreter: engineArreter,
} = useSignalEngine()

onMounted(async () => {
  try {
    await apiService.healthCheck()
    backendOk.value = true
    const p = await apiService.getPrixActuel('BTC')
    btcPrix.value = p
  } catch { backendOk.value = false }
})
</script>
