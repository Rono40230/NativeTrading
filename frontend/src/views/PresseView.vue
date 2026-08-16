<template>
  <div class="space-y-6">
    <h1 class="text-2xl font-bold">📰 Revue de presse</h1>

    <!-- Brief -->
    <div class="glass-card p-5 space-y-3">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Brief 24 h</h2>
        <button
          class="px-3 py-1.5 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 disabled:opacity-40"
          :disabled="enBrief" @click="genererBrief()"
        >{{ enBrief ? '⏳ Génération…' : '⚡ Générer le brief' }}</button>
      </div>
      <div v-if="dernierBrief" class="text-sm text-gray-200 whitespace-pre-line">{{ sansCloture(dernierBrief.contenu) }}</div>
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
        <option value="">Lu + non lus</option><option value="true">Lus</option><option value="false">Non lus</option>
      </select>
      <span class="text-xs text-gray-500">{{ articles.length }} articles</span>
    </div>

    <!-- Bibliothèque -->
    <div class="glass-card p-2 divide-y divide-white/5">
      <button v-for="a in articles" :key="a.hash_titre" class="w-full text-left px-3 py-2.5 hover:bg-white/5 transition" @click="ouvrir(a)">
        <div class="flex items-center justify-between gap-3">
          <span class="text-sm" :class="a.lu ? 'text-gray-500' : 'text-white font-medium'">{{ a.titre }}</span>
          <span class="text-xs text-gray-500 shrink-0">{{ a.source_nom }}</span>
        </div>
        <div class="flex gap-2 mt-1 text-[10px]">
          <span class="px-1.5 py-0.5 rounded" :class="a.impact === 'fort' ? 'bg-red-500/15 text-red-300' : a.impact === 'moyen' ? 'bg-yellow-500/15 text-yellow-300' : 'bg-white/10 text-gray-400'">{{ a.impact }}</span>
          <span class="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300">{{ a.theme }}</span>
          <span v-for="asset in parseAssets(a)" :key="asset" class="px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-300">{{ asset }}</span>
        </div>
      </button>
      <p v-if="articles.length === 0" class="text-sm text-gray-500 p-4">Bibliothèque vide — le collecteur remplit au prochain cycle (30 min).</p>
    </div>

    <!-- Sources -->
    <div class="glass-card p-5 space-y-3">
      <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Sources RSS</h2>
      <div v-for="s in sources" :key="s.id" class="flex items-center justify-between text-sm">
        <span :class="s.actif ? 'text-gray-300' : 'text-gray-600 line-through'">{{ s.nom }} <span class="text-xs text-gray-500">(poids {{ s.poids_score }})</span></span>
        <button class="text-red-400 hover:text-red-300 text-xs" @click="retirerSource(s.id)">Retirer</button>
      </div>
      <div class="flex gap-2 pt-2 border-t border-white/5">
        <input v-model="nouvelleSource.nom" placeholder="Nom" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
        <input v-model="nouvelleSource.url" placeholder="https://flux.example/rss" class="flex-1 bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
        <button class="px-3 py-1.5 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm" @click="ajouterSource()">+ Ajouter</button>
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
import { onMounted, reactive, ref } from 'vue'
import { presseApi, type ArticlePresse } from '@/services/api.presse'

const articles = ref<ArticlePresse[]>([])
const sources = ref<Awaited<ReturnType<typeof presseApi.sources>>>([])
const enBrief = ref(false)
const erreurBrief = ref<string | null>(null)
const dernierBrief = ref<Awaited<ReturnType<typeof presseApi.briefs>>[number] | null>(null)
const articleOuvert = ref<{ article: ArticlePresse; titre_fr: string | null; sentiment: string | null } | null>(null)
const filtre = reactive({ q: '', theme: '', asset: '', lu: '' })
const nouvelleSource = reactive({ nom: '', url: '' })
// Valeurs réellement produites par classer_theme (backend)
const themes = ['macro', 'crypto', 'metaux', 'autre']
const assets = ['BTC', 'ETH', 'XAUUSD', 'XAGUSD', 'EURUSD', 'USDJPY', 'DAX', 'NAS100', 'SP500']

/** Dépouille une éventuelle clôture markdown ```...``` du contenu du brief avant affichage. */
function sansCloture(c: string): string {
  return c.replace(/^\s*```(?:markdown)?\s*\n?/, '').replace(/\n?\s*```\s*$/, '')
}

function parseAssets(a: ArticlePresse): string[] {
  try { return JSON.parse(a.assets_concernes) } catch { return [] }
}

async function charger() {
  articles.value = await presseApi.articles({
    q: filtre.q || undefined, theme: filtre.theme || undefined,
    asset: filtre.asset || undefined, lu: filtre.lu || undefined,
  })
}

async function ouvrir(a: ArticlePresse) {
  try {
    articleOuvert.value = await presseApi.ouvrir(a.hash_titre)
    await charger() // rafraîchir lu/badges
  } catch (err: any) {
    // 410 Gone : article supprimé après traduction impossible ×2 — on referme et rafraîchit la liste
    if (err?.response?.status === 410) {
      articleOuvert.value = null
      await charger()
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
