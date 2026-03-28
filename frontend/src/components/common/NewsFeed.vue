<template>
  <div v-bind="$attrs" class="glass-card p-4 flex flex-col h-full">
    <!-- En-tête -->
    <div class="mb-3 flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold uppercase tracking-widest text-white">
        Revue de Presse
      </p>
      <div v-if="newsStore.chargement" class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
    </div>

    <!-- D.1 — Jauge de risque news globale -->
    <div class="mb-3 shrink-0">
      <div class="flex items-center justify-between mb-1">
        <span class="text-[10px] text-slate-400">Risque news</span>
        <span class="text-[10px] font-bold" :class="classeRisque(newsStore.scoreMax)">
          {{ labelRisque(newsStore.scoreMax) }} · {{ newsStore.scoreMax }}/100
        </span>
      </div>
      <div class="h-1.5 w-full rounded-full bg-white/10 overflow-hidden">
        <div
          class="h-full rounded-full transition-all duration-700"
          :style="{ width: `${newsStore.scoreMax}%` }"
          :class="gaugeClasse(newsStore.scoreMax)"
        />
      </div>
    </div>

    <!-- Barre Fear & Greed + prochain événement macro -->
    <NewsAuxBar />

    <!-- Onglets thème -->
    <div class="mb-2 flex gap-1 shrink-0">
      <button
        v-for="tab in ONGLETS"
        :key="tab.id"
        class="flex-1 rounded-md border px-1.5 py-1 text-[9px] font-semibold uppercase tracking-wide transition-colors"
        :class="newsStore.themeActif === tab.id
          ? 'border-blue-500/40 bg-blue-500/15 text-blue-300'
          : 'border-white/5 bg-white/5 text-slate-500 hover:text-slate-300'"
        @click="newsStore.themeActif = tab.id"
      >
        {{ tab.label }}
        <span
          v-if="newsStore.scoreMaxParTheme[tab.id] >= 60"
          class="ml-0.5 text-[8px]"
          :class="newsStore.scoreMaxParTheme[tab.id] >= 80 ? 'text-red-400' : 'text-orange-400'"
        >●</span>
      </button>
    </div>

    <!-- Liste articles -->
    <div v-if="newsStore.articlesPertinents.length > 0" class="space-y-2 overflow-y-auto scroll-zone flex-1 pr-0.5">
      <button
        v-for="article in newsStore.articlesPertinents"
        :key="article.id"
        class="group w-full text-left rounded-lg border border-white/5 p-2 transition hover:bg-white/10 cursor-pointer"
        :class="estAncien(article.date) ? 'bg-white/3 opacity-50' : 'bg-white/5'"
        @click="ouvrir(article)"
      >
        <div class="mb-0.5 flex items-center gap-1.5">
          <span class="h-1.5 w-1.5 shrink-0 rounded-full" :class="couleurNiveau(article.niveau)" />
          <span class="text-[10px] text-slate-400">{{ article.source }}</span>
          <!-- D.3 — Badge NOUVEAU < 30min -->
          <span
            v-if="estNouveau(article.date)"
            class="text-[9px] font-bold text-red-300 bg-red-600/40 border border-red-500/40 rounded-full px-1.5 py-0.5 leading-none animate-pulse"
          >NOUVEAU</span>
          <span v-else-if="articlesLus.has(article.url)" class="text-[9px] font-semibold text-blue-200 bg-blue-600/70 border border-blue-500/50 rounded-full px-1.5 py-0.5 leading-none">Vu</span>
          <span class="ml-auto text-[10px] font-semibold" :class="classeRisque(article.score)">
            {{ article.score }}
          </span>
        </div>
        <p class="line-clamp-2 text-[11px] leading-snug text-slate-200 group-hover:text-white">
          <!-- C.2 — Flèche sentiment Ollama -->
          <span
            v-if="article.sentiment"
            class="mr-1 font-bold"
            :class="article.sentiment === 'haussier' ? 'text-emerald-400' : article.sentiment === 'baissier' ? 'text-red-400' : 'text-slate-400'"
            :title="`Sentiment IA : ${article.sentiment}`"
          >{{ article.sentiment === 'haussier' ? '↑' : article.sentiment === 'baissier' ? '↓' : '↔' }}</span>{{ article.titre_fr ?? article.titre }}
        </p>
        <!-- D.3 — Temps relatif enrichi -->
        <p class="mt-0.5 text-[10px]" :class="classeTemps(article.date)">{{ tempsRelatif(article.date) }}</p>
      </button>
    </div>

    <!-- Skeleton -->
    <div v-else-if="newsStore.chargement" class="space-y-2 animate-pulse overflow-y-auto scroll-zone flex-1">
      <div v-for="i in 5" :key="i" class="rounded-lg bg-white/5 p-2 space-y-1">
        <div class="h-2 w-24 rounded bg-white/10" />
        <div class="h-3 rounded bg-white/10" :style="{ width: `${70 + (i % 3) * 10}%` }" />
      </div>
    </div>

    <p v-else class="text-center text-[10px] text-slate-500">Aucune actualité filtrée</p>
  </div>

  <!-- Modale article (composant dédié) -->
  <NewsArticleModal :article="articleOuvert" @fermer="fermer" />
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false })
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import { useNewsStore } from '@/stores/news.store'
import type { ThemeNews } from '@/stores/news.store'
import NewsAuxBar from '@/components/common/NewsAuxBar.vue'
import NewsArticleModal from '@/components/common/NewsArticleModal.vue'
import type { ArticleNews, NiveauAlerte } from '@/services/api.types'

const ONGLETS: { id: ThemeNews; label: string }[] = [
  { id: 'tous', label: 'Tout' },
  { id: 'macro', label: 'Macro' },
  { id: 'crypto', label: 'Crypto' },
  { id: 'metaux', label: 'Métaux' },
]

const newsStore = useNewsStore()
const articlesLus = ref<Set<string>>(new Set())
const articleOuvert = ref<ArticleNews | null>(null)

onMounted(async () => {
  const urls = await apiService.obtenirArticlesLus()
  articlesLus.value = new Set(urls)
})

function marquerLu(url: string) {
  articlesLus.value.add(url)
  apiService.marquerArticleLu(url)
}

function ouvrir(article: ArticleNews) {
  marquerLu(article.url)
  articleOuvert.value = article
}

function fermer() {
  articleOuvert.value = null
}

function tempsRelatif(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  const min = Math.floor(diff / 60_000)
  if (min < 1) return 'à l\'instant'
  if (min < 60) return `il y a ${min} min`
  const h = Math.floor(min / 60)
  if (h < 24) return `il y a ${h}h`
  return `il y a ${Math.floor(h / 24)}j`
}

function estNouveau(iso: string): boolean {
  return Date.now() - new Date(iso).getTime() < 30 * 60_000
}

function estAncien(iso: string): boolean {
  return Date.now() - new Date(iso).getTime() > 6 * 3_600_000
}

function classeTemps(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  if (diff < 30 * 60_000) return 'text-red-400'
  if (diff < 4 * 3_600_000) return 'text-slate-400'
  return 'text-slate-600'
}

function couleurNiveau(niveau: NiveauAlerte): string {
  const map: Record<NiveauAlerte, string> = {
    critique: 'bg-red-500', important: 'bg-orange-400',
    modere: 'bg-yellow-400', veille: 'bg-slate-500',
  }
  return map[niveau]
}

function gaugeClasse(score: number): string {
  if (score >= 80) return 'bg-red-500'
  if (score >= 60) return 'bg-orange-400'
  if (score >= 40) return 'bg-yellow-400'
  return 'bg-emerald-500'
}

function labelRisque(score: number): string {
  if (score >= 80) return 'Critique'
  if (score >= 60) return 'Important'
  if (score >= 40) return 'Modéré'
  return 'Veille'
}

function classeRisque(score: number): string {
  if (score >= 80) return 'text-red-400'
  if (score >= 60) return 'text-orange-400'
  if (score >= 40) return 'text-yellow-400'
  return 'text-slate-400'
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.scroll-zone {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.1) transparent;
}
.scroll-zone::-webkit-scrollbar { width: 4px; }
.scroll-zone::-webkit-scrollbar-track { background: transparent; }
.scroll-zone::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 2px; }
</style>
