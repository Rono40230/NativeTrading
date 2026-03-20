<template>
  <div v-bind="$attrs" class="glass-card p-4 flex flex-col h-full">
    <!-- En-tête -->
    <div class="mb-3 flex items-center justify-between shrink-0">
      <p class="text-[11px] font-semibold uppercase tracking-widest text-white">
        Revue de Presse
      </p>
      <div v-if="newsStore.chargement" class="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
    </div>

    <!-- Indicateur risque global -->
    <div class="mb-3 flex items-center gap-2 rounded-lg border border-white/5 bg-white/5 px-3 py-1.5 shrink-0">
      <span class="text-xs text-slate-400">Risque news</span>
      <span class="ml-auto text-xs font-bold" :class="classeRisque(newsStore.scoreMax)">
        {{ newsStore.scoreMax }}/100
      </span>
      <span class="text-sm">{{ iconeRisque(newsStore.scoreMax) }}</span>
    </div>

    <!-- Liste articles -->
    <div v-if="newsStore.articlesPertinents.length > 0" class="space-y-2 overflow-y-auto scroll-zone flex-1 pr-0.5">
      <button
        v-for="article in newsStore.articlesPertinents"
        :key="article.id"
        class="group w-full text-left rounded-lg border border-white/5 bg-white/5 p-2 transition hover:bg-white/10 cursor-pointer"
        @click="ouvrir(article)"
      >
        <div class="mb-0.5 flex items-center gap-1.5">
          <span class="h-1.5 w-1.5 shrink-0 rounded-full" :class="couleurNiveau(article.niveau)" />
          <span class="text-[10px] text-slate-400">{{ article.source }}</span>
          <span class="ml-auto text-[10px] font-semibold" :class="classeRisque(article.score)">
            {{ article.score }}
          </span>
        </div>
        <p class="line-clamp-2 text-[11px] leading-snug text-slate-200 group-hover:text-white">
          {{ article.titre_fr ?? article.titre }}
        </p>
        <p class="mt-0.5 text-[10px] text-slate-500">{{ tempsRelatif(article.date) }}</p>
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

  <!-- Modale article -->
  <Teleport to="body">
    <Transition name="modal-fade">
      <div
        v-if="articleOuvert"
        class="fixed inset-0 z-50 flex items-center justify-center p-6"
        @click.self="fermer"
      >
        <!-- Fond -->
        <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="fermer" />

        <!-- Carte modale -->
        <div class="relative z-10 w-full max-w-lg rounded-2xl border border-white/10 bg-[#0f1629] p-6 shadow-2xl">
          <!-- En-tête modale -->
          <div class="mb-4 flex items-start justify-between gap-3">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="h-2 w-2 shrink-0 rounded-full mt-0.5" :class="couleurNiveau(articleOuvert.niveau)" />
              <span class="text-[11px] font-semibold uppercase tracking-widest" :class="classeRisque(articleOuvert.score)">
                {{ articleOuvert.source }}
              </span>
              <span class="rounded-full px-2 py-0.5 text-[10px] font-bold border" :class="badgeNiveau(articleOuvert.niveau)">
                {{ labelNiveau(articleOuvert.niveau) }}
              </span>
            </div>
            <button
              class="shrink-0 text-slate-400 hover:text-white transition-colors text-lg leading-none"
              @click="fermer"
            >✕</button>
          </div>

          <!-- Titre -->
          <h2 class="text-sm font-semibold text-white leading-snug mb-4">
            {{ articleOuvert.titre_fr ?? articleOuvert.titre }}
            <span v-if="articleOuvert.titre_fr" class="ml-1 text-[9px] text-slate-500 font-normal">(traduit)</span>
          </h2>

          <!-- Méta -->
          <div class="flex items-center gap-4 text-[10px] text-slate-400 mb-3">
            <span>🕐 {{ formatDate(articleOuvert.date) }}</span>
            <span>Score : <span class="font-bold" :class="classeRisque(articleOuvert.score)">{{ articleOuvert.score }}/100</span></span>
          </div>

          <!-- Contenu article -->
          <div class="mb-4 max-h-72 overflow-y-auto scroll-zone rounded-lg bg-white/5 p-3">
            <div v-if="contenuChargement" class="space-y-2 animate-pulse">
              <div v-for="i in 5" :key="i" class="h-2 rounded bg-white/10" :style="{ width: `${50 + i * 9}%` }" />
            </div>
            <div v-else-if="contenu">
              <!-- Indicateurs traduction -->
              <div v-if="traductionContenuChargement" class="mb-2 flex items-center gap-1.5 text-[9px] text-blue-400">
                <span class="inline-block h-2 w-2 animate-spin rounded-full border border-blue-400 border-t-transparent" />
                Traduction en cours…
              </div>
              <div v-else-if="contenuFr" class="mb-2 flex items-center gap-2">
                <span class="text-[9px] text-slate-500">🌐 Traduit automatiquement</span>
                <button
                  class="rounded border border-slate-600/40 bg-white/5 px-1.5 py-0.5 text-[9px] text-slate-500 hover:text-slate-300 transition-colors"
                  @click="contenuFr = null"
                >Voir original</button>
              </div>
              <p class="text-xs text-slate-300 leading-relaxed whitespace-pre-wrap">
                {{ contenuFr ?? contenu }}
              </p>
            </div>
            <p v-else class="text-center text-[11px] text-slate-500 py-2">
              Article non accessible directement.
            </p>
          </div>

          <!-- Actions -->
          <div class="flex gap-2">
            <button
              class="flex-1 rounded-xl border border-white/10 bg-white/5 hover:bg-white/10 transition-colors px-4 py-2.5 text-xs text-slate-300"
              @click="fermer"
            >
              Fermer
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
defineOptions({ inheritAttrs: false })
import { ref } from 'vue'
import { apiService } from '@/services/api.service'
import { useNewsStore } from '@/stores/news.store'
import type { ArticleNews, NiveauAlerte } from '@/services/api.types'

const newsStore = useNewsStore()
const articleOuvert = ref<ArticleNews | null>(null)
const contenu = ref<string | null>(null)
const contenuFr = ref<string | null>(null)
const contenuChargement = ref(false)
const contenuInaccessible = ref(false)
const traductionContenuChargement = ref(false)

async function ouvrir(article: ArticleNews) {
  articleOuvert.value = article
  contenu.value = null
  contenuFr.value = null
  contenuInaccessible.value = false
  traductionContenuChargement.value = false
  if (!article.url) return
  contenuChargement.value = true
  try {
    const res = await apiService.obtenirContenuArticle(article.url)
    contenu.value = res.texte
    traduireContenu()
  } catch {
    contenuInaccessible.value = true
  } finally {
    contenuChargement.value = false
  }
}

function fermer() {
  articleOuvert.value = null
  contenu.value = null
  contenuFr.value = null
  contenuInaccessible.value = false
  contenuChargement.value = false
  traductionContenuChargement.value = false
}

async function traduireContenu() {
  if (!contenu.value || traductionContenuChargement.value) return
  traductionContenuChargement.value = true
  try {
    const res = await apiService.traduire(contenu.value, true)
    contenuFr.value = res.texte_fr
  } catch {
    // silencieux
  } finally {
    traductionContenuChargement.value = false
  }
}

function tempsRelatif(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  const min = Math.floor(diff / 60_000)
  if (min < 1) return 'à l\'instant'
  if (min < 60) return `il y a ${min} min`
  return `il y a ${Math.floor(min / 60)}h`
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString('fr-FR', {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  })
}

function couleurNiveau(niveau: NiveauAlerte): string {
  const map: Record<NiveauAlerte, string> = {
    critique: 'bg-red-500', important: 'bg-orange-400',
    modere: 'bg-yellow-400', veille: 'bg-slate-500',
  }
  return map[niveau]
}

function badgeNiveau(niveau: NiveauAlerte): string {
  const map: Record<NiveauAlerte, string> = {
    critique: 'border-red-500/40 text-red-400 bg-red-500/10',
    important: 'border-orange-400/40 text-orange-400 bg-orange-400/10',
    modere: 'border-yellow-400/40 text-yellow-400 bg-yellow-400/10',
    veille: 'border-slate-500/40 text-slate-400 bg-slate-500/10',
  }
  return map[niveau]
}

function labelNiveau(niveau: NiveauAlerte): string {
  const map: Record<NiveauAlerte, string> = {
    critique: 'Critique', important: 'Important',
    modere: 'Modéré', veille: 'Veille',
  }
  return map[niveau]
}

function classeRisque(score: number): string {
  if (score >= 80) return 'text-red-400'
  if (score >= 60) return 'text-orange-400'
  if (score >= 40) return 'text-yellow-400'
  return 'text-slate-400'
}

function iconeRisque(score: number): string {
  if (score >= 80) return '🔴'
  if (score >= 60) return '🟠'
  if (score >= 40) return '🟡'
  return '🔵'
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
.modal-fade-enter-active, .modal-fade-leave-active { transition: opacity 0.2s ease; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }
</style>

