<template>
  <!-- Hub de navigation (refonte 01/09) : 4 tuiles cliquables qui
       remplacent les menus de la barre de titre. Chaque tuile ouvre sa
       page et affiche un aperçu live de son contenu. -->
  <div class="grid grid-cols-4 gap-2">
    <div
      v-for="t in tuiles"
      :key="t.id"
      class="glass-card p-2.5 flex flex-col gap-1.5 min-h-[118px] cursor-pointer transition-colors hover:bg-white/10 hover:border-white/20"
      @click="router.push(t.route)"
    >
      <div class="flex items-center gap-1.5">
        <span class="text-sm leading-none">{{ t.icone }}</span>
        <span class="text-[11px] font-bold uppercase tracking-wider text-white truncate">{{ t.label }}</span>
      </div>

      <!-- 📰 Presse : 3 derniers titres -->
      <template v-if="t.id === 'presse'">
        <div v-if="!articles.length" class="text-[10px] text-white leading-snug">Bibliothèque vide — le collecteur remplit au prochain cycle (30 min)</div>
        <div v-for="a in articles" :key="a.hash_titre" class="flex items-center gap-1.5 min-w-0">
          <span class="text-[9px] text-white font-mono shrink-0">{{ ageCourt(a) }}</span>
          <span class="text-[10px] text-white truncate">{{ a.titre_fr || a.titre }}</span>
          <span v-if="badgeAsset(a)" class="text-[9px] font-mono font-bold text-amber-300 shrink-0">{{ badgeAsset(a) }}</span>
        </div>
        <span v-if="articles.length" class="mt-auto text-[9px] text-white">Collecte il y a {{ ageCourt(articles[0]) }}</span>
      </template>

      <!-- 📈 Graphiques : slots de la grille + prix + variation -->
      <template v-else-if="t.id === 'graphiques'">
        <div v-if="!slots.length" class="text-[10px] text-white leading-snug">Aucune grille sauvegardée — ouvrez la page Graphiques pour la composer</div>
        <div v-for="s in slots" :key="s.asset + s.timeframe" class="flex items-center gap-1.5">
          <span class="text-[10px] font-semibold text-white w-16 shrink-0 truncate">{{ s.asset }}</span>
          <span class="text-[10px] font-mono text-white w-14 shrink-0 text-right">{{ formaterPrix(prixStore.getPrix(s.asset)) }}</span>
          <span class="text-[10px] font-mono shrink-0" :class="(variations[s.asset] ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ variations[s.asset] === null || variations[s.asset] === undefined ? '' : `${(variations[s.asset] ?? 0) >= 0 ? '▲' : '▼'} ${Math.abs(variations[s.asset] ?? 0).toFixed(2)} %` }}
          </span>
        </div>
        <span class="mt-auto text-[9px] text-white">🔔 {{ nbAlertes }} alerte(s) active(s)</span>
      </template>

      <!-- 🧠 IA : modèle + dernière analyse + raccourcis -->
      <template v-else-if="t.id === 'ia'">
        <span class="text-[10px] text-white truncate">{{ modele ? `Modèle : ${modele}` : 'Statut IA indisponible' }} {{ ollamaOk === false ? '· Ollama ⚠️' : '' }}</span>
        <span v-if="derniereAnalyse" class="text-[10px] text-white truncate">🖼️ {{ derniereAnalyse.asset }} {{ derniereAnalyse.tf }} · il y a {{ ageTs(derniereAnalyse.ts) }}</span>
        <span v-else class="text-[10px] text-white">Aucune analyse graphique encore</span>
        <div class="mt-auto flex gap-1">
          <button v-for="r in raccourcisIa" :key="r.to" class="text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/60 text-white transition-colors" @click.stop="router.push(r.to)">{{ r.label }}</button>
        </div>
      </template>

      <!-- ⚙️ Système : MT5 + Tiingo + raccourcis -->
      <template v-else>
        <span class="text-[10px] text-white truncate">{{ mt5Ok === null ? 'EA MT5 : vérification…' : mt5Ok ? '🟢 EA MT5 connecté' : '🔴 EA MT5 déconnecté' }}</span>
        <span class="text-[10px] text-white truncate">{{ tiingo ? `Tiingo actions : ${tiingo}` : 'Tiingo actions : indisponible' }}</span>
        <div class="mt-auto flex gap-1">
          <button v-for="r in raccourcisSysteme" :key="r.to" class="text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/60 text-white transition-colors" @click.stop="router.push(r.to)">{{ r.label }}</button>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { presseApi, type ArticlePresse } from '@/services/api.presse'
import { alertesApi } from '@/services/api.alertes'
import { apiService } from '@/services/api.service'
import { http } from '@/services/http.client'
import { usePrixStore } from '@/stores/prix.store'
import { CLE_SLOTS } from '@/utils/graphiques'

const router = useRouter()
const prixStore = usePrixStore()

const tuiles = [
  { id: 'presse', icone: '📰', label: 'Revue de presse', route: '/presse' },
  { id: 'graphiques', icone: '📈', label: 'Graphiques', route: '/smc/graphiques' },
  { id: 'ia', icone: '🧠', label: 'Fonctionnalités IA', route: '/ia' },
  { id: 'systeme', icone: '⚙️', label: 'Système', route: '/systeme' },
] as const

const raccourcisIa = [
  { to: '/ia?tab=chart', label: '🖼️ Analyse' },
  { to: '/ia?tab=coach', label: '💬 Coach' },
  { to: '/ia?tab=prompts', label: '✏️ Prompts' },
]

const raccourcisSysteme = [
  { to: '/systeme?tab=settings', label: '⚙️ Paramètres' },
  { to: '/systeme?tab=data', label: '📦 Données' },
]

// ── Presse : 3 derniers articles ─────────────────────────────────────────────
const articles = ref<ArticlePresse[]>([])

function badgeAsset(a: ArticlePresse): string {
  try {
    const assets = JSON.parse(a.assets_concernes || '[]') as string[]
    return Array.isArray(assets) && assets.length ? assets[0] : ''
  } catch { return '' }
}

/// Âge court d'un article : « 4min », « 2h », « 3j ».
function ageCourt(a: ArticlePresse): string {
  const ts = Date.parse(a.publie_le) / 1000
  return Number.isFinite(ts) ? ageTs(ts) : ''
}

function ageTs(ts: number): string {
  const s = Math.max(0, Date.now() / 1000 - ts)
  if (s < 3600) return `${Math.max(1, Math.floor(s / 60))}min`
  if (s < 86400) return `${Math.floor(s / 3600)}h`
  return `${Math.floor(s / 86400)}j`
}

// ── Graphiques : slots sauvegardés + prix + variations D1 ────────────────────
type Slot = { asset: string; timeframe: string }
const slots = ref<Slot[]>([])
const variations = ref<Record<string, number | null>>({})
const nbAlertes = ref(0)

function formaterPrix(p: number | null): string {
  if (p === null) return '—'
  if (p >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(p)
  if (p >= 1) return p.toFixed(2)
  return p.toFixed(4)
}

// ── IA : modèle actif + dernière analyse (persistée par useChartImport) ──────
const modele = ref('')
const ollamaOk = ref<boolean | null>(null)
const derniereAnalyse = ref<{ asset: string; tf: string; ts: number } | null>(null)

// ── Système : EA MT5 + Tiingo ────────────────────────────────────────────────
const mt5Ok = ref<boolean | null>(null)
const tiingo = ref('')

async function chargerTout() {
  try {
    const liste = await presseApi.articles({ page: 1 })
    articles.value = [...liste]
      .sort((a, b) => Date.parse(b.publie_le) - Date.parse(a.publie_le))
      .slice(0, 3)
  } catch { articles.value = [] }

  try {
    const slotsLus = JSON.parse(localStorage.getItem(CLE_SLOTS) ?? '[]') as Slot[]
    slots.value = Array.isArray(slotsLus) ? slotsLus.slice(0, 6) : []
  } catch { slots.value = [] }

  try {
    const alertes = await alertesApi.lister()
    nbAlertes.value = alertes.filter(a => a.active).length
  } catch { nbAlertes.value = 0 }

  // Variation journalière (D1) de chaque asset de la grille.
  await Promise.allSettled(slots.value.map(async s => {
    try {
      const bougies = await apiService.getCandles(s.asset, 'D1', 2)
      const a = bougies.at(-1)?.close
      const b = bougies.at(-2)?.close
      variations.value[s.asset] = a != null && b != null && b !== 0 ? ((a - b) / b) * 100 : null
    } catch { variations.value[s.asset] = null }
  }))

  try {
    const ia = await apiService.statutIA()
    modele.value = ia.modele ?? ''
    ollamaOk.value = ia.ollama_disponible
  } catch { modele.value = ''; ollamaOk.value = false }

  try {
    derniereAnalyse.value = JSON.parse(localStorage.getItem('derniere_analyse_graphique') ?? 'null')
  } catch { derniereAnalyse.value = null }

  try {
    const r = await http.get('/api/mt5/statut')
    mt5Ok.value = !!r.data?.connecte
  } catch { mt5Ok.value = false }
  try {
    const r = await http.get('/api/rockets/actions/backfill/etat')
    const d = r.data as { univers_avec_bougies?: number; univers_total?: number }
    tiingo.value = `${d.univers_avec_bougies ?? 0}/${d.univers_total ?? 0}`
  } catch { tiingo.value = '' }
}

let poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void chargerTout()
  poll = setInterval(chargerTout, 60_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
