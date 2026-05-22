// ─── Couleurs des overlays ────────────────────────────────────────────────────
export const COULEURS = {
  ema: '#f59e0b',
  bollingerHaute: '#6366f1',
  bollingerMilieu: '#818cf8',
  bollingerBasse: '#6366f1',
  ob_long: 'rgba(16, 185, 129, 0.25)',
  ob_short: 'rgba(239, 68, 68, 0.25)',
  fvg_long: 'rgba(59, 130, 246, 0.20)',
  fvg_short: 'rgba(245, 158, 11, 0.20)',
  ifvg_long: 'rgba(99, 102, 241, 0.25)',
  ifvg_short: 'rgba(236, 72, 153, 0.25)',
  fib: 'rgba(148, 163, 184, 0.6)',
  bsl: 'rgba(16, 185, 129, 0.8)',
  ssl: 'rgba(239, 68, 68, 0.8)',
  bos_long: '#10b981',
  bos_short: '#ef4444',
  choch: '#a855f7',
}

/** Convertit un hex #rrggbb en rgba(r,g,b,alpha) */
export function hexVersRgba(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  return `rgba(${r},${g},${b},${alpha})`
}

import { TickMarkType } from 'lightweight-charts'

function tickMarkFormatterParis(ts: number, markType: TickMarkType): string {
  const d = new Date(ts * 1000)
  if (markType <= TickMarkType.DayOfMonth) {
    return new Intl.DateTimeFormat('fr-FR', {
      timeZone: 'Europe/Paris', day: '2-digit', month: '2-digit',
    }).format(d)
  }
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: 'Europe/Paris', hour: '2-digit', minute: '2-digit',
  }).format(d)
}

import type { PrefsIndicateurs } from '@/stores/settings.store'
import type { IndicatorsParams } from '@/services/api.types'

/** Construit les paramètres de la requête /api/indicators depuis les préférences utilisateur. */
export function buildIndicatorsParams(asset: string, tf: string, prefs: PrefsIndicateurs): IndicatorsParams {
  return {
    asset, tf,
    ema: prefs.ema, ema_periode: prefs.emaPeriode, ema_ma_type: prefs.emaMaType,
    rsi: prefs.rsi, rsi_periode: prefs.rsiPeriode,
    macd: prefs.macd, macd_rapide: prefs.macdRapide, macd_lente: prefs.macdLente, macd_signal: prefs.macdSignal,
    bollinger: prefs.bollinger, bollinger_periode: prefs.bollingerPeriode,
    bollinger_stddev: prefs.bollingerStdDev, bollinger_ma_type: prefs.bollingerMaType,
    atr: prefs.atr, atr_periode: prefs.atrPeriode,
    smc_ob: prefs.smcOb, smc_ob_sensitivity: prefs.smcObSensibilite, smc_ob_mitigation: prefs.smcObMitigationType,
    smc_ifvg: prefs.smcBpr, smc_ifvg_show_last: prefs.smcIfvgShowLast,
    smc_ifvg_signal_pref: prefs.smcIfvgSignalPref, smc_ifvg_atr_mult: prefs.smcIfvgAtrMult,
    smc_bpr: prefs.smcBpr, smc_bpr_show_last: prefs.smcBprShowLast,
    smc_bpr_atr_mult: prefs.smcBprAtrMult, smc_bpr_fenetre: prefs.smcBprFenetre, smc_bpr_mitigation: prefs.smcBprMitigation,
    smc_imbalance: prefs.smcImbalance, smc_imb_show_last: prefs.smcImbShowLast,
    smc_imb_show_fvg: prefs.smcImbShowFvg, smc_imb_show_og: prefs.smcImbShowOg, smc_imb_mitigation: prefs.smcImbMitigation,
    smc_fib: prefs.smcFib, smc_tendance: prefs.smcTendance, smc_liquidites: prefs.smcLiquidites,
    smc_liq_swings: prefs.smcLiqSwingsActif,
    smc_liq_sessions: prefs.smcLiqSessionsActif,
    smc_liq_session_asie: prefs.smcLiqSessionAsie,
    smc_liq_dwm: prefs.smcLiqDwmActif,
    smc_liq_dwm_nb: prefs.smcLiqDwmNbJours,
    smc_liq_asie_range: prefs.smcAsianSession,
    smc_liq_asie_heure_debut: prefs.smcLiqAsieHeureDebut,
    smc_liq_asie_heure_fin: prefs.smcLiqAsieHeureFin,
    smc_liq_asie_deviations_nb: prefs.smcLiqAsieDeviationsNb,
    smc_liq_asie_nb_sessions: prefs.smcLiqAsieNbSessions,
    smc_bos: prefs.smcBos,
    smc_choch: prefs.smcChoch,
    signaux: true, limit: 500,
  }
}

/** Options communes à tous les sous-graphiques lightweight-charts */
export function creerOptionsSousGraphique(container: HTMLElement) {
  return {
    layout: { background: { color: 'transparent' }, textColor: '#9ca3af' },
    grid: {
      vertLines: { color: 'rgba(255,255,255,0.05)' },
      horzLines: { visible: false },
    },
    crosshair: { mode: 1, horzLine: { visible: false } },
    rightPriceScale: { borderColor: 'rgba(255,255,255,0.1)', minimumWidth: 80 },
    timeScale: {
      borderColor: 'rgba(255,255,255,0.1)',
      timeVisible: true,
      secondsVisible: false,
      tickMarkFormatter: tickMarkFormatterParis,
    },
    width: container.clientWidth,
    height: container.clientHeight,
  }
}
