<template>
  <div class="glass-card p-5 flex-1">
    <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-3">Statut système</h2>
    <div class="grid grid-cols-3 gap-2">
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">Backend API</span>
        <span :class="backendOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">
          {{ backendOk ? '🟢 Online' : '🔴 Offline' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">Binance Feed</span>
        <span :class="btcPrix ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">
          {{ btcPrix ? '🟢 Connecté' : '🔴 Offline' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">IB Gateway</span>
        <span v-if="ibGatewayOk === null" class="text-gray-500 text-sm font-semibold animate-pulse">⏳ Vérif…</span>
        <span v-else :class="ibGatewayOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">
          {{ ibGatewayOk ? '🟢 Connecté' : '🔴 Déconnecté' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">ML Engine</span>
        <span :class="mlPret ? 'text-emerald-400' : 'text-yellow-400'" class="text-sm font-semibold">
          {{ mlPret ? '🟢 Prêt' : '🟡 Non entraîné' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">Base de données</span>
        <span class="text-emerald-400 text-sm font-semibold">🟢 SQLite</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  backendOk: boolean
  btcPrix: number | null
  ibGatewayOk: boolean | null
  mlPret: boolean
}>()
</script>
