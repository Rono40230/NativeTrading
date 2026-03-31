<template>
  <div class="flex items-center gap-3 flex-wrap">
    <div v-if="dernierPrix !== null" class="flex items-baseline gap-3">
      <span class="text-3xl font-bold">{{ formatPrix(dernierPrix) }}</span>
      <span class="text-sm" :class="variation >= 0 ? 'text-emerald-400' : 'text-red-400'">
        {{ variation >= 0 ? '+' : '' }}{{ variation.toFixed(2) }}%
      </span>
      <span class="text-xs text-gray-500">
        {{ selectedAsset.includes('USD') ? selectedAsset : `${selectedAsset}/USDT` }} · {{ selectedTimeframe }}
      </span>
      <span v-if="wsConnecte" class="flex items-center gap-1 text-xs ml-2"
        :class="['BTC','ETH'].includes(selectedAsset) ? 'text-emerald-400' : 'text-blue-400'">
        <span class="w-1.5 h-1.5 rounded-full animate-pulse inline-block"
          :class="['BTC','ETH'].includes(selectedAsset) ? 'bg-emerald-400' : 'bg-blue-400'" />
        {{ ['BTC','ETH'].includes(selectedAsset) ? 'LIVE' : 'LIVE 5s' }}
      </span>
    </div>

    <div v-if="stats" class="flex items-center gap-2 ml-auto">
      <!-- Dropdown Asset -->
      <select
        :value="selectedAsset"
        class="px-2 py-1 text-xs font-medium rounded-lg bg-white text-black border border-white/20 cursor-pointer focus:outline-none"
        @change="$emit('changer-asset', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
      </select>

      <!-- Dropdown Timeframe -->
      <select
        :value="selectedTimeframe"
        class="px-2 py-1 text-xs font-medium rounded-lg bg-white text-black border border-white/20 cursor-pointer focus:outline-none"
        @change="$emit('changer-timeframe', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="tf in timeframes" :key="tf" :value="tf">{{ tf }}</option>
      </select>

      <div v-for="b in badges" :key="b.label"
        class="cpx-badge"
        @mouseenter="(e) => ouvrirTip(e, b)"
        @mouseleave="fermerTip"
      >
        <span class="text-xs text-slate-400 leading-none">{{ b.label }}</span>
        <span class="text-sm font-semibold mt-1" :class="b.color">{{ b.valeur }}</span>
      </div>
    </div>
  </div>

  <Teleport to="body">
    <Transition name="cpx-tip">
      <div v-if="tip.visible" class="cpx-tooltip"
        :style="{ left: tip.x + 'px', top: tip.y + 'px' }">
        <p class="cpx-def">{{ tip.def }}</p>
        <div class="cpx-sep" />
        <div class="cpx-echelle">
          <div v-for="n in tip.niveaux" :key="n.label"
            class="cpx-niveau"
            :class="{ 'cpx-actif': n.actif }"
            :style="n.actif ? { borderColor: n.color, color: n.color, background: n.color + '20' } : {}">
            <span class="cpx-bullet">{{ n.actif ? '▶' : '·' }}</span>
            {{ n.label }}
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, reactive } from 'vue'

const props = defineProps<{
  dernierPrix: number | null
  variation: number
  stats: {
    count: number; high: number; low: number; volumeMoy: number
    range: number; positionRange: number; volRelatif: number; vwap: number
  } | null
  selectedAsset: string
  selectedTimeframe: string
  wsConnecte: boolean
  assets: string[]
  timeframes: string[]
}>()

defineEmits<{
  'changer-asset': [asset: string]
  'changer-timeframe': [tf: string]
}>()

interface Niveau { label: string; color: string; actif: boolean }
interface BadgeTip { def: string; niveaux: Niveau[] }
interface Badge { label: string; valeur: string; color: string; tip: BadgeTip }

const tip = reactive({ visible: false, x: 0, y: 0, def: '', niveaux: [] as Niveau[] })

function ouvrirTip(e: MouseEvent, b: Badge) {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
  const rawX = r.left + r.width / 2
  tip.x = Math.max(158, Math.min(window.innerWidth - 158, rawX))
  tip.y = r.top - 8
  tip.def = b.tip.def
  tip.niveaux = b.tip.niveaux
  tip.visible = true
}
function fermerTip() { tip.visible = false }

const badges = computed<Badge[]>(() => {
  if (!props.stats) return []
  const s = props.stats
  const px = props.dernierPrix ?? s.high
  const midPx = (s.high + s.low) / 2 || 1
  const rangePct = (s.range / midPx) * 100
  const ecartVwap = s.vwap > 0 ? (px - s.vwap) / s.vwap * 100 : 0
  const distHigh = s.high > 0 ? (s.high - px) / s.high * 100 : 99
  const distLow = s.low > 0 ? (px - s.low) / s.low * 100 : 99
  const ev = ecartVwap >= 0 ? `+${ecartVwap.toFixed(2)}%` : `${ecartVwap.toFixed(2)}%`

  return [
    {
      label: 'Bougies', valeur: String(s.count), color: 'text-white',
      tip: {
        def: 'Nombre de bougies chargées. Plus il y en a, plus l\'analyse statistique est fiable.',
        niveaux: [
          { label: '< 30 · Peu de données, analyse peu fiable', color: '#ef4444', actif: s.count < 30 },
          { label: '30 – 100 · Données correctes', color: '#f59e0b', actif: s.count >= 30 && s.count < 100 },
          { label: '100 – 300 · Bonne analyse disponible', color: '#10b981', actif: s.count >= 100 && s.count < 300 },
          { label: '≥ 300 · Analyse très solide', color: '#3b82f6', actif: s.count >= 300 },
        ],
      },
    },
    {
      label: 'Vol. moy', valeur: formatVolume(s.volumeMoy), color: 'text-white',
      tip: {
        def: 'Volume moyen échangé par bougie sur la période. Référence pour détecter l\'activité anormale (Vol. rel.).',
        niveaux: [
          { label: 'Vol. rel. < 0.5× · Marché désintéressé — éviter les entrées', color: '#64748b', actif: false },
          { label: 'Vol. rel. 0.8 – 1.2× · Activité standard', color: '#10b981', actif: false },
          { label: 'Vol. rel. ≥ 2× · Activité institutionnelle probable', color: '#f59e0b', actif: false },
        ],
      },
    },
    {
      label: 'Plus haut', valeur: formatPrix(s.high), color: 'text-emerald-400',
      tip: {
        def: `Résistance maximale de la période · Zone de liquidité BSL. Distance du prix actuel au plus haut : ${distHigh.toFixed(2)}%.`,
        niveaux: [
          { label: '< 0.3% · Très proche — cassure ou rejet imminent', color: '#f59e0b', actif: distHigh < 0.3 },
          { label: '0.3 – 1% · Zone d\'attention — surveiller la réaction', color: '#10b981', actif: distHigh >= 0.3 && distHigh < 1 },
          { label: '> 1% · Éloigné — marge de progression libre', color: '#94a3b8', actif: distHigh >= 1 },
        ],
      },
    },
    {
      label: 'Plus bas', valeur: formatPrix(s.low), color: 'text-red-400',
      tip: {
        def: `Support minimal de la période · Zone de liquidité SSL. Distance du plus bas au prix actuel : ${distLow.toFixed(2)}%.`,
        niveaux: [
          { label: '< 0.3% · Très proche — rebond ou rupture imminent', color: '#f59e0b', actif: distLow < 0.3 },
          { label: '0.3 – 1% · Zone d\'attention — surveiller la réaction', color: '#10b981', actif: distLow >= 0.3 && distLow < 1 },
          { label: '> 1% · Éloigné — risque de cassure peu préoccupant', color: '#94a3b8', actif: distLow >= 1 },
        ],
      },
    },
    {
      label: 'Range', valeur: formatPrix(s.range), color: 'text-white',
      tip: {
        def: `Amplitude High − Low de la période en % du prix médian · Actuellement ${rangePct.toFixed(2)}% du prix médian.`,
        niveaux: [
          { label: '< 0.5% · Serré — marché indécis, faible liquidité', color: '#64748b', actif: rangePct < 0.5 },
          { label: '0.5 – 2% · Normal — conditions standards', color: '#10b981', actif: rangePct >= 0.5 && rangePct < 2 },
          { label: '2 – 4% · Large — volatilité élevée, opportunités', color: '#f59e0b', actif: rangePct >= 2 && rangePct < 4 },
          { label: '> 4% · Exceptionnel — mouvement extrême, risque fort', color: '#ef4444', actif: rangePct >= 4 },
        ],
      },
    },
    {
      label: 'Position',
      valeur: `${s.positionRange.toFixed(0)}% ${s.positionRange >= 50 ? 'Prem.' : 'Disc.'}`,
      color: s.positionRange >= 50 ? 'text-emerald-400' : 'text-red-400',
      tip: {
        def: 'Position du prix dans le range (0% = plus bas, 100% = plus haut). 50% = équilibre institutionnel.',
        niveaux: [
          { label: '0 – 25% · Discount profond — institutions en accumulation (BUY)', color: '#10b981', actif: s.positionRange < 25 },
          { label: '25 – 50% · Discount — biais baissier, chercher BUY sur POI', color: '#f59e0b', actif: s.positionRange >= 25 && s.positionRange < 50 },
          { label: '50 – 75% · Premium — biais haussier, chercher SELL sur POI', color: '#f59e0b', actif: s.positionRange >= 50 && s.positionRange < 75 },
          { label: '75 – 100% · Premium élevé — institutions en distribution (SELL)', color: '#ef4444', actif: s.positionRange >= 75 },
        ],
      },
    },
    {
      label: 'Vol. rel.',
      valeur: `×${s.volRelatif.toFixed(1)}`,
      color: s.volRelatif >= 2 ? 'text-amber-400' : s.volRelatif >= 1.2 ? 'text-emerald-400' : 'text-slate-300',
      tip: {
        def: 'Volume de la dernière bougie rapporté au volume moyen. ×1 = activité normale.',
        niveaux: [
          { label: '< 0.5× · Très faible — désintérêt total, éviter', color: '#64748b', actif: s.volRelatif < 0.5 },
          { label: '0.5 – 0.8× · Faible — momentum absent', color: '#94a3b8', actif: s.volRelatif >= 0.5 && s.volRelatif < 0.8 },
          { label: '0.8 – 1.2× · Normal — activité standard', color: '#10b981', actif: s.volRelatif >= 0.8 && s.volRelatif < 1.2 },
          { label: '1.2 – 2× · Élevé — signal renforcé, momentum actif', color: '#10b981', actif: s.volRelatif >= 1.2 && s.volRelatif < 2 },
          { label: '≥ 2× · Institutionnel — activité anormale, très fort intérêt', color: '#f59e0b', actif: s.volRelatif >= 2 },
        ],
      },
    },
    {
      label: 'VWAP', valeur: formatPrix(s.vwap), color: 'text-blue-400',
      tip: {
        def: `Prix moyen pondéré par le volume. Référence institutionnelle du "juste prix". Écart actuel : ${ev}.`,
        niveaux: [
          { label: '< −1% sous VWAP · Structure baissière confirmée', color: '#ef4444', actif: ecartVwap < -1 },
          { label: '−1 à 0% · Légèrement en discount — faiblesse relative', color: '#f59e0b', actif: ecartVwap >= -1 && ecartVwap < 0 },
          { label: '0 à +1% · Légèrement en premium — force relative', color: '#10b981', actif: ecartVwap >= 0 && ecartVwap < 1 },
          { label: '> +1% sur VWAP · Structure haussière confirmée', color: '#3b82f6', actif: ecartVwap >= 1 },
        ],
      },
    },
  ]
})

function formatPrix(v: number): string {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD',
    minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(v)
}
function formatVolume(v: number): string {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(2)}M`
  if (v >= 1_000) return `${(v / 1_000).toFixed(1)}K`
  return v.toFixed(2)
}
</script>

<style>
.cpx-badge {
  display: flex; flex-direction: column; align-items: center;
  padding: 8px 14px; border-radius: 12px; cursor: default; min-width: 70px;
  border: 1px solid rgba(255,255,255,0.1);
  background: rgba(255,255,255,0.05);
  backdrop-filter: blur(4px);
  transition: border-color 0.15s;
}
.cpx-badge:hover { border-color: rgba(168,85,247,0.45); }
.cpx-tooltip {
  position: fixed;
  transform: translate(-50%, -100%);
  width: 300px;
  background: rgba(10,14,39,0.97);
  border: 1px solid rgba(168,85,247,0.4);
  border-radius: 10px;
  padding: 12px 14px;
  z-index: 9999;
  pointer-events: none;
  box-shadow: 0 8px 32px rgba(0,0,0,0.65);
}
.cpx-def {
  font-size: 0.71rem; color: #e2e8f0;
  line-height: 1.55; margin: 0 0 9px;
}
.cpx-sep { height: 1px; background: rgba(255,255,255,0.1); margin-bottom: 8px; }
.cpx-echelle { display: flex; flex-direction: column; gap: 3px; }
.cpx-niveau {
  display: flex; align-items: center; gap: 6px;
  font-size: 0.67rem; color: #64748b;
  padding: 3px 7px; border-radius: 5px;
  border: 1px solid transparent;
}
.cpx-actif { font-weight: 600; }
.cpx-bullet { font-size: 0.58rem; width: 10px; flex-shrink: 0; }
.cpx-tip-enter-active, .cpx-tip-leave-active { transition: opacity 0.1s, transform 0.1s; }
.cpx-tip-enter-from, .cpx-tip-leave-to { opacity: 0; transform: translate(-50%, calc(-100% + 6px)); }
.cpx-tip-enter-to, .cpx-tip-leave-from { opacity: 1; transform: translate(-50%, -100%); }
</style>
