<template>
  <!-- Hub de navigation (refonte 01/09) : 4 tuiles cliquables qui
       remplacent les menus de la barre de titre. Empilées dans la colonne
       gauche du dashboard (sous la surveillance assets), scroll interne
       si la fenêtre est basse. Chaque tuile ouvre sa page et affiche un
       aperçu live de son contenu. -->
  <div class="flex flex-col gap-2 flex-1 min-h-0 overflow-y-auto pr-0.5">
    <div
      v-for="t in tuiles"
      :key="t.id"
      class="rounded-xl border backdrop-blur-sm p-2.5 flex flex-col gap-1.5 shrink-0 cursor-pointer transition-colors"
      :class="TEINTES[t.id]"
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
          <span class="text-[9px] text-blue-400 font-mono shrink-0">{{ ageCourt(a) }}</span>
          <span class="text-[10px] text-white truncate cursor-help" :title="a.resume_source || a.titre">{{ a.titre_fr || a.titre }}</span>
        </div>
      </template>

      <!-- 📈 Graphiques : slots de la grille (cliquables) + alertes prix actives -->
      <template v-else-if="t.id === 'graphiques'">
        <div v-if="!slots.length" class="text-[10px] text-white leading-snug">Aucune grille sauvegardée — ouvrez la page Graphiques pour la composer</div>
        <div
          v-for="s in slots"
          :key="s.asset + s.timeframe"
          class="flex items-center gap-1.5 rounded px-0.5 -mx-0.5 hover:bg-white/10 transition-colors"
          title="Ouvrir ce graphique"
          @click.stop="ouvrirGraphique(s.asset, s.timeframe)"
        >
          <span class="text-[10px] font-semibold text-white w-16 shrink-0 truncate">{{ s.asset }}</span>
          <span class="text-[10px] font-mono text-white w-14 shrink-0 text-right">{{ formaterPrix(prixStore.getPrix(s.asset)) }}</span>
          <span class="text-[10px] font-mono shrink-0" :class="(variations[s.asset] ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">
            {{ variations[s.asset] === null || variations[s.asset] === undefined ? '' : `${(variations[s.asset] ?? 0) >= 0 ? '▲' : '▼'} ${Math.abs(variations[s.asset] ?? 0).toFixed(2)} %` }}
          </span>
        </div>
        <!-- Alertes prix actives (fusion de l'ancien bloc 🔔 dédié) -->
        <div v-if="alertesActives.length" class="mt-auto pt-1.5 border-t border-white/10 flex flex-col gap-1">
          <div v-for="a in alertesActives" :key="a.id" class="flex items-center gap-1.5" :title="titreAlerte(a)">
            <span class="text-[10px]">{{ a.sens === 'en_dessous' ? '🔻' : '🔺' }}</span>
            <span class="text-[10px] font-semibold text-white truncate">{{ a.asset }}</span>
            <span class="text-[10px] font-mono text-amber-300">{{ formaterPrix(a.prix) }}</span>
            <button class="ml-auto text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/60 text-white transition-colors" title="Ouvrir le graphique de cet asset" @click.stop="ouvrirGraphique(a.asset)">👁</button>
            <button class="text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-red-600/60 text-white transition-colors" title="Supprimer l'alerte (bloc + graphique)" @click.stop="supprimerAlerte(a)">✕</button>
          </div>
        </div>
      </template>

      <!-- 🧠 IA : modèle + dernière analyse + raccourcis -->
      <template v-else-if="t.id === 'ia'">
        <span class="text-[10px] text-white truncate">{{ modele ? `Modèle : ${modele}` : 'Statut IA indisponible' }} {{ ollamaOk === false ? '· Ollama ⚠️' : '' }}</span>
        <span v-if="derniereAnalyse" class="text-[10px] text-white truncate">🖼️ {{ derniereAnalyse.asset }} {{ derniereAnalyse.tf }} · il y a {{ ageTs(derniereAnalyse.ts) }}</span>
        <div class="mt-auto flex gap-1">
          <button v-for="r in raccourcisIa" :key="r.to" class="text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/60 text-white transition-colors" @click.stop="router.push(r.to)">{{ r.label }}</button>
        </div>
      </template>

      <!-- ⚙️ Système : raccourcis (l'état EA/Tiingo vit dans Data & IA Engine) -->
      <template v-else>
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
import { usePrixStore } from '@/stores/prix.store'
import { CLE_SLOTS, ciblerPremierSlot } from '@/utils/graphiques'
import type { AlertePrix } from '@/services/api.alertes'

const router = useRouter()
const prixStore = usePrixStore()

/// Teinte de chaque tuile — la couleur voyage jusqu'à la page ouverte.
const TEINTES: Record<string, string> = {
  presse: 'bg-sky-500/10 border-sky-500/25 hover:border-sky-400/50',
  graphiques: 'bg-emerald-500/10 border-emerald-500/25 hover:border-emerald-400/50',
  ia: 'bg-violet-500/10 border-violet-500/25 hover:border-violet-400/50',
  systeme: 'bg-rose-500/10 border-rose-500/25 hover:border-rose-400/50',
}

const tuiles = [
  { id: 'presse', icone: '📰', label: 'Revue de presse', route: '/presse' },
  { id: 'graphiques', icone: '📈', label: 'Graphiques', route: '/smc/graphiques' },
  { id: 'ia', icone: '🧠', label: 'Fonctionnalités IA', route: '/ia' },
  { id: 'systeme', icone: '📦', label: 'Données', route: '/donnees' },
] as const

const raccourcisIa = [
  { to: '/ia?tab=chart', label: '🖼️ Analyse' },
  { to: '/ia?tab=coach', label: '💬 Coach' },
  { to: '/ia?tab=prompts', label: '✏️ Prompts' },
]

const raccourcisSysteme = [
  { to: '/donnees?tab=risque', label: '📊 Risque' },
  { to: '/donnees?tab=connexions', label: '🔌 Connexions' },
]

// ── Presse : 3 derniers articles ─────────────────────────────────────────────
const articles = ref<ArticlePresse[]>([])

/// Âge court d'un article : « 4min », « 2h », « 3j ».
function ageCourt(a: ArticlePresse): string {
  const ts = Date.parse(a.publie_le) / 1000
  return Number.isFinite(ts) ? ageTs(ts) : ''
}

function ageTs(ts: number): string {
  const s = Math.max(0, Date.now() / 1000 - ts)
  if (s < 3600) return `${Math.max(1, Math.floor(s / 60))}mn`
  if (s < 86400) return `${Math.floor(s / 3600)}h`
  return `${Math.floor(s / 86400)}j`
}

// ── Graphiques : slots sauvegardés + prix + variations D1 ────────────────────
type Slot = { asset: string; timeframe: string }
const slots = ref<Slot[]>([])
const variations = ref<Record<string, number | null>>({})
const alertesActives = ref<AlertePrix[]>([])

/// Ouvre la page Graphiques sur un asset précis (premier slot ciblé).
function ouvrirGraphique(asset: string, timeframe?: string) {
  ciblerPremierSlot(asset, timeframe)
  router.push('/smc/graphiques')
}

function titreAlerte(a: AlertePrix): string {
  return `${a.asset} — ${a.sens === 'en_dessous' ? 'descente sous' : 'montée au-dessus de'} ${formaterPrix(a.prix)}${a.note ? ` · ${a.note}` : ''}`
}

async function supprimerAlerte(a: AlertePrix) {
  try {
    await alertesApi.supprimer(a.id)
    alertesActives.value = alertesActives.value.filter(x => x.id !== a.id)
  } catch { /* le prochain poll réaffichera l'état réel */ }
}

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
    alertesActives.value = alertes.filter(a => a.active)
    // Rattrapage (hérité de l'ancien bloc 🔔) : une alerte déclenchée ne
    // doit exister nulle part. Les graphs la suppriment en notifiant
    // (son + OS, poll 10 s) ; on nettoie ici les déclenchées de plus de
    // 2 minutes, fenêtre laissée aux charts pour la notification.
    const vieilles = alertes.filter(
      a => !a.active && a.declenchee_le && Date.now() / 1000 - a.declenchee_le > 120,
    )
    if (vieilles.length) {
      await Promise.all(vieilles.map(a => alertesApi.supprimer(a.id).catch(() => null)))
    }
  } catch { alertesActives.value = [] }

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
