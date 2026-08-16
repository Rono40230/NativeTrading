<template>
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <h1 class="text-2xl font-bold">📰 Revue de presse</h1>
      <button
        class="px-3 py-1.5 rounded-lg bg-white/5 text-gray-300 text-sm hover:bg-white/10 transition"
        @click="modaleSources = true"
      >📡 Sources RSS</button>
    </div>

    <!-- Brief -->
    <div class="glass-card p-5 space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Brief 24 h</h2>
        <button
          class="px-3 py-1.5 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 disabled:opacity-40"
          :disabled="enBrief" @click="genererBrief()"
        >{{ enBrief ? '⏳ Génération…' : '⚡ Générer le brief' }}</button>
      </div>

      <template v-if="briefParse">
        <!-- Contexte marché — bandeau d'intro -->
        <div class="rounded-xl border border-blue-500/20 bg-blue-500/5 px-4 py-3">
          <p class="text-[10px] font-semibold uppercase tracking-wider text-blue-300 mb-1">🌍 Contexte marché</p>
          <p class="text-sm text-gray-200 leading-relaxed">{{ briefParse.contexte }}</p>
        </div>

        <!-- Articles du brief — cartes détaillées, 4/ligne -->
        <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          <article
            v-for="art in briefParse.articles"
            :key="art.numero"
            class="rounded-xl border border-white/10 bg-white/[0.04] p-4 flex flex-col gap-3 hover:bg-white/[0.07] hover:border-white/20 transition relative overflow-hidden"
          >
            <!-- Accent coloré selon le score, en haut de carte -->
            <div class="absolute top-0 left-0 right-0 h-1" :class="art.score >= 60 ? 'bg-red-400/70' : art.score >= 40 ? 'bg-yellow-400/70' : 'bg-gray-500/50'"></div>

            <div class="flex items-start justify-between gap-2">
              <span class="text-[10px] font-bold text-gray-500 bg-white/5 rounded-md px-1.5 py-0.5 shrink-0">#{{ art.numero }}</span>
              <div class="text-right shrink-0">
                <span class="text-lg font-bold tabular-nums" :class="art.score >= 60 ? 'text-red-300' : art.score >= 40 ? 'text-yellow-300' : 'text-gray-400'">{{ art.score }}</span>
                <span class="text-[10px] text-gray-500">/100</span>
              </div>
            </div>

            <h3 class="text-sm font-semibold text-white leading-snug line-clamp-3">{{ art.titre }}</h3>
            <p class="text-xs text-gray-400 leading-relaxed line-clamp-4">{{ art.resume }}</p>

            <div class="mt-auto flex flex-wrap items-center gap-1.5 text-[10px]">
              <span class="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300">{{ art.theme }}</span>
              <span class="px-1.5 py-0.5 rounded bg-white/10 text-gray-400 truncate max-w-[10rem]">{{ art.source }}</span>
            </div>
          </article>
        </div>

        <p class="text-[10px] text-gray-500 text-right">Brief du {{ new Date(dernierBrief?.genere_le ? dernierBrief.genere_le * 1000 : Date.now()).toLocaleString('fr-FR') }} · {{ briefParse.articles.length }} articles</p>
      </template>

      <p v-else class="text-sm text-gray-500">Aucun brief — clique « Générer » (Ollama, ~1 min).</p>
      <p v-if="erreurBrief" class="text-sm text-red-400">{{ erreurBrief }}</p>
    </div>

    <!-- Filtres -->
    <div class="glass-card p-4 flex flex-wrap gap-3 items-center">
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

    <!-- Bibliothèque — cartes -->
    <div class="grid gap-3 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6 2xl:grid-cols-8">
      <button
        v-for="a in articles"
        :key="a.hash_titre"
        class="glass-card p-3 text-left hover:bg-white/10 transition flex flex-col gap-2 min-h-28"
        :class="{ 'opacity-50': a.lu }"
        @click="ouvrir(a)"
      >
        <span class="text-xs leading-snug line-clamp-3" :class="a.lu ? 'text-gray-500' : 'text-white font-medium'">{{ a.titre }}</span>
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
    <div v-if="!aToutCharge && articles.length > 0" class="flex justify-center">
      <button
        class="px-4 py-2 rounded-lg bg-white/5 text-gray-300 text-sm hover:bg-white/10 transition"
        @click="charger(false)"
      >Charger plus</button>
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
      </div>
    </div>

    <!-- Modal article (opaque) -->
    <div v-if="articleOuvert" class="fixed inset-0 z-50 flex items-center justify-center bg-black/30" @click.self="articleOuvert = null">
      <div class="w-full max-w-lg p-6 space-y-3 rounded-2xl border border-white/10 bg-[#16181d] shadow-2xl">
        <h3 class="font-bold text-white">{{ articleOuvert.titre_fr || articleOuvert.article.titre }}</h3>
        <p v-if="!articleOuvert.titre_fr" class="text-xs text-yellow-400">Traduction indisponible (réessai à la prochaine ouverture)</p>
        <p v-if="articleOuvert.sentiment" class="text-sm text-gray-400">Sentiment : {{ articleOuvert.sentiment }}</p>
        <a :href="articleOuvert.article.url" target="_blank" class="text-sm text-blue-400 hover:underline">Lire l'article source ↗</a>
        <div class="flex justify-end"><button class="px-4 py-2 rounded-lg bg-white/5 text-gray-300 text-sm" @click="articleOuvert = null">Fermer</button></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
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

const articles = ref<ArticlePresse[]>([])
const page = ref(1) // prochaine page à demander au backend (50 articles/page)
const aToutCharge = ref(false) // dernière page servie < 50 articles → rien de plus à charger
const sources = ref<Awaited<ReturnType<typeof presseApi.sources>>>([])
const enBrief = ref(false)
const erreurBrief = ref<string | null>(null)
const dernierBrief = ref<Awaited<ReturnType<typeof presseApi.briefs>>[number] | null>(null)
const articleOuvert = ref<{ article: ArticlePresse; titre_fr: string | null; sentiment: string | null } | null>(null)
const filtre = reactive({ q: '', theme: '', asset: '', lu: '' })
const modaleSources = ref(false)
const nouvelleSource = reactive({ nom: '', url: '' })
// Valeurs réellement produites par classer_theme (backend)
const themes = ['macro', 'crypto', 'metaux', 'autre']
const assets = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD', 'EURUSD', 'USDJPY', 'DAX', 'NAS100', 'SP500']

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

async function ouvrir(a: ArticlePresse) {
  try {
    const res = await presseApi.ouvrir(a.hash_titre)
    articleOuvert.value = res
    // Mise à jour en place (badge lu, statut) : un rechargement complet
    // replongerait la bibliothèque à la page 1 et perdrait les pages chargées.
    articles.value = articles.value.map(x => (x.hash_titre === a.hash_titre ? res.article : x))
    // Filtre « Non lus » : l'article désormais lu n'y appartient plus.
    if (filtre.lu === 'false') {
      articles.value = articles.value.filter(x => x.hash_titre !== a.hash_titre)
    }
  } catch (err: any) {
    // 410 Gone : article supprimé après traduction impossible ×2 — on referme et on le retire de la liste
    if (err?.response?.status === 410) {
      articleOuvert.value = null
      articles.value = articles.value.filter(x => x.hash_titre !== a.hash_titre)
      return
    }
    // Autre erreur réseau/serveur : on referme sans crash
    articleOuvert.value = null
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

async function ajouterSource() {
  if (!nouvelleSource.nom || !nouvelleSource.url.startsWith('https://')) return
  await presseApi.ajouterSource(nouvelleSource.nom, nouvelleSource.url, 30, 'marches')
  nouvelleSource.nom = ''; nouvelleSource.url = ''
  sources.value = await presseApi.sources()
}

async function retirerSource(id: number) {
  await presseApi.retirerSource(id)
  sources.value = await presseApi.sources()
}

onMounted(async () => {
  await charger()
  sources.value = await presseApi.sources()
  dernierBrief.value = (await presseApi.briefs())[0] ?? null
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
