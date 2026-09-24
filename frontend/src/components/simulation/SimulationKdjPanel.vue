<template>
  <div class="flex flex-col gap-3">
    <!-- ═══ 7.G — Balayage de calibration (rejeu 24 mois H1) ═══ -->
    <section class="glass-card p-4 flex flex-col gap-3">
      <div class="flex items-center gap-2 flex-wrap">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">Calibrage des paramètres — rejeu 24 mois H1</h2>
        <span class="text-[10px] text-white/60">un paramètre à la fois autour du réglage actuel · frais 0,05 %/ordre · validé propriétaire 24/09</span>
      </div>
      <p class="text-[11px] text-white/60">
        Réglage actuel : period {{ params.period }} · signal {{ params.signal }} · amplitude {{ params.amplitude }} ·
        ratio_risk {{ params.ratio_risk }} · adx_min {{ params.adx_min }} — calibrer AVANT la mesure 30 trades (7.F).
      </p>
      <!-- Périmètre d'étude (virtuel — ne touche pas à l'armement : le
           tableau complet reste l'outil de décision AVANT d'armer). -->
      <div class="flex flex-col gap-1.5">
        <p class="text-[10px] font-semibold uppercase tracking-wider text-white/60">Paires étudiées <span class="font-normal normal-case text-white/40">— vide = toutes celles avec historique H1</span></p>
        <div class="flex flex-wrap gap-1">
          <button v-for="a in assetsDispo" :key="'ka' + a"
                  class="text-[10px] px-1.5 py-0.5 rounded border font-mono transition-colors"
                  :class="filtreAssets.includes(a) ? 'border-teal-400/50 bg-teal-500/20 text-white' : 'border-white/10 bg-white/[0.03] text-white/50 hover:border-white/25'"
                  @click="basculer(a)">{{ a }}</button>
        </div>
      </div>

      <div class="flex gap-2 flex-wrap">
        <button class="btn-action bg-teal-500/20 text-teal-300 hover:bg-teal-500/30 disabled:opacity-40"
                :disabled="enCours" @click="balayer">
          {{ enCours ? '⏳ Balayage… (~1-2 min)' : '📊 Balayer les paramètres' }}</button>
      </div>
      <p v-if="erreur" class="text-[11px] text-red-400">{{ erreur }}</p>

      <div v-for="(lignes, param) in resultat?.balayages" :key="param" class="flex flex-col gap-1">
        <p class="text-[11px] font-semibold text-white/80 uppercase tracking-wider pt-1">{{ etiquettes[param] ?? param }}</p>
        <div class="overflow-x-auto">
          <table class="w-full text-xs">
            <thead>
              <tr class="text-white/60 uppercase tracking-wider border-b border-white/10">
                <th class="text-right py-1.5 pr-2">Valeur</th>
                <th class="text-right py-1.5 px-2">Trades</th>
                <th class="text-right py-1.5 px-2">WR</th>
                <th class="text-right py-1.5 px-2">PF</th>
                <th class="text-right py-1.5 px-2">Σ R net</th>
                <th class="text-right py-1.5 pl-2">R net/trade</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="l in lignes" :key="String(l.valeur)"
                  class="border-b border-white/5"
                  :class="l.actuel ? 'bg-teal-500/10' : ''"
                  :title="infobulle(l)">
                <td class="py-1.5 pr-2 text-right font-mono"
                    :class="l.actuel ? 'text-teal-300 font-bold' : 'text-white'">
                  {{ formatValeur(l.valeur) }}{{ l.actuel ? ' ·actuel' : '' }}{{ estMeilleur(lignes, l) ? ' 🏆' : '' }}
                </td>
                <td class="py-1.5 px-2 text-right font-mono text-white/70">{{ l.trades }}</td>
                <td class="py-1.5 px-2 text-right font-mono text-white/70">{{ l.wr }} %</td>
                <td class="py-1.5 px-2 text-right font-mono text-white/70">{{ l.pf }}</td>
                <td class="py-1.5 px-2 text-right font-mono" :class="l.r_net_total >= 0 ? 'text-emerald-400' : 'text-red-400'">
                  {{ l.r_net_total >= 0 ? '+' : '−' }}{{ Math.abs(l.r_net_total).toFixed(1) }} R</td>
                <td class="py-1.5 pl-2 text-right font-mono" :class="l.r_net_par_trade >= 0 ? 'text-emerald-400' : 'text-red-400'">
                  {{ (l.r_net_par_trade >= 0 ? '+' : '−') + Math.abs(l.r_net_par_trade).toFixed(3) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <!-- Par actif, réglage actuel — LA lecture du balayage : l'écart
           entre actifs (forex vs crypto) dépasse celui de tous les
           paramètres balayés. -->
      <div v-if="configActuelle" class="flex flex-col gap-1">
        <p class="text-[11px] font-semibold text-white/80 uppercase tracking-wider pt-1">Par actif — réglage actuel (la lecture clé)</p>
        <div class="overflow-x-auto">
          <table class="w-full text-xs">
            <thead>
              <tr class="text-white/60 uppercase tracking-wider border-b border-white/10">
                <th class="text-left py-1.5 pr-2">Actif</th>
                <th class="text-right py-1.5 px-2">Trades</th>
                <th class="text-right py-1.5 px-2">Σ R net</th>
                <th class="text-right py-1.5 pl-2">R net/trade</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="a in parActifTriee" :key="a.asset" class="border-b border-white/5">
                <td class="py-1.5 pr-2 font-mono text-white">{{ a.asset }}</td>
                <td class="py-1.5 px-2 text-right font-mono text-white/70">{{ a.trades }}</td>
                <td class="py-1.5 px-2 text-right font-mono" :class="a.r_net_total >= 0 ? 'text-emerald-400' : 'text-red-400'">
                  {{ a.r_net_total >= 0 ? '+' : '−' }}{{ Math.abs(a.r_net_total).toFixed(1) }} R</td>
                <td class="py-1.5 pl-2 text-right font-mono" :class="rtr(a) >= 0 ? 'text-emerald-400' : 'text-red-400'">
                  {{ (rtr(a) >= 0 ? '+' : '−') + Math.abs(rtr(a)).toFixed(3) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <p v-if="resultat" class="text-[10px] text-white/50">
        Rejeu multi-actifs : {{ resultat.actifs.join(' · ') }}. Le 🏆 marque le meilleur R net/trade du paramètre —
        attention à la règle des 30 : une valeur gagnante sur peu de trades n'est pas une preuve. NB : en unités R,
        les frais pèsent d'autant plus lourd que le risque (distance EMA200) est serré — comparer les actifs entre eux
        reste valable, comparer ce R à un %/trade MT5 ne l'est pas.
      </p>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'
import { useAlerteStore } from '@/stores/alerte.store'
import { useAssetsStore } from '@/stores/assets.store'

/// Balayage de calibration KDJ (7.G, 24/09) — le rejeu 24 mois H1 validé,
/// rejoué un paramètre à la fois autour du réglage actuel. Calibrer avant
/// la mesure 30 trades (7.F) ; le comparatif vécu viendra plus tard.
interface LigneKdj {
  valeur: number | string
  actuel: boolean
  trades: number
  wr: number
  pf: number
  r_net_total: number
  r_net_par_trade: number
  par_asset: { asset: string; trades: number; ouverts: number; r_net_total: number }[]
}
interface BalayageKdj {
  parametres_actuels: Record<string, number | string>
  frais: string
  fenetre_mois: number
  actifs: string[]
  balayages: Record<string, LigneKdj[]>
}

const alerteStore = useAlerteStore()
const assetsStore = useAssetsStore()
const assetsDispo = ref<string[]>([])
const filtreAssets = ref<string[]>([])
const enCours = ref(false)
const erreur = ref('')
const resultat = ref<BalayageKdj | null>(null)
const params = ref<Record<string, number | string>>({ period: 20, signal: 7, amplitude: 2, ratio_risk: 2.0, adx_min: 'off' })

onMounted(async () => {
  if (!assetsStore.assets.length) await assetsStore.chargerAssets()
  assetsDispo.value = assetsStore.assets.map(a => a.id).sort()
})

function basculer(a: string) {
  filtreAssets.value = filtreAssets.value.includes(a)
    ? filtreAssets.value.filter(x => x !== a)
    : [...filtreAssets.value, a]
}

const etiquettes: Record<string, string> = {
  period: 'Period (fenêtre KDJ)',
  signal: 'Signal (lissage K/D)',
  amplitude: 'Amplitude (HalfTrend)',
  ratio_risk: 'RatioRisk (TP × distance EMA200)',
  adx_min: 'ADX minimum (filtre tendance)',
}

/// Ligne du réglage actuel (présente dans chaque balayage) — détail par actif.
const configActuelle = computed(() => {
  if (!resultat.value) return null
  for (const lignes of Object.values(resultat.value.balayages)) {
    const l = lignes.find(x => x.actuel)
    if (l) return l
  }
  return null
})
const parActifTriee = computed(() =>
  [...(configActuelle.value?.par_asset ?? [])].sort((a, b) => a.r_net_total - b.r_net_total)
)
function rtr(a: { r_net_total: number; trades: number }): number {
  return a.trades ? a.r_net_total / a.trades : 0
}

async function balayer() {
  enCours.value = true
  erreur.value = ''
  try {
    const res = await http.post('/api/strategies/kdj_halftrend/simulation/balayage',
      { assets: filtreAssets.value }, { timeout: 300_000 })
    if (res.data?.erreur) {
      erreur.value = res.data.erreur
      return
    }
    resultat.value = res.data
    params.value = res.data.parametres_actuels
  } catch (e) {
    alerteStore.afficherErreur(`Balayage KDJ : ${(e as Error).message}`)
  } finally {
    enCours.value = false
  }
}

function formatValeur(v: number | string): string {
  return typeof v === 'number' ? String(v).replace('.', ',') : v
}

function estMeilleur(lignes: LigneKdj[], l: LigneKdj): boolean {
  const actuelle = lignes.find(x => x.actuel)
  if (!actuelle) return false
  const meilleure = lignes.reduce((a, b) => (b.r_net_par_trade > a.r_net_par_trade ? b : a))
  return meilleure === l && l.r_net_par_trade > actuelle.r_net_par_trade
}

function infobulle(l: LigneKdj): string {
  return l.par_asset
    .filter(a => a.trades > 0)
    .map(a => `${a.asset} : ${a.trades} tr · ${a.r_net_total >= 0 ? '+' : '−'}${Math.abs(a.r_net_total).toFixed(1)} R`)
    .join('\n') || 'aucun trade'
}
</script>
