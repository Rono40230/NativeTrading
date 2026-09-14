<template>
  <!-- Métriques ML par stratégie — onglet de la page Fonctionnalités IA
       (14/09, ex-route /ia/ml) : hauteur portée par le parent. -->
  <div class="flex flex-col gap-3 h-full">
    <div class="shrink-0 px-1">
      <h1 class="text-xl font-bold text-white">📉 Métriques ML</h1>
      <p class="text-sm text-white/80">État des réseaux par stratégie.</p>
    </div>
    <div class="flex-1 min-h-0 overflow-y-auto pr-1">
      <div class="grid grid-cols-1 xl:grid-cols-3 gap-4 h-full">
        <!-- Straddle -->
        <div class="glass-card flex flex-col rounded-xl border border-purple-500/30 bg-white/5 overflow-hidden">
          <div class="p-4 border-b border-white/10 shrink-0">
            <h2 class="font-bold flex items-center gap-2 text-base text-purple-400"><span>⚡</span> Straddle</h2>
          </div>
          <div class="flex-1 min-h-0 overflow-y-auto p-4 relative">
            <StraddleMonitoringML compact />
          </div>
        </div>
        <!-- SMC -->
        <div class="glass-card flex flex-col rounded-xl border border-blue-500/30 bg-white/5 overflow-hidden">
          <div class="p-4 border-b border-white/10 shrink-0">
            <h2 class="font-bold flex items-center gap-2 text-base text-blue-400"><span>📊</span> SMC</h2>
          </div>
          <div class="flex-1 min-h-0 overflow-y-auto p-4 relative">
            <SmcMonitoringML compact />
          </div>
        </div>
        <!-- Rockets -->
        <div class="glass-card flex flex-col rounded-xl border border-orange-500/30 bg-white/5 overflow-hidden">
          <div class="p-4 border-b border-white/10 shrink-0">
            <h2 class="font-bold flex items-center gap-2 text-base text-orange-400"><span>🚀</span> Rockets</h2>
          </div>
          <div class="flex-1 min-h-0 overflow-y-auto p-4 relative">
            <RocketsMonitoringML compact />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import StraddleMonitoringML from '@/components/common/StraddleMonitoringML.vue'
import RocketsMonitoringML from '@/components/common/RocketsMonitoringML.vue'
import SmcMonitoringML from '@/components/common/SmcMonitoringML.vue'
</script>
