<template>
  <!-- Mini-graphique du Scanner SMC (11/09, v3) : chandeliers temps réel
       (WS par couple — maj SÉRIE.update à chaque tick, prix affiché au
       niveau du prix par lightweight-charts comme le graphique principal)
       + OB rendus À L'IDENTIQUE de l'indicateur principal (filtre force
       ≥ 5, couleurs/alphas/bordures/label copiés de dessinerObsEtFvgs :
       début = bougie d'origine, fin = bougie en cours) + niveaux des
       setups vivants en pointillé. -->
  <div ref="conteneur" class="relative w-full h-[230px] cursor-pointer" @click="$emit('ouvrir')" />
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import {
  createChart,
  type IChartApi,
  type ISeriesApi,
  type Time,
} from 'lightweight-charts'
import { useMarketStore } from '@/stores/market.store'
import { apiService } from '@/services/api.service'
import type { ObV12 } from '@/services/api.smc'

const props = defineProps<{
  asset: string
  tf: string
  /** Niveaux des setups vivants à dessiner en pointillé. */
  setups: { direction: string; entree: number; sl: number; tp1: number }[]
}>()
defineEmits<{ (e: 'ouvrir'): void }>()

const conteneur = ref<HTMLElement | null>(null)
const marketStore = useMarketStore()

let chart: IChartApi | null = null
let serie: ISeriesApi<'Candlestick'> | null = null
let canvas: HTMLCanvasElement | null = null
let obs: ObV12[] = []
let derniereLongueur = 0

// ── Rendu OB : copie exacte du graphique principal (dessinerObsEtFvgs) ──
const COUL_OB_BULL = '#00C853'
const COUL_OB_BEAR = '#D50000'
const OB_ALPHA: Record<string, number> = {
  vierge: (100 - 70) / 100,
  partiel: (100 - 83) / 100,
  profond: (100 - 91) / 100,
}
const OB_BORD_ALPHA = (100 - 20) / 100

function hexVersRgba(hex: string, alpha: number): string {
  const n = parseInt(hex.slice(1), 16)
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha})`
}

const bougies = () => marketStore.getBougies(props.asset, props.tf)
/// Signature de la dernière bougie : change à CHAQUE tick WS (close vivant)
/// comme à chaque nouvelle bougie (longueur) — le cœur du temps réel.
const signature = () => {
  const b = bougies()
  return b.length ? `${b.length}:${b[b.length - 1].close}` : ''
}

function monter() {
  if (!conteneur.value) return
  chart = createChart(conteneur.value, {
    height: 230,
    autoSize: true,
    layout: { background: { color: 'transparent' }, textColor: '#8b949e', fontSize: 9 },
    grid: {
      vertLines: { visible: false },
      horzLines: { color: 'rgba(255,255,255,0.04)' },
    },
    rightPriceScale: { borderVisible: false, scaleMargins: { top: 0.08, bottom: 0.08 } },
    timeScale: { visible: false, borderVisible: false },
    crosshair: { mode: 0 },
    handleScale: false,
    handleScroll: false,
  })
  serie = chart.addCandlestickSeries({
    upColor: '#26a69a', downColor: '#ef5350', wickUpColor: '#26a69a', wickDownColor: '#ef5350',
    borderVisible: false,
    // Prix au niveau du prix + ligne pointillée, mis à jour à chaque tick —
    // exactement comme le graphique principal.
    priceLineVisible: true,
    lastValueVisible: true,
    priceLineColor: '#e6edf3',
    priceLineStyle: 2,
  })
  canvas = document.createElement('canvas')
  canvas.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;pointer-events:none;z-index:4'
  conteneur.value.appendChild(canvas)
  void marketStore.chargerBougies(props.asset, props.tf, 180)
  marketStore.abonner(props.asset, props.tf)
  void chargerAnalyse()
}

async function chargerAnalyse() {
  try {
    const a = await apiService.getSmcV12Analyse(props.asset, props.tf, 200)
    obs = a.obs ?? []
    dessiner()
  } catch { /* analyse indisponible — chandeliers seuls */ }
}

function majSeries() {
  if (!serie) return
  const b = bougies()
  if (!b.length) return
  const derniere = b[b.length - 1]
  const point = {
    time: (new Date(derniere.timestamp).getTime() / 1000) as unknown as Time,
    open: derniere.open, high: derniere.high, low: derniere.low, close: derniere.close,
  }
  if (b.length !== derniereLongueur) {
    serie.setData(b.map(c => ({
      time: (new Date(c.timestamp).getTime() / 1000) as unknown as Time,
      open: c.open, high: c.high, low: c.low, close: c.close,
    })))
    chart?.timeScale().scrollToRealTime()
    derniereLongueur = b.length
  } else {
    serie.update(point)
  }
  dessiner()
}

function dessiner() {
  if (!canvas || !chart || !serie || !conteneur.value) return
  const ratio = window.devicePixelRatio || 1
  const W = conteneur.value.clientWidth
  const H = conteneur.value.clientHeight
  if (!W || !H) return
  canvas.width = W * ratio
  canvas.height = H * ratio
  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.scale(ratio, ratio)
  ctx.clearRect(0, 0, W, H)

  const ts = chart.timeScale()
  const b = bougies()
  const dernierTs = b.length ? (new Date(b[b.length - 1].timestamp).getTime() / 1000) : null
  const xD = dernierTs !== null
    ? Math.min(ts.timeToCoordinate(dernierTs as unknown as Time) ?? W - 4, W - 4)
    : W - 4

  // ── OB — rendu identique au graphique principal : force ≥ 5 (filtre
  // d'affichage Pine), début = bougie d'origine, fin = bougie en cours. ──
  for (const o of obs) {
    if (o.force < 5) continue
    const yHaut = serie.priceToCoordinate(o.top)
    const yBas = serie.priceToCoordinate(o.bot)
    if (yHaut === null || yBas === null) continue
    const yTop = Math.min(yHaut, yBas)
    const hauteur = Math.abs(yHaut - yBas)
    if (hauteur < 1) continue
    const xGRaw = ts.timeToCoordinate(o.ts as unknown as Time)
    const xG = xGRaw !== null ? Math.max(0, xGRaw) : 0
    if (xD <= xG) continue
    const hex = o.dir === 'bull' ? COUL_OB_BULL : COUL_OB_BEAR
    const alpha = OB_ALPHA[o.state] ?? OB_ALPHA.vierge
    ctx.fillStyle = hexVersRgba(hex, alpha)
    ctx.fillRect(xG, yTop, xD - xG, hauteur)
    ctx.strokeStyle = hexVersRgba(hex, OB_BORD_ALPHA)
    ctx.lineWidth = 1.5
    ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xD, yTop); ctx.stroke()
    ctx.beginPath(); ctx.moveTo(xG, yTop + hauteur); ctx.lineTo(xD, yTop + hauteur); ctx.stroke()
    ctx.lineWidth = 2
    ctx.beginPath(); ctx.moveTo(xG, yTop); ctx.lineTo(xG, yTop + hauteur); ctx.stroke()
    ctx.font = 'bold 10px sans-serif'
    ctx.fillStyle = hexVersRgba(hex, 1)
    ctx.textAlign = 'right'
    ctx.textBaseline = 'top'
    ctx.fillText(`OB ${o.force}/10`, xD - 3, yTop + 2)
  }

  // ── Niveaux des setups vivants : entrée/SL/TP1 en pointillé ──
  const y = (p: number) => serie!.priceToCoordinate(p)
  for (const s of props.setups.slice(0, 2)) {
    const lignes: [number, string, string][] = [
      [s.entree, '#e6edf3', 'E'],
      [s.sl, 'rgba(239,83,80,0.85)', 'SL'],
      [s.tp1, 'rgba(38,166,154,0.85)', 'TP1'],
    ]
    for (const [prix, coul, lbl] of lignes) {
      const yp = y(prix)
      if (yp === null || yp < 0 || yp > H) continue
      ctx.strokeStyle = coul
      ctx.setLineDash([4, 3])
      ctx.beginPath(); ctx.moveTo(W * 0.35, yp); ctx.lineTo(xD, yp); ctx.stroke()
      ctx.setLineDash([])
      ctx.font = 'bold 8px sans-serif'
      ctx.fillStyle = coul
      ctx.textAlign = 'right'; ctx.textBaseline = 'bottom'
      ctx.fillText(lbl, xD - 2, yp - 1)
    }
  }
}

watch(signature, () => majSeries())
watch(() => props.tf, () => {
  marketStore.desabonner(props.asset, props.tf)
  obs = []
  derniereLongueur = 0
  void marketStore.chargerBougies(props.asset, props.tf, 180)
  marketStore.abonner(props.asset, props.tf)
  void chargerAnalyse()
  majSeries()
})
watch(() => props.setups.length, () => dessiner())

onMounted(monter)
onBeforeUnmount(() => {
  marketStore.desabonner(props.asset, props.tf)
  chart?.remove()
})
</script>
