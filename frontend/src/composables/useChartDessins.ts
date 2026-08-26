/**
 * useChartDessins — outils de dessin interactifs sur le graphique.
 *
 * Couche canvas dédiée (même pattern que l'overlay SMC v12) au-dessus de
 * tout le reste (z-index 5) : ligne, rectangle et retracement Fibonacci
 * dessinés au clic-glissé, gomme par clic, persistance par asset en
 * localStorage (clé `trading_dessins`) — les dessins survivent aux
 * rechargements et sont visibles sur tous les timeframes de l'asset.
 *
 * Le canvas est transparent aux événements (pointer-events: none) tant
 * qu'aucun outil n'est actif : le chart garde son pan/zoom normal. Un
 * outil actif capture le pointeur (crosshair) et bloque le pan le temps
 * du tracé. Échap ou re-clic sur l'outil = désactivation.
 */
import type { IChartApi, ISeriesApi } from 'lightweight-charts'
import { ref } from 'vue'

export type OutilDessin = 'aucun' | 'ligne' | 'rectangle' | 'fibo' | 'gomme'
type TypeDessin = 'ligne' | 'rectangle' | 'fibo'

interface Dessin {
  id: number
  type: TypeDessin
  /** Ancrages : (t1,p1) = 0 % du fib / premier sommet, (t2,p2) = 100 % / second. */
  t1: number; p1: number
  t2: number; p2: number
}

const CLE_STOCKAGE = 'trading_dessins'
const COULEUR = '#3b82f6' // blue-500
const COULEUR_FIB = '#f59e0b' // amber-500
const NIVEAUX_FIB = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1]

function lireStock(): Record<string, Dessin[]> {
  try { return JSON.parse(localStorage.getItem(CLE_STOCKAGE) ?? '{}') } catch { return {} }
}
function ecrireStock(d: Record<string, Dessin[]>) {
  localStorage.setItem(CLE_STOCKAGE, JSON.stringify(d))
}

export function useChartDessins() {
  const outil = ref<OutilDessin>('aucun')

  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let serieRef: ISeriesApi<'Candlestick'> | null = null
  let containerRef: HTMLElement | null = null
  let ro: ResizeObserver | null = null
  let desabonnements: (() => void)[] = []
  let animFrame: number | null = null

  let assetCourant = ''
  let dessins: Dessin[] = []
  let prochainId = Date.now()
  /** Tracé en cours (aperçu) — null si aucun. */
  let enCours: Dessin | null = null
  let pointeurActif = false
  /** Index du dessin survolé en mode gomme (surlignage avant suppression). */
  let survolGomme: number | null = null

  // ── Persistances ────────────────────────────────────────────────────────────
  function recharger() {
    dessins = lireStock()[assetCourant] ?? []
    prochainId = Math.max(Date.now(), ...dessins.map(d => d.id)) + 1
  }

  function persister() {
    const stock = lireStock()
    stock[assetCourant] = dessins
    ecrireStock(stock)
  }

  function definirAsset(asset: string) {
    if (asset === assetCourant) return
    assetCourant = asset
    recharger()
    planifierRedessin()
  }

  // ── Canvas ──────────────────────────────────────────────────────────────────
  function monterCanvas(container: HTMLElement): HTMLCanvasElement {
    containerRef = container
    const c = document.createElement('canvas')
    c.style.position = 'absolute'
    c.style.top = '0'
    c.style.left = '0'
    c.style.width = '100%'
    c.style.height = '100%'
    c.style.pointerEvents = 'none'
    c.style.zIndex = '5'
    container.appendChild(c)
    return c
  }

  function redimensionner() {
    if (!canvas || !containerRef) return
    const ratio = window.devicePixelRatio || 1
    const w = containerRef.offsetWidth
    const h = containerRef.offsetHeight
    if (w === 0 || h === 0) return
    canvas.width = w * ratio
    canvas.height = h * ratio
    const ctx = canvas.getContext('2d')
    if (ctx) ctx.scale(ratio, ratio)
  }

  function majPointerEvents() {
    if (!canvas) return
    const actif = outil.value !== 'aucun'
    canvas.style.pointerEvents = actif ? 'auto' : 'none'
    canvas.style.cursor = actif ? 'crosshair' : 'default'
  }

  // ── Conversions pixel ↔ données ─────────────────────────────────────────────
  function versTemps(x: number): number | null {
    if (!chartRef) return null
    const t = chartRef.timeScale().coordinateToTime(x)
    if (t !== null) return Number(t)
    // Entre deux bougies / bord droit : repli sur la borne visible droite.
    const plage = chartRef.timeScale().getVisibleRange()
    return plage ? Number(plage.to) : null
  }

  function versX(t: number): number | null {
    if (!chartRef) return null
    const x = chartRef.timeScale().timeToCoordinate(t as never)
    return x ?? null
  }

  function versY(p: number): number | null {
    if (!serieRef) return null
    return serieRef.priceToCoordinate(p) ?? null
  }

  // ── Rendu ───────────────────────────────────────────────────────────────────
  function dessinerDessin(ctx: CanvasRenderingContext2D, d: Dessin, apercu: boolean, W: number, surligne = false) {
    const x1 = versX(d.t1); const x2 = versX(d.t2)
    const y1 = versY(d.p1); const y2 = versY(d.p2)
    if (x1 === null || x2 === null || y1 === null || y2 === null) return
    ctx.save()
    ctx.globalAlpha = apercu ? 0.6 : 1
    ctx.lineWidth = 1.5
    if (surligne) {
      // Cible de la gomme : contour rouge + halo.
      ctx.shadowColor = '#ef4444'
      ctx.shadowBlur = 6
      ctx.strokeStyle = '#ef4444'
      ctx.fillStyle = 'rgba(239,68,68,0.10)'
    }

    if (d.type === 'ligne') {
      ctx.strokeStyle = COULEUR
      ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke()
    } else if (d.type === 'rectangle') {
      ctx.strokeStyle = COULEUR
      ctx.fillStyle = 'rgba(59,130,246,0.08)'
      const gx = Math.min(x1, x2), gy = Math.min(y1, y2)
      const gw = Math.abs(x2 - x1), gh = Math.abs(y2 - y1)
      ctx.fillRect(gx, gy, gw, gh)
      ctx.strokeRect(gx, gy, gw, gh)
    } else {
      // Fibonacci : p1 = 0 %, p2 = 100 % — lignes de l'ancre gauche au bord
      // droit, zone dorée (0.5 → 0.786) ombrée, libellés à droite.
      const xa = Math.min(x1, x2)
      ctx.fillStyle = 'rgba(245,158,11,0.07)'
      const y05 = versY(d.p1 + (d.p2 - d.p1) * 0.5)
      const y0786 = versY(d.p1 + (d.p2 - d.p1) * 0.786)
      if (y05 !== null && y0786 !== null) {
        ctx.fillRect(xa, Math.min(y05, y0786), W - xa, Math.abs(y0786 - y05))
      }
      ctx.strokeStyle = COULEUR_FIB
      ctx.font = '10px sans-serif'
      ctx.fillStyle = COULEUR_FIB
      for (const n of NIVEAUX_FIB) {
        const y = versY(d.p1 + (d.p2 - d.p1) * n)
        if (y === null) continue
        ctx.beginPath(); ctx.moveTo(xa, y); ctx.lineTo(W, y); ctx.stroke()
        const label = n === 0 ? '0' : n === 1 ? '1' : `${n}`
        ctx.fillText(label, W - 30, y - 3)
      }
      // Diagonale d'ancrage 0→1 (repère visuel du tracé).
      ctx.globalAlpha *= 0.5
      ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke()
    }
    ctx.restore()
  }

  function redessiner() {
    if (!canvas || !chartRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const W = canvas.offsetWidth
    ctx.clearRect(0, 0, W, canvas.offsetHeight)
    dessins.forEach((d, i) => dessinerDessin(ctx, d, false, W, outil.value === 'gomme' && survolGomme === i))
    if (enCours) dessinerDessin(ctx, enCours, true, W)
  }

  function planifierRedessin() {
    if (animFrame !== null) cancelAnimationFrame(animFrame)
    animFrame = requestAnimationFrame(() => { animFrame = null; redessiner() })
  }

  // ── Interactions ────────────────────────────────────────────────────────────
  function pointeurVersAncres(e: PointerEvent): { t: number; p: number } | null {
    if (!canvas) return null
    const rect = canvas.getBoundingClientRect()
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    const t = versTemps(x)
    const p = serieRef?.coordinateToPrice(y) ?? null
    if (t === null || p === null) return null
    return { t, p }
  }

  /** Distance d'un point aux segments constituant un dessin (hit-test gomme). */
  function distanceAuDessin(x: number, y: number, d: Dessin): number {
    const x1 = versX(d.t1); const x2 = versX(d.t2)
    const y1 = versY(d.p1); const y2 = versY(d.p2)
    if (x1 === null || x2 === null || y1 === null || y2 === null) return Infinity
    if (d.type === 'ligne') {
      return distanceSegment(x, y, x1, y1, x2, y2)
    }
    if (d.type === 'rectangle') {
      const gx = Math.min(x1, x2), gy = Math.min(y1, y2)
      const gw = Math.abs(x2 - x1), gh = Math.abs(y2 - y1)
      const bords = [
        distanceSegment(x, y, gx, gy, gx + gw, gy),
        distanceSegment(x, y, gx, gy + gh, gx + gw, gy + gh),
        distanceSegment(x, y, gx, gy, gx, gy + gh),
        distanceSegment(x, y, gx + gw, gy, gx + gw, gy + gh),
      ]
      return Math.min(...bords)
    }
    // Fib : distance aux lignes de niveaux.
    let min = distanceSegment(x, y, x1, y1, x2, y2)
    const xa = Math.min(x1, x2)
    for (const n of NIVEAUX_FIB) {
      const y = versY(d.p1 + (d.p2 - d.p1) * n)
      if (y !== null) min = Math.min(min, distanceSegment(x, y, xa, y, xa + 4000, y))
    }
    return min
  }

  function distanceSegment(px: number, py: number, x1: number, y1: number, x2: number, y2: number): number {
    const dx = x2 - x1, dy = y2 - y1
    const long2 = dx * dx + dy * dy
    if (long2 === 0) return Math.hypot(px - x1, py - y1)
    let t = ((px - x1) * dx + (py - y1) * dy) / long2
    t = Math.max(0, Math.min(1, t))
    return Math.hypot(px - (x1 + t * dx), py - (y1 + t * dy))
  }

  /** Dessin le plus proche du pointeur sous 8 px (cible gomme), sinon null. */
  function dessinSousPointeur(e: PointerEvent): number | null {
    if (!canvas) return null
    const rect = canvas.getBoundingClientRect()
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    let meilleur: number | null = null
    let meilleureDist = 8
    dessins.forEach((d, i) => {
      const dist = distanceAuDessin(x, y, d)
      if (dist < meilleureDist) { meilleureDist = dist; meilleur = i }
    })
    return meilleur
  }

  function onPointerDown(e: PointerEvent) {
    if (outil.value === 'aucun') return
    const a = pointeurVersAncres(e)
    if (!a) return
    if (outil.value === 'gomme') {
      const cible = dessinSousPointeur(e)
      if (cible !== null && cible < dessins.length) {
        dessins.splice(cible, 1)
        persister()
        survolGomme = null
        planifierRedessin()
      }
      return
    }
    pointeurActif = true
    canvas?.setPointerCapture(e.pointerId)
    enCours = { id: prochainId, type: outil.value as TypeDessin, t1: a.t, p1: a.p, t2: a.t, p2: a.p }
    planifierRedessin()
  }

  function onPointerMove(e: PointerEvent) {
    if (outil.value === 'gomme' && !pointeurActif) {
      const cible = dessinSousPointeur(e)
      if (cible !== survolGomme) { survolGomme = cible; planifierRedessin() }
      return
    }
    if (!pointeurActif || !enCours) return
    const a = pointeurVersAncres(e)
    if (!a) return
    enCours.t2 = a.t
    enCours.p2 = a.p
    planifierRedessin()
  }

  function onPointerUp() {
    if (!pointeurActif || !enCours) return
    pointeurActif = false
    const d = enCours
    enCours = null
    // Clic sans traîné (ancres quasi identiques) → aucun dessin.
    const tailleOk = Math.abs(d.p2 - d.p1) > 1e-10 || d.t1 !== d.t2
    if (tailleOk) {
      dessins.push(d)
      persister()
    }
    // Le tracé fini rend la main : l'outil se désactive pour ne pas
    // capturer le pointeur (pan/zoom du chart) indéfiniment.
    outil.value = 'aucun'
    majPointerEvents()
    planifierRedessin()
  }

  function choisirOutil(o: OutilDessin) {
    outil.value = outil.value === o ? 'aucun' : o
    enCours = null
    survolGomme = null
    majPointerEvents()
    planifierRedessin()
  }

  function toutEffacer() {
    dessins = []
    persister()
    planifierRedessin()
  }

  function surEscape(e: KeyboardEvent) {
    if (e.key !== 'Escape') return
    enCours = null
    outil.value = 'aucun'
    majPointerEvents()
    planifierRedessin()
  }

  // ── Cycle de vie ────────────────────────────────────────────────────────────
  function initialiser(chart: IChartApi, serie: ISeriesApi<'Candlestick'>, container: HTMLElement, asset: string) {
    detruire()
    chartRef = chart
    serieRef = serie
    assetCourant = asset
    recharger()
    canvas = monterCanvas(container)
    redimensionner()

    canvas.addEventListener('pointerdown', onPointerDown)
    canvas.addEventListener('pointermove', onPointerMove)
    canvas.addEventListener('pointerup', onPointerUp)
    window.addEventListener('keydown', surEscape)

    const handler = () => planifierRedessin()
    chart.timeScale().subscribeVisibleTimeRangeChange(handler)
    chart.timeScale().subscribeVisibleLogicalRangeChange(handler)
    desabonnements.push(() => {
      try {
        chart.timeScale().unsubscribeVisibleTimeRangeChange(handler)
        chart.timeScale().unsubscribeVisibleLogicalRangeChange(handler)
      } catch { /* chart détruit */ }
    })

    ro = new ResizeObserver(() => { redimensionner(); planifierRedessin() })
    ro.observe(container)
    majPointerEvents()
    planifierRedessin()
  }

  function detruire() {
    if (animFrame !== null) { cancelAnimationFrame(animFrame); animFrame = null }
    if (canvas) {
      canvas.removeEventListener('pointerdown', onPointerDown)
      canvas.removeEventListener('pointermove', onPointerMove)
      canvas.removeEventListener('pointerup', onPointerUp)
      canvas.parentElement?.removeChild(canvas)
      canvas = null
    }
    window.removeEventListener('keydown', surEscape)
    desabonnements.forEach(d => d())
    desabonnements = []
    ro?.disconnect()
    ro = null
    chartRef = null
    serieRef = null
    containerRef = null
    enCours = null
  }

  return { outil, initialiser, detruire, definirAsset, choisirOutil, toutEffacer }
}
