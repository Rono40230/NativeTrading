<template>
  <div class="glass-card px-4 py-2 flex flex-col gap-1.5">
    <div class="flex items-center justify-between shrink-0 gap-2">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">⏰ Créneaux de volatilité</p>
      <div class="flex items-center gap-1.5 min-w-0">
        <span class="text-[9px] text-white truncate">{{ jourLabel }} · heures Paris · 24 mois d'historique</span>
        <button
          class="h-5 w-5 shrink-0 flex items-center justify-center rounded bg-white/5 border border-white/10 hover:bg-white/10 text-[10px] text-white transition-colors"
          title="⚡ Radar ATR temps réel — volatilité actuelle par unité de temps"
          @click="ouvrirRadar"
        >⚡</button>
      </div>
    </div>

    <div v-if="chargement" class="text-center text-white text-xs py-3">Calcul…</div>
    <div v-else-if="!cartes.length" class="text-center text-white text-xs py-3">Aucune donnée</div>

    <template v-else>
      <!-- Cartes 2×3 : titre au format complet (repli naturel) + mini-barre -->
      <div class="grid grid-cols-2 gap-1.5">
        <div v-for="c in cartes" :key="c.asset"
          class="rounded-lg border border-white/10 bg-black/20 p-2 flex flex-col gap-1">
          <div class="text-[9px] leading-snug text-white" :title="titreLigne(c)">
            <span class="text-[11px] font-bold">{{ c.asset }}</span>
            <span> | Meilleur créneau horaire : </span>
            <span v-if="c.fenetre" class="font-mono font-semibold"
              :class="COULEUR_CLUSTER_TEXTE[c.fenetre.cluster]"
            >{{ c.fenetre.heureDebut }}h – {{ c.fenetre.heureFin }}h {{ NOM_CLUSTER[c.fenetre.cluster] }}</span>
            <span v-else class="font-mono">—</span>
            <span> | Meilleur jour : </span>
            <span class="font-mono font-semibold">{{ (c.meilleurJour ?? '—').toLowerCase() }}.</span>
          </div>

          <!-- Mini-barre : le jour type heure par heure, heure courante encadrée -->
          <div class="flex gap-[1px] h-2.5">
            <div v-for="h in 24" :key="c.asset + '-b' + h"
              class="flex-1 rounded-[1px]"
              :style="styleBarre(c, h - 1)"
            />
          </div>
        </div>
      </div>

      <!-- Légende -->
      <div class="flex items-center gap-2 text-[8px] text-white">
        <span v-for="(nom, i) in NOM_CLUSTER" :key="nom" class="flex items-center gap-1">
          <span class="w-3 h-2 rounded-[2px]" :style="{ background: COULEURS_CLUSTER_PLEIN[i] }" /> {{ nom.toLowerCase() }}
        </span>
        <span class="ml-auto">cadre blanc = heure en cours</span>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { apiService } from '@/services/api.service'
import type { ReponsePatternsVolatilite } from '@/services/api.types'
import type { PatternHoraire } from '@/services/api.types.marche'
import { offsetParisHeures } from '@/utils/date'
import { JOURS, COULEURS_CLUSTER, COULEURS_CLUSTER_PLEIN } from './heatmapConstants'
import { calculerAnalyse, NOM_CLUSTER, COULEUR_CLUSTER_TEXTE } from '@/composables/useVolatiliteAnalyse'

/// Source : patterns horaires (clusters quartiles) de tous les assets actifs —
/// 24 mois au M1, cache serveur d'une heure. Une CARTE par asset : statut
/// « maintenant », mini-barre du jour de semaine en cours (24 heures UTC du
/// jour placées à leur heure Paris — bijection), meilleure fenêtre du jour,
/// jour le plus actif de la semaine, seuil straddle P85.
interface CarteAsset {
  asset: string
  seuil: number
  maintenant: PatternHoraire | null
  barre: (PatternHoraire | null)[]
  fenetre: { heureDebut: number; heureFin: number; cluster: number } | null
  meilleurJour: string | null
}

const donnees = ref<ReponsePatternsVolatilite[]>([])
const chargement = ref(true)
const maintenant = ref(new Date())
const router = useRouter()

let horloge: ReturnType<typeof setInterval> | null = null

/** Ouvre la page Radar ATR (volatilité live par unité de temps). */
function ouvrirRadar() {
  router.push('/heatmap')
}

async function charger() {
  try {
    donnees.value = await apiService.obtenirPatternsJourTousActifs()
  } catch { /* silencieux */ } finally {
    chargement.value = false
  }
}

/// Jour de semaine courant, à Paris (convention données : 0=Dim … 6=Sam).
const jourCourant = computed(() =>
  new Date(maintenant.value.getTime() + offsetParisHeures() * 3_600_000).getUTCDay())
const jourLabel = computed(() => JOURS[jourCourant.value]?.label ?? '')

const heureParis = computed(() =>
  Number(new Intl.DateTimeFormat('en-US', { timeZone: 'Europe/Paris', hour: 'numeric', hour12: false }).format(maintenant.value)))

function uniteAsset(asset: string): string {
  return ['BTC', 'ETH'].includes(asset) ? '$' : 'pts'
}

/** Meilleure fenêtre contiguë du jour courant : fusion des heures adjacentes
 * du MÊME cluster (pas de propagation max — sinon tout le jour fusionne),
 * puis classement par cluster, durée, ATR. */
function fenetreDuJour(patterns: PatternHoraire[], jour: number) {
  const offset = offsetParisHeures()
  const cellules = patterns
    .filter(p => p.jour_semaine === jour && p.nb_points > 0)
    .map(p => ({ heureParis: (p.heure + offset) % 24, cluster: p.cluster, atr: p.atr_moyen }))
    .sort((a, b) => a.heureParis - b.heureParis)
  if (!cellules.length) return null
  const fenetres: { heureDebut: number; heureFin: number; cluster: number; atr: number }[] = []
  for (const c of cellules) {
    const l = fenetres.at(-1)
    if (l && c.heureParis === l.heureFin && c.cluster === l.cluster) {
      l.heureFin++
      l.atr = Math.max(l.atr, c.atr)
    } else {
      fenetres.push({ heureDebut: c.heureParis, heureFin: c.heureParis + 1, cluster: c.cluster, atr: c.atr })
    }
  }
  return fenetres.sort((a, b) =>
    b.cluster - a.cluster ||
    (b.heureFin - b.heureDebut) - (a.heureFin - a.heureDebut) ||
    b.atr - a.atr)[0]
}

const cartes = computed<CarteAsset[]>(() => {
  const offset = offsetParisHeures()
  const utcJour = maintenant.value.getUTCDay()
  const utcHeure = maintenant.value.getUTCHours()
  return donnees.value.map(d => {
    const analyse = calculerAnalyse(d.patterns)
    const barre: (PatternHoraire | null)[] = Array.from({ length: 24 }, () => null)
    for (const p of d.patterns) {
      if (p.jour_semaine === jourCourant.value && p.nb_points > 0) {
        barre[(p.heure + offset) % 24] = p
      }
    }
    return {
      asset: d.asset,
      seuil: d.seuil_straddle_calibre,
      maintenant: d.patterns.find(p => p.heure === utcHeure && p.jour_semaine === utcJour) ?? null,
      barre,
      fenetre: fenetreDuJour(d.patterns, jourCourant.value),
      meilleurJour: analyse?.meilleurJour.label ?? null,
    }
  })
})

function libelleMaintenant(c: CarteAsset): string {
  if (!c.maintenant) return '—'
  return `${NOM_CLUSTER[c.maintenant.cluster]} · ${c.maintenant.atr_moyen.toFixed(1)} ${uniteAsset(c.asset)}`
}

/** Infobulle de l'en-tête de carte : le détail complet des métriques. */
function titreLigne(c: CarteAsset): string {
  const fenetre = c.fenetre
    ? `${c.fenetre.heureDebut}h – ${c.fenetre.heureFin}h (${NOM_CLUSTER[c.fenetre.cluster]})`
    : '—'
  return `Meilleur créneau : ${fenetre} · Meilleur jour : ${c.meilleurJour ?? '—'} · Seuil straddle P85 : ${c.seuil.toFixed(1)} ${uniteAsset(c.asset)} · Maintenant : ${libelleMaintenant(c)}`
}

function styleBarre(c: CarteAsset, hParis: number): Record<string, string> {
  const p = c.barre[hParis]
  if (!p) return { background: '#ffffff08' }
  const style: Record<string, string> = { background: COULEURS_CLUSTER[p.cluster] ?? '#ffffff08' }
  if (hParis === heureParis.value) {
    style.border = '1px solid #ffffff'
    style.background = COULEURS_CLUSTER_PLEIN[p.cluster] ?? '#ffffff40'
  }
  return style
}

onMounted(() => {
  void charger()
  // Tick horloge : heure courante + bascule du jour à minuit Paris.
  horloge = setInterval(() => { maintenant.value = new Date() }, 30_000)
})

onUnmounted(() => {
  if (horloge !== null) clearInterval(horloge)
})
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
</style>
