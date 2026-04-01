<template>
  <nav class="bg-gray-800 border-b border-gray-700 py-4">
    <div class="container mx-auto px-4 flex items-center justify-between">
      <h1 class="text-2xl font-bold text-blue-500">📊 Native Trading AI</h1>
      
      <div class="flex space-x-6">
        <RouterLink to="/" class="text-gray-300 hover:text-white">🏠 Home</RouterLink>
        <RouterLink to="/charts" class="text-gray-300 hover:text-white">📈 Charts</RouterLink>
        <RouterLink to="/pnl" class="text-gray-300 hover:text-white">⚡ Backtest heure précise</RouterLink>
        <RouterLink to="/history" class="text-gray-300 hover:text-white">📜 History</RouterLink>
        <RouterLink to="/heatmap" class="text-gray-300 hover:text-white">🔥 Heatmap</RouterLink>
        <RouterLink to="/ia/analyser" class="text-gray-300 hover:text-white">🧠 IA Analyse</RouterLink>
        <RouterLink to="/ia/coach" class="text-gray-300 hover:text-white">💬 IA Coach</RouterLink>
        <RouterLink to="/settings" class="text-gray-300 hover:text-white">⚙️ Settings</RouterLink>
      </div>
    </div>
  </nav>
</template>

<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>
