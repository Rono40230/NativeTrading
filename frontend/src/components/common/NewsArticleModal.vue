<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div
        v-if="article"
        class="fixed inset-0 z-50 flex items-center justify-center p-6"
        @click.self="$emit('fermer')"
      >
        <!-- Fond -->
        <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="$emit('fermer')" />

        <!-- Carte modale -->
        <div class="relative z-10 w-full max-w-3xl rounded-2xl border border-white/10 bg-[#0f1629] p-8 shadow-2xl">
          <!-- En-tête modale -->
          <div class="mb-4 flex items-start justify-between gap-3">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="h-2 w-2 shrink-0 rounded-full mt-0.5" :class="couleurNiveau(article.niveau)" />
              <span class="text-[11px] font-semibold uppercase tracking-widest" :class="classeRisque(article.score)">
                {{ article.source }}
              </span>
              <span class="rounded-full px-2 py-0.5 text-[10px] font-bold border" :class="badgeNiveau(article.niveau)">
                {{ labelNiveau(article.niveau) }}
              </span>
            </div>
            <button
              class="shrink-0 text-slate-400 hover:text-white transition-colors text-lg leading-none"
              @click="$emit('fermer')"
            >✕</button>
          </div>

          <!-- Titre -->
          <h2 class="text-sm font-semibold text-white leading-snug mb-4">
            {{ article.titre_fr ?? article.titre }}
            <span v-if="article.titre_fr" class="ml-1 text-[9px] text-slate-500 font-normal">(traduit)</span>
          </h2>

          <!-- Méta -->
          <div class="flex items-center gap-4 text-[10px] text-slate-400 mb-3">
            <span>🕐 {{ formatDate(article.date) }}</span>
            <span>Score : <span class="font-bold" :class="classeRisque(article.score)">{{ article.score }}/100</span></span>
          </div>

          <!-- Contenu article -->
          <div class="mb-4 max-h-[36rem] overflow-y-auto scroll-zone rounded-lg bg-white/5 p-3">
            <div v-if="chargement" class="space-y-2 animate-pulse">
              <div v-for="i in 5" :key="i" class="h-2 rounded bg-white/10" :style="{ width: `${50 + i * 9}%` }" />
            </div>
            <div v-else-if="contenu">
              <!-- Origine du contenu : résumé RSS (collecte) vs article scrapé -->
              <div class="mb-2 flex items-center gap-2">
                <span
                  class="rounded-full border px-1.5 py-0.5 text-[9px] font-medium"
                  :class="sourceContenu === 'rss'
                    ? 'border-blue-400/30 bg-blue-400/5 text-blue-300/80'
                    : 'border-emerald-400/30 bg-emerald-400/5 text-emerald-300/80'"
                >{{ sourceContenu === 'rss' ? '📄 Résumé RSS' : '📰 Article complet' }}</span>
                <span v-if="enrichissement" class="flex items-center gap-1.5 text-[9px] text-slate-500">
                  <span class="inline-block h-2 w-2 animate-spin rounded-full border border-slate-400 border-t-transparent" />
                  Récupération de l'article complet…
                </span>
              </div>
              <div v-if="traductionChargement" class="mb-2 flex items-center gap-1.5 text-[9px] text-blue-400">
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
              @click="$emit('fermer')"
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
import { ref, watch } from 'vue'
import { apiService } from '@/services/api.service'
import { formatParis } from '@/utils/date'
import type { ArticleNews, NiveauAlerte } from '@/services/api.types'

const props = defineProps<{ article: ArticleNews | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const contenu = ref<string | null>(null)
const contenuFr = ref<string | null>(null)
const chargement = ref(false)
const traductionChargement = ref(false)
/** Origine du contenu affiché : résumé RSS (collecte) ou scrape complet. */
const sourceContenu = ref<'rss' | 'scrape'>('scrape')
/** Scrape d'enrichissement en cours pendant que le résumé RSS est affiché. */
const enrichissement = ref(false)

watch(() => props.article, async (art, ancien) => {
  // Ré-émission du MÊME article (titre_fr/resume_source revenus de
  // POST /ouvrir côté presse) : pas de re-scrape — on adopte juste le
  // résumé RSS s'il vient d'arriver et que rien n'est encore affiché.
  if (art && ancien && art.id === ancien.id) {
    if (!contenu.value && art.resume_source) {
      contenu.value = art.resume_source
      sourceContenu.value = 'rss'
      chargement.value = false // le scrape en cours reste l'enrichissement
    }
    return
  }

  contenuFr.value = null
  traductionChargement.value = false

  // Résumé RSS immédiat (toujours disponible, collecté au cycle presse).
  // Si on a un résumé : c'est LE contenu — PAS de scrape (les sites JS
  // renvoient des pages de cookies/consentement plus longues que le résumé
  // et l'écraseraient). Le scrape ne sert que sans résumé RSS.
  contenu.value = art?.resume_source || null
  sourceContenu.value = contenu.value ? 'rss' : 'scrape'
  chargement.value = !!art?.url && !contenu.value

  if (!art?.url) return
  // Sans résumé RSS → scrape (seule source de contenu).
  if (contenu.value) return
  try {
    const res = await apiService.obtenirContenuArticle(art.url)
    if (res.texte) {
      contenu.value = res.texte
      sourceContenu.value = 'scrape'
      await traduireContenu()
    }
  } catch {
    // Échec (page rendue en JavaScript, mur de cookies…) : rien à afficher
    // pour cet article sans résumé RSS.
  } finally {
    chargement.value = false
  }
})

async function traduireContenu() {
  if (!contenu.value) return
  traductionChargement.value = true
  try {
    const res = await apiService.traduire(contenu.value, true)
    contenuFr.value = res.texte_fr
  } catch {
    // silencieux
  } finally {
    traductionChargement.value = false
  }
}

function formatDate(iso: string): string {
  return formatParis(new Date(iso), {
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
</script>

<style scoped>
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
