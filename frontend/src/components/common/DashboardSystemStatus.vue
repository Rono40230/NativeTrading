<template>
  <div class="glass-card p-5 flex-1">
    <div class="flex gap-2 items-stretch">
      <!-- Tiles fixes -->
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5 shrink-0">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">Backend API</span>
        <span :class="backendOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">
          {{ backendOk ? '🟢 Online' : '🔴 Offline' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5 shrink-0">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">Binance Feed</span>
        <span :class="btcPrix ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">
          {{ btcPrix ? '🟢 Connecté' : '🔴 Offline' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5 shrink-0">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">IG Markets</span>
        <span v-if="igOk === null" class="text-gray-500 text-sm font-semibold animate-pulse">⏳ Vérif…</span>
        <span v-else :class="igOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">
          {{ igOk ? '🟢 Connecté' : '🔴 Déconnecté' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5 shrink-0">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">Ollama IA</span>
        <span v-if="ollamaOk === null" class="text-gray-500 text-sm font-semibold animate-pulse">⏳ Vérif…</span>
        <span v-else :class="ollamaOk ? 'text-emerald-400' : 'text-red-400'" class="text-sm font-semibold">
          {{ ollamaOk ? '🟢 Connecté' : '🔴 Hors ligne' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5 shrink-0">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">ML Engine</span>
        <span :class="mlPret ? 'text-emerald-400' : 'text-yellow-400'" class="text-sm font-semibold">
          {{ mlPret ? '🟢 Prêt' : '🟡 Non entraîné' }}
        </span>
      </div>
      <div class="rounded-lg bg-white/5 px-3 py-2 flex flex-col gap-0.5 shrink-0">
        <span class="text-gray-500 text-[10px] uppercase tracking-wider">Base de données</span>
        <span class="text-emerald-400 text-sm font-semibold">🟢 SQLite</span>
      </div>

      <!-- Signal Engine — tile élargie, prend l'espace restant -->
      <div class="rounded-lg bg-white/5 px-3 py-2 flex items-center justify-between gap-3 flex-1 min-w-0">
        <div class="flex items-center gap-2 min-w-0">
          <span class="text-base shrink-0">{{ engineActif ? '🟢' : '🔴' }}</span>
          <div class="min-w-0">
            <p class="text-[10px] uppercase tracking-wider text-gray-500">Signal Engine</p>
            <p class="text-sm font-semibold text-white truncate">{{ engineActif ? 'Actif' : 'Arrêté' }}</p>
            <p class="text-[10px] text-gray-400 truncate">
              <template v-if="engineActif && engineSecondes > 0">dans {{ engineSecondes }}s</template>
              <template v-else-if="engineActif">Analyse…</template>
              <template v-else>5 min — 13 assets</template>
            </p>
          </div>
        </div>
        <div class="flex items-center gap-2 shrink-0">
          <span class="text-[10px] text-gray-400">{{ engineSignaux24h }}/24h</span>
          <button
            v-if="!engineActif"
            class="px-2 py-1 rounded-lg bg-emerald-500/20 text-emerald-400 text-[10px] font-semibold hover:bg-emerald-500/30 transition disabled:opacity-50"
            :disabled="engineChargement"
            @click="$emit('engine-demarrer')"
          >Start</button>
          <button
            v-else
            class="px-2 py-1 rounded-lg bg-red-500/20 text-red-400 text-[10px] font-semibold hover:bg-red-500/30 transition disabled:opacity-50"
            :disabled="engineChargement"
            @click="$emit('engine-arreter')"
          >Stop</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  backendOk: boolean
  btcPrix: number | null
  igOk: boolean | null
  ollamaOk: boolean | null
  mlPret: boolean
  engineActif: boolean
  engineSecondes: number
  engineSignaux24h: number
  engineChargement: boolean
}>()

defineEmits<{
  (e: 'engine-demarrer'): void
  (e: 'engine-arreter'): void
}>()
</script>
