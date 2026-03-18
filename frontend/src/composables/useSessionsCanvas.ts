import type { IChartApi } from 'lightweight-charts'
import type { PrefsIndicateurs } from '@/stores/settings.store'
import { hexVersRgba } from './chartIndicatorsConfig'

/** Définition d'une session boursière (horaires locaux via IANA timezone) */
interface SessionDef {
  key: keyof PrefsIndicateurs   // ex: 'sessionsSydney'
  nom: string
  timezone: string              // IANA : 'Australia/Sydney'
  ouvertureH: number            // heure locale d'ouverture
  ouvertureM: number
  fermetureH: number            // heure locale de fermeture
  fermetureM: number
  couleurKey: keyof PrefsIndicateurs  // ex: 'sessionsCouleurSydney'
}

export const SESSIONS_DEF: SessionDef[] = [
  { key: 'sessionsSydney',   nom: 'Sydney',    timezone: 'Australia/Sydney',  ouvertureH: 10, ouvertureM: 0, fermetureH: 16, fermetureM: 0, couleurKey: 'sessionsCouleurSydney'   },
  { key: 'sessionsTokyo',    nom: 'Tokyo',     timezone: 'Asia/Tokyo',        ouvertureH: 9,  ouvertureM: 0, fermetureH: 18, fermetureM: 0, couleurKey: 'sessionsCouleurTokyo'    },
  { key: 'sessionsHongKong', nom: 'Hong Kong', timezone: 'Asia/Hong_Kong',    ouvertureH: 9,  ouvertureM: 30,fermetureH: 16, fermetureM: 0, couleurKey: 'sessionsCouleurHongKong' },
  { key: 'sessionsLondres',  nom: 'Londres',   timezone: 'Europe/London',     ouvertureH: 8,  ouvertureM: 0, fermetureH: 16, fermetureM: 30,couleurKey: 'sessionsCouleurLondres'  },
  { key: 'sessionsNewYork',  nom: 'New York',  timezone: 'America/New_York',  ouvertureH: 9,  ouvertureM: 30,fermetureH: 16, fermetureM: 0, couleurKey: 'sessionsCouleurNewYork'  },
]

/**
 * Convertit une heure locale (ex: 9h30 America/New_York) d'un jour donné
 * en timestamp Unix secondes UTC.
 */
function localVersUtcSec(dateUtcJour: Date, tz: string, h: number, m: number): number {
  // On cherche par dichotomie le timestamp UTC tel que
  // Intl.DateTimeFormat(tz).format(ts) = h:m pour ce jour calendaire.
  // Approche simple : construire une date "approximative" puis corriger avec le vrai offset.
  const annee  = new Intl.DateTimeFormat('en-CA', { timeZone: tz, year:  'numeric' }).format(dateUtcJour)
  const mois   = new Intl.DateTimeFormat('en-CA', { timeZone: tz, month: '2-digit' }).format(dateUtcJour)
  const jour   = new Intl.DateTimeFormat('en-CA', { timeZone: tz, day:   '2-digit' }).format(dateUtcJour)
  // ISO local approximatif (sans offset) → on le passe à Date en UTC pour obtenir ~ le bon jour
  const isoLocal = `${annee}-${mois}-${jour}T${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}:00`
  // Date.UTC ne connaît pas les offsets — on utilise l'astuce Intl
  // Construire la date en UTC "naïf", puis corriger par la différence d'offset
  const naif = new Date(`${isoLocal}Z`) // UTC "naïf" sans offset
  // Calculer l'offset réel du TZ à ce moment approximatif
  const partsTz  = new Intl.DateTimeFormat('en-CA', {
    timeZone: tz, year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false,
  }).formatToParts(naif)
  const get = (type: string) => parseInt(partsTz.find(p => p.type === type)?.value ?? '0')
  const localTs = Date.UTC(get('year'), get('month') - 1, get('day'), get('hour'), get('minute'), get('second'))
  const offsetMs = naif.getTime() - localTs
  return Math.floor((naif.getTime() + offsetMs) / 1000)
}

/**
 * Génère les intervalles [tsDebut, tsFin] UTC secondes pour une session
 * sur une fenêtre de jours couvrant [depuis, jusqu'a] (Unix secondes).
 */
function genererIntervallesSession(
  s: SessionDef,
  depuis: number,
  jusqua: number,
): Array<{ debut: number; fin: number }> {
  const intervalles: Array<{ debut: number; fin: number }> = []
  // Itérer sur les jours de la fenêtre visible + marge de 2 jours de chaque côté
  const debutMs = (depuis - 2 * 86400) * 1000
  const finMs   = (jusqua + 2 * 86400) * 1000
  let curseur = new Date(debutMs)
  curseur.setUTCHours(0, 0, 0, 0)

  while (curseur.getTime() <= finMs) {
    const debut = localVersUtcSec(curseur, s.timezone, s.ouvertureH, s.ouvertureM)
    let fin     = localVersUtcSec(curseur, s.timezone, s.fermetureH, s.fermetureM)
    // Si fermeture < ouverture (cross-midnight), fin = lendemain
    if (fin <= debut) fin += 86400
    intervalles.push({ debut, fin })
    curseur = new Date(curseur.getTime() + 86400_000)
  }
  return intervalles
}

export function useSessionsCanvas() {
  let canvas: HTMLCanvasElement | null = null
  let chartRef: IChartApi | null = null
  let containerRef: HTMLElement | null = null
  let prefsRef: PrefsIndicateurs | null = null
  let unsubscribe: (() => void) | null = null
  let animFrame: number | null = null

  function monterCanvas(container: HTMLElement): HTMLCanvasElement {
    containerRef = container
    const c = document.createElement('canvas')
    c.style.position = 'absolute'
    c.style.top      = '0'
    c.style.left     = '0'
    c.style.width    = '100%'
    c.style.height   = '100%'
    c.style.pointerEvents = 'none'
    c.style.zIndex   = '1'  // sous les zones SMC (zIndex 2)
    container.appendChild(c)
    return c
  }

  function redimensionner() {
    if (!canvas || !containerRef) return
    const ratio = window.devicePixelRatio || 1
    const w = containerRef.offsetWidth
    const h = containerRef.offsetHeight
    if (w === 0 || h === 0) return
    canvas.width  = w * ratio
    canvas.height = h * ratio
    const ctx = canvas.getContext('2d')
    if (ctx) ctx.scale(ratio, ratio)
  }

  function planifierRedessiner() {
    if (animFrame !== null) return
    animFrame = requestAnimationFrame(() => {
      animFrame = null
      redessiner()
    })
  }

  function redessiner() {
    if (!canvas || !chartRef || !prefsRef) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const W = canvas.offsetWidth
    const H = canvas.offsetHeight
    ctx.clearRect(0, 0, W, H)

    if (!prefsRef.sessionsActif) return

    const timeScale = chartRef.timeScale()
    const range = timeScale.getVisibleRange()
    if (!range) return

    const depuis  = range.from as number
    const jusqua  = range.to   as number
    const opacite = prefsRef.sessionsOpacite

    for (const s of SESSIONS_DEF) {
      if (!prefsRef[s.key]) continue
      const hex = prefsRef[s.couleurKey] as string
      const couleur = hexVersRgba(hex, opacite)
      const intervalles = genererIntervallesSession(s, depuis, jusqua)

      for (const { debut, fin } of intervalles) {
        if (fin < depuis || debut > jusqua) continue
        const x1Raw = timeScale.timeToCoordinate(debut as any)
        const x2Raw = timeScale.timeToCoordinate(fin   as any)
        const x1 = x1Raw !== null ? x1Raw : 0
        const x2 = x2Raw !== null ? x2Raw : W
        if (x2 <= x1) continue

        ctx.fillStyle = couleur
        ctx.fillRect(x1, 0, x2 - x1, H)

        // Label session en haut de la bande (si assez large)
        if (prefsRef.sessionsLabels && x2 - x1 > 30) {
          ctx.font      = '10px Inter, sans-serif'
          ctx.fillStyle = hexVersRgba(hex, Math.min(opacite * 4, 0.9))
          ctx.fillText(s.nom, x1 + 4, 14)
        }
      }
    }
  }

  function initialiser(chart: IChartApi, container: HTMLElement, prefs: PrefsIndicateurs) {
    if (canvas) detruire()
    chartRef      = chart
    containerRef  = container
    prefsRef      = prefs
    canvas        = monterCanvas(container)
    redimensionner()

    const ro = new ResizeObserver(() => { redimensionner(); planifierRedessiner() })
    ro.observe(container)
    ;(canvas as any).__ro = ro

    unsubscribe = () => chart.timeScale().unsubscribeVisibleTimeRangeChange(planifierRedessiner)
    chart.timeScale().subscribeVisibleTimeRangeChange(planifierRedessiner)
    planifierRedessiner()
  }

  function mettreAJourPrefs(prefs: PrefsIndicateurs) {
    prefsRef = prefs
    planifierRedessiner()
  }

  function detruire() {
    if (animFrame !== null) { cancelAnimationFrame(animFrame); animFrame = null }
    unsubscribe?.()
    unsubscribe = null
    if (canvas) {
      ;(canvas as any).__ro?.disconnect()
      canvas.parentElement?.removeChild(canvas)
      canvas = null
    }
    chartRef     = null
    containerRef = null
    prefsRef     = null
  }

  return { initialiser, mettreAJourPrefs, detruire }
}
