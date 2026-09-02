<template>
  <div class="flex flex-col min-h-full bg-sky-500/5 rounded-xl px-3 py-2 -mx-1">
    <!-- Bandeau -->
    <div class="flex items-center justify-between shrink-0 mb-4">
      <h1 class="text-2xl font-bold">📰 Revue de presse</h1>
      <button
        class="px-3 py-1.5 rounded-lg bg-white/5 text-white text-sm hover:bg-white/10 transition"
        @click="modaleSources = true"
      >📡 Sources RSS</button>
    </div>

    <!-- Colonne unique : filtres, brief, cartes enrichies (le <main> d'App
         scrolle — la vue reste en flux simple, pas de double scroll) -->
    <div class="flex-1 space-y-4">

      <!-- ── Colonne unique : filtres, brief, cartes ── -->


        <!-- Brief repliable (bas de colonne) -->
        <details open class="glass-card p-4 shrink-0">
          <summary class="flex items-center justify-between cursor-pointer list-none gap-3 [&::-webkit-details-marker]:hidden">
            <span class="text-sm font-semibold text-white">
              📝 Brief 24 h
              <span v-if="briefParse" class="text-xs text-white font-normal">· {{ briefParse.articles.length }} articles</span>
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
                <p class="text-sm text-white leading-relaxed">{{ briefParse.contexte }}</p>
              </div>

              <!-- Articles du brief — cartes autonomes -->
              <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                <button
                  v-for="art in briefParse.articles"
                  :key="art.numero"
                  class="rounded-xl border border-white/10 bg-white/[0.04] p-4 flex flex-col gap-3 hover:bg-white/[0.07] hover:border-white/20 transition text-left relative overflow-hidden"
                  @click="ouvrirArticleBrief(art)"
                >
                  <div class="absolute top-0 left-0 right-0 h-1" :class="art.score >= 60 ? 'bg-red-400/70' : art.score >= 40 ? 'bg-yellow-400/70' : 'bg-gray-500/50'"></div>

                  <div class="flex items-start justify-between gap-2">
                    <span class="text-[10px] font-bold text-white bg-white/5 rounded-md px-1.5 py-0.5 shrink-0">#{{ art.numero }}</span>
                    <span class="text-lg font-bold tabular-nums shrink-0" :class="classeScore(art.score)">{{ art.score }}</span>
                  </div>

                  <h3 class="text-sm font-semibold text-white leading-snug line-clamp-3">{{ art.titre }}</h3>
                  <p class="text-xs text-white leading-relaxed line-clamp-4">{{ art.resume }}</p>

                  <div class="mt-auto flex flex-wrap items-center gap-1.5 text-[10px]">
                    <span class="px-1.5 py-0.5 rounded" :class="art.score >= 60 ? 'bg-red-500/15 text-red-300' : art.score >= 40 ? 'bg-yellow-500/15 text-yellow-300' : 'bg-white/10 text-white'">{{ art.score >= 60 ? 'fort' : art.score >= 40 ? 'moyen' : 'faible' }}</span>
                    <span class="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300">{{ art.theme }}</span>
                    <span class="px-1.5 py-0.5 rounded bg-white/10 text-white truncate max-w-[10rem]">{{ art.source }}</span>
                  </div>
                </button>
              </div>

              <p class="text-[10px] text-white text-right">Brief du {{ new Date(dernierBrief?.genere_le ? dernierBrief.genere_le * 1000 : Date.now()).toLocaleString('fr-FR') }} · {{ briefParse.articles.length }} articles</p>
            </template>

            <p v-else class="text-sm text-white">Aucun brief — clique « Générer » (Ollama, ~1 min).</p>
            <p v-if="erreurBrief" class="text-sm text-red-400">{{ erreurBrief }}</p>
          </div>
        </details>

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
            <option value="">Lu + non lus</option><option value="true">Lus</option><option value="false">Non lus</option>
          </select>
          <!-- articles.length = total chargé (toutes pages « Charger plus » confondues) -->
          <span class="text-xs text-white">{{ articles.length }} articles</span>
        </div>

        <!-- Bibliothèque — cartes enrichies (résumé FR intégré, design brief) -->
        <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4">
          <article
            v-for="a in articles"
            :key="a.hash_titre"
            class="rounded-xl border border-white/10 bg-white/[0.04] p-4 flex flex-col gap-2 hover:bg-white/[0.07] hover:border-white/20 transition relative overflow-hidden cursor-pointer"
            @click="lire(a)"
          >
            <div class="absolute top-0 left-0 right-0 h-1" :class="a.score >= 60 ? 'bg-red-400/70' : a.score >= 40 ? 'bg-yellow-400/70' : 'bg-gray-500/50'"></div>

            <div class="flex items-start justify-between gap-1">
              <span
                v-if="estNouveau(a.ajoute_le) && !a.lu"
                class="text-[9px] font-bold text-red-300 bg-red-600/40 border border-red-500/40 rounded-full px-1.5 py-0.5 leading-none animate-pulse shrink-0"
              >NOUVEAU</span>
              <span
                v-else-if="a.lu"
                class="text-[9px] font-semibold text-blue-200 bg-blue-600/70 border border-blue-500/50 rounded-full px-1.5 py-0.5 leading-none shrink-0"
              >Vu</span>
              <span class="ml-auto text-lg font-bold tabular-nums shrink-0" :class="classeScore(a.score)">{{ a.score }}</span>
            </div>

            <h3 class="text-sm font-semibold leading-snug line-clamp-2" :class="a.lu ? 'text-white' : 'text-white'">{{ a.titre_fr ?? a.titre }}</h3>
            <p v-if="resumeAffiche(a)" class="text-xs text-white leading-relaxed line-clamp-4">{{ resumeAffiche(a) }}</p>

            <div class="mt-auto flex flex-wrap items-center gap-1.5 text-[10px]">
              <span class="px-1.5 py-0.5 rounded" :class="a.impact === 'fort' ? 'bg-red-500/15 text-red-300' : a.impact === 'moyen' ? 'bg-yellow-500/15 text-yellow-300' : 'bg-white/10 text-white'">{{ a.impact }}</span>
              <span class="px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300">{{ a.theme }}</span>
              <span class="px-1.5 py-0.5 rounded bg-white/10 text-white truncate max-w-[8rem]">{{ a.source_nom }}</span>
            </div>
          </article>
        </div>
        <p v-if="articles.length === 0" class="text-sm text-white p-4">Bibliothèque vide — le collecteur remplit au prochain cycle (30 min).</p>
        <!-- Pagination : le backend sert 50 articles/page, on empile les pages suivantes -->
        <div v-if="!aToutCharge && articles.length > 0" class="flex justify-center shrink-0">
          <button
            class="px-4 py-2 rounded-lg bg-white/5 text-white text-sm hover:bg-white/10 transition"
            @click="charger(false)"
          >Charger plus</button>
        </div>
    </div>

    <!-- Modal sources (opaque, ouverte par le bouton du bandeau) -->
    <div v-if="modaleSources" class="fixed inset-0 z-50 flex items-center justify-center bg-black/30" @click.self="modaleSources = false">
      <div class="w-full max-w-lg p-6 space-y-4 rounded-2xl border border-white/10 bg-[#16181d] shadow-2xl">
        <div class="flex items-center justify-between">
          <h3 class="font-bold text-white">📡 Sources RSS</h3>
          <button class="text-white hover:text-white transition" @click="modaleSources = false">✕</button>
        </div>
        <div class="space-y-2 max-h-72 overflow-y-auto">
          <div v-for="s in sources" :key="s.id" class="flex items-center justify-between text-sm">
            <span :class="s.actif ? 'text-white' : 'text-white line-through'">{{ s.nom }} <span class="text-xs text-white">(poids {{ s.poids_score }})</span></span>
            <button class="text-red-400 hover:text-red-300 text-xs" @click="retirerSource(s.id)">Retirer</button>
          </div>
        </div>
        <div class="grid grid-cols-1 gap-2 pt-2 border-t border-white/5">
          <input v-model="nouvelleSource.nom" placeholder="Nom" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
          <input v-model="nouvelleSource.url" placeholder="https://flux.example/rss" class="bg-white/5 border border-white/10 rounded-lg px-3 py-1.5 text-sm text-white" />
        </div>
        <div class="flex gap-3 pt-1">
          <button class="flex-1 px-4 py-2 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 transition" @click="ajouterSource()">+ Ajouter</button>
          <button class="flex-1 px-4 py-2 rounded-lg bg-white/5 text-white text-sm hover:bg-white/10 transition" @click="modaleSources = false">Fermer</button>
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

/** Ouvre une URL externe : commande Tauri `ouvrir_url` (portal Flatpak puis
 * xdg-open) dans l'app native, `window.open` en dev navigateur. Un simple
 * <a target=_blank> ne fait rien en Tauri (pas de shell navigateur). */
async function ouvrirExterne(url: string) {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('ouvrir_url', { url })
  } catch {
    window.open(url, '_blank', 'noopener')
  }
}

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
  return score >= 60 ? 'text-red-300' : score >= 40 ? 'text-yellow-300' : 'text-white'
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

/** Traduit le résumé RSS en FR via Ollama (endpoint news partagé
 *  /api/news/traduire, voie courte `long=false`). Cette voie passe par le
 *  cache backend news_traductions : premier affichage ~1 s Ollama, suivants
 *  servis depuis le cache. On borne l'envoi à 3000 caractères (le GET passe
 *  le texte en query param). L'endpoint rend le texte ORIGINAL en cas
 *  d'échec (Ollama down, ou texte déjà français) : une « traduction »
 *  identique à l'extrait = échec → VO conservée à l'affichage, en silence. */
/** Clic sur une carte : marquage lu + MAJ de la carte en place (titre FR,
 *  badge Vu). Le contenu vit DANS la carte — plus de liseuse. */
async function lire(a: ArticlePresse) {
  try {
    const res = await presseApi.ouvrir(a.hash_titre)
    articles.value = articles.value.map(x => (x.hash_titre === a.hash_titre ? res.article : x))
    if (filtre.lu === 'false') {
      articles.value = articles.value.filter(x => x.hash_titre !== a.hash_titre)
    }
  } catch (err: any) {
    if (err?.response?.status === 410) {
      articles.value = articles.value.filter(x => x.hash_titre !== a.hash_titre)
    }
  }
}

/** Résumé affiché dans la carte : FR servi sinon résumé VO. */
function resumeAffiche(a: ArticlePresse): string {
  return (a as any).resume_fr ?? a.resume_source ?? ''
}

/** Titre normalisé : minuscules, sans accents ni ponctuation. */
function normaliserTitre(t: string): string {
  return t
    .normalize('NFD').replace(/[\u0300-\u036f]/g, '')
    .toLowerCase().replace(/[^a-z0-9 ]/g, ' ').replace(/\s+/g, ' ').trim()
}

/** Correspondance approximative brief → bibliothèque. Le brief affiche des
 * titres FR (traduits par le LLM), la bibliothèque les titres VO : le
 * matching par mots est voué à l'échec entre langues. On s'appuie donc sur
 * (1) l'inclusion stricte (mêmes titres), puis (2) source identique + score
 * identique (chaque article du brief cite sa source et son score, qui
 * viennent directement de la bibliothèque — c'est une clé fiable). */
function trouverArticleBibliotheque(art: ArticleBrief): ArticlePresse | null {
  // (1) Titre strictement identique (FR/FR ou VO/VO)
  const cible = normaliserTitre(art.titre)
  if (cible) {
    for (const a of articles.value) {
      for (const t of [a.titre_fr, a.titre]) {
        if (t && normaliserTitre(t) === cible) return a
      }
    }
  }
  // (2) Source + score : clé stable entre le brief et la bibliothèque.
  // Le brief compile "- [score/100|theme] titre (source)" — le score et la
  // source viennent de presse_articles à la génération.
  const candidats = articles.value.filter(a =>
    a.source_nom === art.source && a.score === art.score)
  if (candidats.length > 0) return candidats[0]
  return null
}

/** Carte du brief : ouvre l'article bibliothèque correspondant dans la
 * liseuse (recherche par titre approximatif) ; à défaut affiche le résumé
 * LLM du brief seul (les articles du brief n'ont pas d'URL source). */
function ouvrirArticleBrief(art: ArticleBrief) {
  // Les cartes du brief portent déjà leur résumé LLM en FR — la liseuse a
  // été supprimée (contenu dans les cartes), rien à ouvrir.
  void art
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
    // Avertissement si le flux n'inclut pas de description (pas de résumé)
    messageSource.value = res.description_incluse
      ? { ok: true, texte: `✅ Source ajoutée — ${res.items_avec_description}/${res.items_testes} items avec description` }
      : { ok: false, texte: `⚠️ Source ajoutée MAIS ce flux n'a AUCUNE description (${res.items_testes} items testés) — les articles n'afficheront que leur titre. Considère un flux alternatif.` }
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
