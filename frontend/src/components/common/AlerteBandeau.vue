<template>
  <!-- Bandeau visible uniquement si score_max ≥ 80 -->
  <Transition name="alerte-slide">
    <div
      v-if="newsStore.alerteCritique && article"
      class="flex items-center gap-3 rounded-xl border border-red-500/40 bg-red-900/30 px-4 py-2.5 backdrop-blur-sm"
    >
      <!-- Indicateur clignotant -->
      <span class="relative flex h-2.5 w-2.5 shrink-0">
        <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-400 opacity-75" />
        <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-red-500" />
      </span>

      <!-- Texte -->
      <div class="flex min-w-0 flex-1 items-baseline gap-2">
        <span class="shrink-0 text-[10px] font-bold uppercase tracking-widest text-red-400">
          {{ article.source }}
        </span>
        <span class="truncate text-xs text-red-100">{{ article.titre }}</span>
      </div>

      <!-- Score + heure -->
      <div class="flex shrink-0 items-center gap-2 text-[10px] text-red-300">
        <span class="font-bold">{{ article.score }}/100</span>
        <span>{{ tempsRelatif(article.date) }}</span>
      </div>

      <!-- Lien externe -->
      <a
        v-if="article.url"
        :href="article.url"
        target="_blank"
        rel="noopener noreferrer"
        class="shrink-0 text-[10px] text-red-400 underline hover:text-red-200"
      >
        Lire →
      </a>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { useNewsStore } from '@/stores/news.store'

const newsStore = useNewsStore()

const article = newsStore.articleCritique

function tempsRelatif(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  const min = Math.floor(diff / 60_000)
  if (min < 1) return 'à l\'instant'
  if (min < 60) return `il y a ${min} min`
  const h = Math.floor(min / 60)
  return `il y a ${h}h`
}
</script>

<style scoped>
.alerte-slide-enter-active,
.alerte-slide-leave-active {
  transition: all 0.3s ease;
}
.alerte-slide-enter-from,
.alerte-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
