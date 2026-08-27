<template>
  <div class="glass-card flex flex-col p-2 min-h-0 shrink-0">
    <div class="flex items-center justify-between shrink-0 mb-1 border-b border-white/10 pb-1">
      <span class="text-xs uppercase font-bold text-white">⚙️ Data & IA Engine</span>
      <span class="text-[9px] text-emerald-400">runtime v12 · signaux officiels</span>
    </div>
    
    <div class="grid grid-cols-2 gap-x-2 gap-y-1.5 flex-1 overflow-y-auto">
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-gray-500 text-[9px] uppercase">API Serveur</span>
        <span :class="backendOk ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold">
          {{ backendOk ? '🟢 Actif' : '🔴 Erreur' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-gray-500 text-[9px] uppercase">Crypto (Binance)</span>
        <span :class="btcPrix ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold">
          {{ btcPrix ? '🟢 Connecté' : '🔴 Erreur' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-gray-500 text-[9px] uppercase">LLM (Ollama)</span>
        <span v-if="ollamaOk === null" class="text-gray-500 text-[10px] font-semibold animate-pulse">⏳ Vérif</span>
        <span v-else :class="ollamaOk ? 'text-emerald-400' : 'text-red-400'" class="text-[10px] font-semibold">
          {{ ollamaOk ? '🟢 Local' : '🔴 Hors ligne' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-gray-500 text-[9px] uppercase">Inférence ML</span>
        <span :class="mlPret ? 'text-emerald-400' : 'text-yellow-400'" class="text-[10px] font-semibold">
          {{ mlPret ? '🟢 LSTM/XGB' : '🟡 Prépa' }}
        </span>
      </div>
      <div class="flex items-center justify-between bg-white/5 rounded px-1.5 py-0.5 shrink-0">
        <span class="text-gray-500 text-[9px] uppercase">Tick Engine</span>
        <span class="text-emerald-400 text-[10px] font-semibold truncate text-right">🟢 Runtime v12</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  backendOk: boolean
  btcPrix: number | null
  ollamaOk: boolean | null
  mlPret: boolean
}>()
</script>
