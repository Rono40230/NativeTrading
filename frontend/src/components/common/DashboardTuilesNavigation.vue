<template>
  <!-- Hub de navigation (refonte 01/09) : 4 tuiles cliquables qui
       remplacent les menus de la barre de titre. Depuis le 14/09 elles
       vivent dans la colonne latérale unique du dashboard, en deux
       groupes (l'ordre est celui du propriétaire : presse entre le
       calendrier et le rapport d'activité, puis graphiques/IA/données).
       Chaque tuile ouvre sa page et affiche un aperçu live de son
       contenu ; `ids` sélectionne les tuiles rendues (et donc les
       données chargées — pas de requête pour une tuile absente). -->
  <div class="flex flex-col gap-2 shrink-0">
    <div
      v-for="t in tuilesAffichees"
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

      <!-- 📈 Graphiques : alertes prix actives (fusion de l'ancien bloc 🔔).
           La liste des graphiques ouverts est retirée (14/09) — la page
           Graphiques montre la grille. -->
      <template v-else-if="t.id === 'graphiques'">
        <div v-if="alertesActives.length" class="flex flex-col gap-1">
          <div v-for="a in alertesActives" :key="a.id" class="flex items-center gap-1.5" :title="titreAlerte(a)">
            <span class="text-[10px]">{{ a.sens === 'en_dessous' ? '🔻' : '🔺' }}</span>
            <span class="text-[10px] font-semibold text-white truncate">{{ a.asset }}</span>
            <span class="text-[10px] font-mono text-amber-300">{{ formaterPrix(a.prix) }}</span>
            <button class="ml-auto text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/60 text-white transition-colors" title="Ouvrir le graphique de cet asset" @click.stop="ouvrirGraphique(a.asset)">👁</button>
            <button class="text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-red-600/60 text-white transition-colors" title="Supprimer l'alerte (bloc + graphique)" @click.stop="supprimerAlerte(a)">✕</button>
          </div>
        </div>
      </template>

      <!-- 🧠 IA et 📦 Données : boutons simples (14/09) — les raccourcis
           vivent en onglets dans la page Fonctionnalités IA ; l'état
           EA/Tiingo vit dans Data & IA Engine. -->
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { presseApi, type ArticlePresse } from '@/services/api.presse'
import { alertesApi } from '@/services/api.alertes'
import { ciblerPremierSlot } from '@/utils/graphiques'
import type { AlertePrix } from '@/services/api.alertes'

/// Tuiles rendues par cette instance — le dashboard les place en deux
/// groupes depuis le 14/09 (presse entre calendrier et rapport, puis
/// graphiques/IA/données). Défaut : les quatre (comportement d'origine).
const props = withDefaults(defineProps<{ ids?: string[] }>(), {
  ids: () => ['presse', 'graphiques', 'ia', 'systeme'],
})
const affiche = (id: string) => props.ids.includes(id)

const router = useRouter()

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

const tuilesAffichees = computed(() => tuiles.filter(t => affiche(t.id)))

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

// ── Graphiques : alertes prix actives (l'ancienne liste des slots de la
//    grille est retirée le 14/09 — décision propriétaire) ────────────────────
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

// ── IA : plus d'aperçu (14/09) — la ligne « Modèle » est retirée, les
//    raccourcis vivent en onglets dans la page Fonctionnalités IA ────────────

async function chargerTout() {
  // Chaque instance ne charge que les données des tuiles qu'elle rend.
  if (affiche('presse')) {
    try {
      const liste = await presseApi.articles({ page: 1 })
      articles.value = [...liste]
        .sort((a, b) => Date.parse(b.publie_le) - Date.parse(a.publie_le))
        .slice(0, 3)
    } catch { articles.value = [] }
  }

  if (affiche('graphiques')) {
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
  }
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
