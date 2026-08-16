<template>
  <div class="flex flex-col h-[calc(100vh-8rem)]">
    <!-- Bandeau -->
    <div class="flex items-center justify-between shrink-0 mb-4">
      <h1 class="text-2xl font-bold">📰 Revue de presse</h1>
      <button
        class="px-3 py-1.5 rounded-lg bg-white/5 text-gray-300 text-sm hover:bg-white/10 transition"
        @click="modaleSources = true"
      >📡 Sources RSS</button>
    </div>

    <!-- Split-panel : bibliothèque 2/3 | liseuse 1/3 -->
    <div class="flex gap-4 flex-1 min-h-0">

      <!-- ── Colonne gauche (2/3) : filtres, cartes, brief repliable ── -->
      <div class="flex-1 min-w-0 flex flex-col gap-4 overflow-y-auto scroll-zone pr-1">

        <!-- Filtres (une ligne) -->
        <div class="glass-card p-3 flex flex-wrap gap-3 items-center shrink-0">
          <input v-model="filtre.q" placeholder="Recherche…" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" @keyup.enter="charger()" />
          <select v-model="filtre.theme" class="bg-white text-black rounded-lg px-2 py-1.5 text-sm" @change="charger()">
            <option value="">Tous thèmes</option>
            <option v-for="t in themes" :key="t" :value="t">{{ t }}</option>
          </select>
          <select v-model="filtre.asset" class="bg-white text-black rounded-lg px-2 py-1.5 text-sm" @change="charger()">
            <option value="">Tous assets</option>
            <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
          </select>
          <select v-model="filtre.lu" class="bg-white text-black rounded-lg px-2 py-1.5 text-sm" @change="charger()">
            <!-- lu=true → articles LUS, lu=false → NON LUS (interprétation backend) -->
            <option value="">Lu + non lus</option><option value="true">Lis</option><option value="false">Non lus</option>
          </select>
          <!-- articles.length = total chargé (toutes pages « Charger plus » confondues) -->
          <span class="text-xs text-gray-500">{{ articles.length }} articles</span>
        </div>

        <!-- Bibliothèque — cartes (4/ligne max, clic → liseuse) -->
        <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <button
            v-for="a in articles"
            :key="a.hash_titre"
            class="glass-card p-3 text-left hover:bg-white/10 transition flex flex-col gap-2 min-h-28"
            :class="{ 'opacity-50': a.lu, 'border-blue-500/60 ring-1 ring-blue-500/50': selectionnee(a) }"
            @click="lire(a)"
          >
            <div class="flex items-start justify-between gap-1">
              <span
                v-if="estNouveau(a.ajoute_le) && !a.lu"
                class="text-[9px] font-bold text-red-300 bg-red-600/40 border border-red-500/40 rounded-full px-1.5 py-0.5 leading-none animate-pulse shrink-0"
              >NOUVEAU</span>
              <span
                v-else-if="a.lu"
                class="text-[9px] font-semibold text-blue-200 bg-blue-600/70 border border-blue-500/50 rounded-full px-1.5 py-0.5 leading-none shrink-0"
              >Vu</span>
              <span class="ml-auto text-[10px] font-semibold tabular-nums" :class="classeScore(a.score)">{{ a.score }}</span>
            </div>
            <span class="text-xs leading-snug line-clamp-3" :class="a.lu ? 'text-gray-500' : 'text-white font-medium'">{{ a.titre_fr ?? a.titre }}</span>
            <div class="flex flex-wrap gap-1 text-[10px] mt-auto">
              <span class="px-1.5 py-0.5 rounded" :class="a.impact === 'fort' ? 'bg-red-500/15 text-red-300' : a.impact === 'moyen' ? 'bg-yellow-500/15 text-yellow-300' : 'bg-white/10 text-gray-400'">{{ a.impact }}</span>
              <span class="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300">{{ a.theme }}</span>
              <span v-for="asset in parseAssets(a)" :key="asset" class="px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-300">{{ asset }}</span>
            </div>
            <span class="text-[10px] text-gray-500 truncate">{{ a.source_nom }}</span>
          </button>
        </div>
        <p v-if="articles.length === 0" class="text-sm text-gray-500 p-4">Bibliothèque vide — le collecteur remplit au prochain cycle (30 min).</p>
        <!-- Pagination : le backend sert 50 articles/page, on empile les pages suivantes -->
        <div v-if="!aToutCharge && articles.length > 0" class="flex justify-center shrink-0">
          <button
            class="px-4 py-2 rounded-lg bg-white/5 text-gray-300 text-sm hover:bg-white/10 transition"
            @click="charger(false)"
          >Charger plus</button>
        </div>

        <!-- Brief repliable (bas de colonne) -->
        <details class="glass-card p-4 shrink-0">
          <summary class="flex items-center justify-between cursor-pointer list-none gap-3 [&::-webkit-details-marker]:hidden">
            <span class="text-sm font-semibold text-gray-300">
              📝 Brief 24 h
              <span v-if="briefParse" class="text-xs text-gray-500 font-normal">· {{ briefParse.articles.length }} articles</span>
            </span>
            <button
              class="px-3 py-1.5 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 disabled:opacity-40"
              :disabled="enBrief" @click.stop="genererBrief()"
            >{{ enBrief ? '⏳ Génération…' : '⚡ Générer' }}</button>
          </summary>

          <div class="mt-4 space-y-4">
            <template v-if="briefParse">
              <!-- Contexte marché — bandeau d'intro -->
              <div class="rounded-xl border border-blue-500/20 bg-blue-500/5 px-4 py-3">
                <p class="text-[10px] font-semibold uppercase tracking-wider text-blue-300 mb-1">🌍 Contexte marché</p>
                <p class="text-sm text-gray-200 leading-relaxed">{{ briefParse.contexte }}</p>
              </div>

              <!-- Articles du brief — cartes, clic → liseuse -->
              <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                <button
                  v-for="art in briefParse.articles"
                  :key="art.numero"
                  class="rounded-xl border border-white/10 bg-white/[0.04] p-4 flex flex-col gap-3 hover:bg-white/[0.07] hover:border-white/20 transition text-left relative overflow-hidden"
                  @click="ouvrirArticleBrief(art)"
                >
                  <div class="absolute top-0 left-0 right-0 h-1" :class="art.score >= 60 ? 'bg-red-400/70' : art.score >= 40 ? 'bg-yellow-400/70' : 'bg-gray-500/50'"></div>

                  <div class="flex items-start justify-between gap-2">
                    <span class="text-[10px] font-bold text-gray-500 bg-white/5 rounded-md px-1.5 py-0.5 shrink-0">#{{ art.numero }}</span>
                    <span class="text-lg font-bold tabular-nums shrink-0" :class="classeScore(art.score)">{{ art.score }}</span>
                  </div>

                  <h3 class="text-sm font-semibold text-white leading-snug line-clamp-3">{{ art.titre }}</h3>
                  <p class="text-xs text-gray-400 leading-relaxed line-clamp-4">{{ art.resume }}</p>

                  <div class="mt-auto flex flex-wrap items-center gap-1.5 text-[10px]">
                    <span class="px-1.5 py-0.5 rounded" :class="art.score >= 60 ? 'bg-red-500/15 text-red-300' : art.score >= 40 ? 'bg-yellow-500/15 text-yellow-300' : 'bg-white/10 text-gray-400'">{{ art.score >= 60 ? 'fort' : art.score >= 40 ? 'moyen' : 'faible' }}</span>
                    <span class="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300">{{ art.theme }}</span>
                    <span class="px-1.5 py-0.5 rounded bg-white/10 text-gray-400 truncate max-w-[10rem]">{{ art.source }}</span>
                  </div>
                </button>
              </div>

              <p class="text-[10px] text-gray-500 text-right">Brief du {{ new Date(dernierBrief?.genere_le ? dernierBrief.genere_le * 1000 : Date.now()).toLocaleString('fr-FR') }} · {{ briefParse.articles.length }} articles</p>
            </template>

            <p v-else class="text-sm text-gray-500">Aucun brief — clique « Générer » (Ollama, ~1 min).</p>
            <p v-if="erreurBrief" class="text-sm text-red-400">{{ erreurBrief }}</p>
          </div>
        </details>
      </div>

      <!-- ── Liseuse (1/3 écran) — design éditorial ── -->
      <aside class="w-1/3 min-w-[22rem] shrink-0 rounded-xl border border-white/10 bg-[#101218] overflow-hidden flex flex-col">
        <template v-if="liseuse">
          <!-- En-tête éditorial : bandeau coloré + titre + méta -->
          <div class="relative bg-gradient-to-br from-slate-800/80 to-slate-900/60 px-5 pt-4 pb-3 border-b border-white/10">
            <div class="absolute top-0 left-0 right-0 h-1" :class="liseuse.article.score >= 60 ? 'bg-red-400/80' : liseuse.article.score >= 40 ? 'bg-yellow-400/80' : 'bg-slate-500/60'"></div>
            <p class="text-[10px] font-bold uppercase tracking-widest text-blue-300/90 mb-1.5">{{ liseuse.article.source_nom }}</p>
            <h2 class="text-base font-bold text-white leading-snug">{{ liseuse.titre_fr ?? liseuse.article.titre }}</h2>
            <div class="mt-2 flex flex-wrap items-center gap-2 text-[10px]">
              <span class="px-1.5 py-0.5 rounded-full font-bold" :class="liseuse.article.score >= 60 ? 'bg-red-500/20 text-red-300' : liseuse.article.score >= 40 ? 'bg-yellow-500/20 text-yellow-300' : 'bg-white/10 text-gray-400'">Score {{ liseuse.article.score }}/100</span>
              <span class="px-1.5 py-0.5 rounded-full bg-blue-500/15 text-blue-300">{{ liseuse.article.theme }}</span>
              <span class="text-gray-500">{{ formaterDate(liseuse.article.ajoute_le) }}</span>
            </div>
          </div>

          <!-- Corps scrollable -->
          <div class="flex-1 overflow-y-auto scroll-zone px-5 py-4">
            <!-- Skeleton : Jina en cours, résumé RSS affiché dessous s'il existe -->
            <div v-if="liseuse.chargement" class="space-y-3">
              <p class="text-xs text-blue-300 flex items-center gap-2">
                <span class="inline-block h-2 w-2 animate-spin rounded-full border border-blue-400 border-t-transparent" />
                Récupération de l'article…
              </p>
              <p v-if="liseuse.contenu" class="text-sm text-gray-300 leading-relaxed whitespace-pre-wrap">{{ liseuse.contenu }}</p>
              <div v-else class="animate-pulse space-y-2.5">
                <div v-for="i in 8" :key="i" class="h-2.5 rounded bg-white/10" :style="{ width: `${55 + i * 5}%` }" />
              </div>
            </div>

            <template v-else-if="liseuse.contenu">
              <!-- Origine du contenu -->
              <div class="mb-3 flex items-center gap-2 flex-wrap">
                <span
                  class="rounded-full border px-2 py-0.5 text-[10px] font-semibold"
                  :class="liseuse.sourceContenu === 'jina'
                    ? 'border-emerald-400/30 bg-emerald-400/10 text-emerald-300'
                    : 'border-blue-400/30 bg-blue-400/10 text-blue-300'"
                >{{ liseuse.sourceContenu === 'jina' ? '📰 Article complet' : '📄 Résumé RSS' }}</span>
                <span v-if="liseuse.enTraduction" class="flex items-center gap-1.5 text-[10px] text-blue-400">
                  <span class="inline-block h-2 w-2 animate-spin rounded-full border border-blue-400 border-t-transparent" />
                  Traduction…
                </span>
                <button
                  v-else-if="liseuse.contenu_fr"
                  class="rounded-full border border-slate-600/40 bg-white/5 px-2 py-0.5 text-[10px] text-slate-400 hover:text-slate-200 transition-colors"
                  @click="liseuse.contenu_fr = null"
                >Voir original</button>
              </div>
              <p class="text-sm text-gray-200 leading-7 whitespace-pre-wrap font-light">{{ liseuse.contenu_fr ?? liseuse.contenu }}</p>
            </template>
            <div v-else class="flex flex-col items-center justify-center py-10 gap-2 text-center">
              <span class="text-2xl opacity-30">🔒</span>
              <p class="text-xs text-gray-500">Article non accessible<br/><span class="text-[10px]">Ce site bloque la lecture externe</span></p>
            </div>
          </div>

          <!-- Pied : lien source -->
          <div class="px-5 py-2.5 border-t border-white/10 bg-white/[0.02] shrink-0">
            <a :href="liseuse.article.url" target="_blank" class="text-[11px] text-blue-400 hover:text-blue-300 hover:underline transition-colors">Lire sur le site source ↗</a>
          </div>
        </template>

        <!-- Liseuse vide -->
        <div v-else class="h-full flex flex-col items-center justify-center text-center gap-2">
          <span class="text-3xl opacity-40">📰</span>
          <p class="text-sm text-gray-500">Clique un article pour le lire ici</p>
        </div>
      </aside>
    </div>

    <!-- Modal sources (opaque, ouverte par le bouton du bandeau) -->
    <div v-if="modaleSources" class="fixed inset-0 z-50 flex items-center justify-center bg-black/30" @click.self="modaleSources = false">
      <div class="w-full max-w-lg p-6 space-y-4 rounded-2xl border border-white/10 bg-[#16181d] shadow-2xl">
        <div class="flex items-center justify-between">
          <h3 class="font-bold text-white">📡 Sources RSS</h3>
          <button class="text-gray-400 hover:text-white transition" @click="modaleSources = false">✕</button>
        </div>
        <div class="space-y-2 max-h-72 overflow-y-auto">
          <div v-for="s in sources" :key="s.id" class="flex items-center justify-between text-sm">
            <span :class="s.actif ? 'text-gray-300' : 'text-gray-600 line-through'">{{ s.nom }} <span class="text-xs text-gray-500">(poids {{ s.poids_score }})</span></span>
            <button class="text-red-400 hover:text-red-300 text-xs" @click="retirerSource(s.id)">Retirer</button>
          </div>
        </div>
        <div class="grid grid-cols-1 gap-2 pt-2 border-t border-white/5">
          <input v-model="nouvelleSource.nom" placeholder="Nom" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
          <input v-model="nouvelleSource.url" placeholder="https://flux.example/rss" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
        </div>
        <div class="flex gap-3 pt-1">
          <button class="flex-1 px-4 py-2 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 transition" @click="ajouterSource()">+ Ajouter</button>
          <button class="flex-1 px-4 py-2 rounded-lg bg-white/5 text-gray-300 text-sm hover:bg-white/10 transition" @click="modaleSources = false">Fermer</button>
        </div>
        <p v-if="messageSource" class="text-xs leading-relaxed" :class="messageSource.ok ? 'text-emerald-400' : 'text-amber-400'">
          {{ messageSource.texte }}
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { apiService } from '@/services/api.service'
import { presseApi, type ArticlePresse } from '@/services/api.presse'

/** Article structuré extrait du markdown du brief. */
interface ArticleBrief {
  numero: number
  titre: string
  source: string
  score: number
  theme: string
  resume: string
}

/** État de la liseuse (panneau droit) : article affiché + contenu chargé. */
interface EtatLiseuse {
  article: ArticlePresse
  titre_fr: string | null
  contenu: string | null
  contenu_fr: string | null
  chargement: boolean
  enTraduction: boolean
  sourceContenu: 'rss' | 'jina' | null
}

const articles = ref<ArticlePresse[]>([])
const page = ref(1) // prochaine page à demander au backend (50 articles/page)
const aToutCharge = ref(false) // dernière page servie < 50 articles → rien de plus à charger
const sources = ref<Awaited<ReturnType<typeof presseApi.sources>>>([])
const enBrief = ref(false)
const erreurBrief = ref<string | null>(null)
const dernierBrief = ref<Awaited<ReturnType<typeof presseApi.briefs>>[number] | null>(null)
const liseuse = ref<EtatLiseuse | null>(null)
/** Garde-fou concurrence : incrémenté à chaque sélection — une réponse réseau
 *  tardive d'une sélection précédente ne doit pas écraser la nouvelle. */
let selectionLiseuse = 0

/** Badge NOUVEAU : < 30 min (même logique que NewsFeed dashboard). */
function estNouveau(epochSec: number): boolean {
  return Date.now() / 1000 - epochSec < 1800
}
const filtre = reactive({ q: '', theme: '', asset: '', lu: '' })
const modaleSources = ref(false)
const nouvelleSource = reactive({ nom: '', url: '' })
// Valeurs réellement produites par classer_theme (backend)
const themes = ['macro', 'crypto', 'metaux', 'autre']
const assets = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD', 'EURUSD', 'USDJPY', 'DAX', 'NAS100', 'SP500']

function classeScore(score: number): string {
  return score >= 60 ? 'text-red-300' : score >= 40 ? 'text-yellow-300' : 'text-gray-400'
}

function formaterDate(epochSec: number): string {
  return new Date(epochSec * 1000).toLocaleString('fr-FR', {
    day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit',
  })
}

/** Dépouille une éventuelle clôture markdown ```...``` du contenu du brief avant affichage. */
function sansCloture(c: string): string {
  return c.replace(/^\s*```(?:markdown)?\s*\n?/, '').replace(/\n?\s*```\s*$/, '')
}

/** Parse le markdown du brief en structure affichable : contexte + articles
 * détaillés (cartes). Format LLM : « ## Contexte marché » / « ## Articles
 * marquants » / « ### N. Titre (Source) - [score/100|theme] » + résumé. */
const briefParse = computed<{ contexte: string; articles: ArticleBrief[] } | null>(() => {
  if (!dernierBrief.value) return null
  const texte = sansCloture(dernierBrief.value.contenu)
  const sections = texte.split(/^##\s+/m)

  let contexte = ''
  for (const s of sections) {
    const corps = s.replace(/^Contexte marché\s*\n?/i, '').trim()
    if (s.toLowerCase().startsWith('contexte marché')) {
      contexte = corps
      break
    }
  }

  const articles: ArticleBrief[] = []
  for (const s of sections) {
    if (!/^Articles marquants/i.test(s)) continue
    for (const bloc of s.split(/^###\s+/m).slice(1)) {
      const lignes = bloc.split('\n').map(l => l.trim()).filter(Boolean)
      if (lignes.length === 0) continue
      const entete = lignes[0]
      // « 1. Titre (Source) - [62/100|crypto] » (le / de « /100 » est échappé)
      const m = entete.match(/^(\d+)[.]\s*(.+?)(?:\s*[（(]([^)）]+)[)）])?\s*-?\s*[（([]\s*(\d+)\s*\/\s*100\s*\|\s*([a-zàâçéèêëîïôùûü]+)\s*[)）\]]/i)
      if (!m) continue
      articles.push({
        numero: parseInt(m[1], 10),
        titre: m[2].trim(),
        source: (m[3] ?? '').trim() || '—',
        score: parseInt(m[4], 10),
        theme: (m[5] ?? '').trim().toLowerCase(),
        resume: lignes.slice(1).join(' '),
      })
    }
  }

  if (!contexte && articles.length === 0) return null
  return { contexte, articles }
})

function parseAssets(a: ArticlePresse): string[] {
  try { return JSON.parse(a.assets_concernes) } catch { return [] }
}

/** Carte sélectionnée = celle affichée dans la liseuse (surbrillance bleue). */
function selectionnee(a: ArticlePresse): boolean {
  return liseuse.value?.article.hash_titre === a.hash_titre
}

/** Charge une page de la bibliothèque. reset=true (filtres, montage) repart de
 * la page 1 ; reset=false empile la page suivante (« Charger plus »). */
async function charger(reset = true) {
  if (reset) {
    page.value = 1
    articles.value = []
  }
  const res = await presseApi.articles({
    q: filtre.q || undefined, theme: filtre.theme || undefined,
    asset: filtre.asset || undefined, lu: filtre.lu || undefined,
    page: page.value,
  })
  if (reset) articles.value = res
  else articles.value.push(...res)
  aToutCharge.value = res.length < 50
  page.value += 1
}

/** Dépouille le préambule de Jina AI Reader (« Title: … / URL Source: … /
 *  Markdown Content: ») — ne garder que la chair de l'article. */
function nettoyerJina(texte: string): string {
  const marqueur = 'Markdown Content:'
  const idx = texte.indexOf(marqueur)
  if (idx >= 0) return texte.slice(idx + marqueur.length).trim()
  return texte.replace(/^(Title|URL Source|Published Time|Warning):.*\n?/gm, '').trim()
}

/** Traduit le contenu Jina en FR via Ollama (endpoint news partagé
 *  /api/news/traduire). Le backend tronque la traduction à 3000 caractères —
 *  on borne l'envoi à la même longueur (le GET passe le texte en query
 *  param, l'URL doit rester raisonnable). La VO complète reste affichable
 *  via « Voir original ». */
async function traduireContenuLiseuse(id: number) {
  const texte = liseuse.value?.contenu
  if (!texte || !liseuse.value) return
  liseuse.value.enTraduction = true
  try {
    const extrait = Array.from(texte).slice(0, 3000).join('')
    const res = await apiService.traduire(extrait, true)
    // L'endpoint rend le texte ORIGINAL en cas d'échec : une « traduction »
    // identique à l'extrait = échec → VO complète conservée à l'affichage.
    if (res.texte_fr && res.texte_fr.trim() !== extrait.trim()) {
      if (id === selectionLiseuse && liseuse.value) liseuse.value.contenu_fr = res.texte_fr
    }
  } catch {
    // silencieux : VO affichée
  } finally {
    if (id === selectionLiseuse && liseuse.value) liseuse.value.enTraduction = false
  }
}

/** Ouvre un article dans la LISEUSE (panneau droit) : affichage immédiat du
 * titre FR (cache) ou VO + résumé RSS, puis EN PARALLÈLE (a) machine à états
 * traduction + marquage lu (POST /ouvrir) et (b) article complet via Jina AI
 * Reader (rend le JavaScript, contourne les murs de cookies) + traduction FR.
 * Si Jina échoue, le résumé RSS reste le contenu affiché. */
async function lire(a: ArticlePresse) {
  const id = ++selectionLiseuse
  liseuse.value = {
    article: a,
    titre_fr: a.titre_fr ?? null,
    contenu: a.resume_source ?? null,
    contenu_fr: null,
    chargement: true,
    enTraduction: false,
    sourceContenu: a.resume_source ? 'rss' : null,
  }

  // (a) Traduction du titre (porte d'entrée) + marquage lu + MAJ carte en place.
  presseApi.ouvrir(a.hash_titre).then(res => {
    if (id !== selectionLiseuse) return
    if (res.titre_fr && liseuse.value) liseuse.value.titre_fr = res.titre_fr
    articles.value = articles.value.map(x => (x.hash_titre === a.hash_titre ? res.article : x))
    // Filtre « Non lus » : l'article désormais lu n'y appartient plus.
    if (filtre.lu === 'false') {
      articles.value = articles.value.filter(x => x.hash_titre !== a.hash_titre)
    }
  }).catch((err: any) => {
    // 410 Gone : article supprimé après traduction impossible ×2
    if (err?.response?.status === 410) {
      articles.value = articles.value.filter(x => x.hash_titre !== a.hash_titre)
      if (liseuse.value?.article.hash_titre === a.hash_titre) liseuse.value = null
    }
    // Autre erreur : la liseuse reste ouverte sur le titre VO + résumé RSS.
  })

  // (b) Article complet via Jina AI Reader.
  if (!a.url) {
    if (id === selectionLiseuse && liseuse.value) liseuse.value.chargement = false
    return
  }
  try {
    const res = await presseApi.articleComplet(a.url)
    if (id !== selectionLiseuse || !liseuse.value) return
    if (res.contenu) {
      liseuse.value.contenu = nettoyerJina(res.contenu)
      liseuse.value.sourceContenu = 'jina'
      liseuse.value.chargement = false
      await traduireContenuLiseuse(id)
    } else {
      // Jina a échoué : le résumé RSS reste le contenu (s'il existe).
      liseuse.value.chargement = false
      liseuse.value.sourceContenu = liseuse.value.contenu ? 'rss' : null
    }
  } catch {
    if (id === selectionLiseuse && liseuse.value) {
      liseuse.value.chargement = false
      liseuse.value.sourceContenu = liseuse.value.contenu ? 'rss' : null
    }
  }
}

/** Titre normalisé : minuscules, sans accents ni ponctuation. */
function normaliserTitre(t: string): string {
  return t
    .normalize('NFD').replace(/[\u0300-\u036f]/g, '')
    .toLowerCase().replace(/[^a-z0-9 ]/g, ' ').replace(/\s+/g, ' ').trim()
}

/** Correspondance approximative brief → bibliothèque (le brief affiche des
 * titres FR, la bibliothèque référence les mêmes articles) : titres
 * normalisés identiques/inclus, ou ≥ 60 % de mots significatifs communs. */
function trouverArticleBibliotheque(titre: string): ArticlePresse | null {
  const cible = normaliserTitre(titre)
  if (!cible) return null
  const motsCible = new Set(cible.split(' ').filter(m => m.length > 3))
  for (const a of articles.value) {
    for (const t of [a.titre_fr, a.titre]) {
      if (!t) continue
      const n = normaliserTitre(t)
      if (!n) continue
      if (n === cible || n.includes(cible) || cible.includes(n)) return a
      const mots = n.split(' ').filter(m => m.length > 3)
      if (motsCible.size === 0 || mots.length === 0) continue
      const communs = [...motsCible].filter(m => mots.includes(m)).length
      if (communs / Math.max(motsCible.size, mots.length) >= 0.6) return a
    }
  }
  return null
}

/** Carte du brief : ouvre l'article bibliothèque correspondant dans la
 * liseuse (recherche par titre approximatif) ; à défaut affiche le résumé
 * LLM du brief seul (les articles du brief n'ont pas d'URL source). */
function ouvrirArticleBrief(art: ArticleBrief) {
  const correspondance = trouverArticleBibliotheque(art.titre)
  if (correspondance) {
    lire(correspondance)
    return
  }
  selectionLiseuse++ // invalide les chargements réseau en cours
  liseuse.value = {
    article: {
      hash_titre: `brief-${dernierBrief.value?.id ?? 0}-${art.numero}`,
      titre: art.titre,
      url: '',
      source_nom: art.source,
      publie_le: '',
      score: art.score,
      theme: art.theme,
      assets_concernes: '[]',
      impact: art.score >= 60 ? 'fort' : art.score >= 40 ? 'moyen' : 'faible',
      statut_traduction: 'ok',
      lu: true,
      ajoute_le: dernierBrief.value?.genere_le ?? Math.floor(Date.now() / 1000),
      resume_source: art.resume,
    },
    titre_fr: art.titre, // déjà FR (résumé LLM)
    contenu: art.resume || '—',
    contenu_fr: null,
    chargement: false,
    enTraduction: false,
    sourceContenu: 'rss',
  }
}

async function genererBrief() {
  erreurBrief.value = null
  enBrief.value = true
  try {
    await presseApi.genererBrief()
    dernierBrief.value = (await presseApi.briefs())[0] ?? null
  } catch (err: any) {
    erreurBrief.value = err?.response?.data?.erreur ?? 'Erreur inconnue'
  } finally { enBrief.value = false }
}

const messageSource = ref<{ ok: boolean; texte: string } | null>(null)

async function ajouterSource() {
  if (!nouvelleSource.nom || !nouvelleSource.url.startsWith('https://')) return
  messageSource.value = null
  try {
    const res = await presseApi.ajouterSource(nouvelleSource.nom, nouvelleSource.url, 30, 'marches')
    nouvelleSource.nom = ''; nouvelleSource.url = ''
    sources.value = await presseApi.sources()
    // Avertissement si le flux n'inclut pas de description (articles illisibles)
    messageSource.value = res.description_incluse
      ? { ok: true, texte: `✅ Source ajoutée — ${res.items_avec_description}/${res.items_testes} items avec description` }
      : { ok: false, texte: `⚠️ Source ajoutée MAIS ce flux n'a AUCUNE description (${res.items_testes} items testés) — les articles afficheront « non accessible ». Considère un flux alternatif.` }
  } catch (err: any) {
    messageSource.value = { ok: false, texte: err?.response?.data?.erreur ?? 'Erreur inconnue (flux injoignable ?)' }
  }
}

async function retirerSource(id: number) {
  await presseApi.retirerSource(id)
  // Rafraîchir les sources ET la bibliothèque (les articles du flux
  // supprimé ont été purgés côté backend).
  sources.value = await presseApi.sources()
  await charger()
}

onMounted(async () => {
  await charger()
  sources.value = await presseApi.sources()
  dernierBrief.value = (await presseApi.briefs())[0] ?? null
})
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
