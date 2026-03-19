import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

const CLE_CAPITAL = 'trading_capital_depart'
const CLE_ASSET = 'trading_asset_actif'
const CLE_TIMEFRAME = 'trading_timeframe_actif'
const CLE_INDICATEURS = 'trading_indicateurs'
const CAPITAL_DEFAUT = 2000

export interface PrefsIndicateurs {
  ema: boolean
  rsi: boolean
  macd: boolean
  bollinger: boolean
  atr: boolean
  emaPeriode: number
  emaMaType: 'ema' | 'sma'
  emaCouleur: string
  rsiPeriode: number
  rsiSurachat: number
  rsiSurvente: number
  rsiCouleur: string
  macdRapide: number
  macdLente: number
  macdSignal: number
  bollingerPeriode: number
  bollingerStdDev: number
  bollingerMaType: 'sma' | 'ema'
  bollingerCouleurHaute: string
  bollingerCouleurMilieu: string
  bollingerCouleurBasse: string
  atrPeriode: number
  atrCouleur: string
  smcOb: boolean
  smcBpr: boolean
  smcIfvg: boolean
  smcImbalance: boolean
  smcFib: boolean
  smcTendance: boolean
  smcLiquidites: boolean
  // SMC couleurs & opacités
  smcObCouleurLong: string
  smcObCouleurShort: string
  smcObOpacite: number
  smcObSensibilite: number
  smcObMitigationType: 'close' | 'wick'
  smcIfvgCouleurLong: string
  smcIfvgCouleurShort: string
  smcIfvgOpacite: number
  smcIfvgShowLast: number
  smcIfvgSignalPref: 'close' | 'wick'
  smcIfvgAtrMult: number
  smcBprCouleurBull: string
  smcBprCouleurBear: string
  smcBprOpacite: number
  smcBprShowLast: number
  smcBprAtrMult: number
  smcBprFenetre: number
  smcBprMitigation: 'close' | 'wick'
  // Imbalance (FVG + OG)
  smcImbCouleurBull: string
  smcImbCouleurBear: string
  smcImbOpacite: number
  smcImbShowLast: number
  smcImbShowFvg: boolean
  smcImbShowOg: boolean
  smcImbMitigation: 'close' | 'wick'
  smcFibCouleur: string
  smcFibAfficher236: boolean
  smcFibAfficher786: boolean
  smcLiqCouleurBsl: string
  smcLiqCouleurSsl: string
  smcLiqSwingsActif: boolean
  smcLiqSessionsActif: boolean
  smcLiqSessionAsie: boolean
  smcLiqDwmActif: boolean
  smcLiqDwmNbJours: number
  smcLiqCouleurAsie: string
  smcLiqCouleurDwm: string
  // Range session Asie
  smcLiqAsieRangeActif: boolean
  smcLiqAsieDeviationsActif: boolean
  smcLiqAsieDeviationsNb: number
  smcLiqAsieNbSessions: number
  smcLiqAsieHeureDebut: number
  smcLiqAsieHeureFin: number
  smcLiqAsieOpacite: number
  smcLiqAsieCouleur: string
  // Tendance EMA Multi-TF
  kasperTendance: boolean
  kasperPeriodeRapide: number
  kasperPeriodeLente: number
  kasperModeCalcul: 'bougie_cloturee' | 'bougie_en_cours'
}

const INDICATEURS_DEFAUT: PrefsIndicateurs = {
  ema: true,
  rsi: false,
  macd: false,
  bollinger: false,
  atr: false,
  emaPeriode: 20,
  emaMaType: 'ema',
  emaCouleur: '#f59e0b',
  rsiPeriode: 14,
  rsiSurachat: 70,
  rsiSurvente: 30,
  rsiCouleur: '#a855f7',
  macdRapide: 12,
  macdLente: 26,
  macdSignal: 9,
  bollingerPeriode: 20,
  bollingerStdDev: 2.0,
  bollingerMaType: 'sma',
  bollingerCouleurHaute: '#6366f1',
  bollingerCouleurMilieu: '#818cf8',
  bollingerCouleurBasse: '#6366f1',
  atrPeriode: 14,
  atrCouleur: '#f43f5e',
  smcOb: true,
  smcBpr: true,
  smcIfvg: true,
  smcImbalance: false,
  smcFib: true,
  smcTendance: true,
  smcLiquidites: true,
  // SMC couleurs & opacités
  smcObCouleurLong: '#10b981',
  smcObCouleurShort: '#ef4444',
  smcObOpacite: 0.25,
  smcObSensibilite: 28,
  smcObMitigationType: 'close',
  smcIfvgCouleurLong: '#6366f1',
  smcIfvgCouleurShort: '#ec4899',
  smcIfvgOpacite: 0.25,
  smcIfvgShowLast: 5,
  smcIfvgSignalPref: 'close',
  smcIfvgAtrMult: 0.25,
  smcBprCouleurBull: '#3b82f6',
  smcBprCouleurBear: '#ef4444',
  smcBprOpacite: 0.25,
  smcBprShowLast: 5,
  smcBprAtrMult: 0.5,
  smcBprFenetre: 30,
  smcBprMitigation: 'close',
  // Imbalance
  smcImbCouleurBull: '#2157f3',
  smcImbCouleurBear: '#ff1100',
  smcImbOpacite: 0.2,
  smcImbShowLast: 5,
  smcImbShowFvg: true,
  smcImbShowOg: true,
  smcImbMitigation: 'close',
  smcFibCouleur: '#94a3b8',
  smcFibAfficher236: true,
  smcFibAfficher786: true,
  smcLiqCouleurBsl: '#10b981',
  smcLiqCouleurSsl: '#ef4444',
  smcLiqSwingsActif: true,
  smcLiqSessionsActif: true,
  smcLiqSessionAsie: true,
  smcLiqDwmActif: false,
  smcLiqDwmNbJours: 2,
  smcLiqCouleurAsie: '#f59e0b',
  smcLiqCouleurDwm: '#94a3b8',
  // Range session Asie
  smcLiqAsieRangeActif: true,
  smcLiqAsieDeviationsActif: true,
  smcLiqAsieDeviationsNb: 2,
  smcLiqAsieNbSessions: 3,
  smcLiqAsieHeureDebut: 20,
  smcLiqAsieHeureFin: 1,
  smcLiqAsieOpacite: 0.15,
  smcLiqAsieCouleur: '#f59e0b',
  // Tendance EMA Multi-TF
  kasperTendance: false,
  kasperPeriodeRapide: 9,
  kasperPeriodeLente: 21,
  kasperModeCalcul: 'bougie_cloturee',
}

function chargerIndicateurs(): PrefsIndicateurs {
  try {
    const raw = localStorage.getItem(CLE_INDICATEURS)
    if (raw) {
      const sauvegarde = JSON.parse(raw)
      // Migration : anciennes valeurs UTC (22h/7h) → nouvelles valeurs heure Paris (20h/1h)
      if (sauvegarde.smcLiqAsieHeureDebut === 22) sauvegarde.smcLiqAsieHeureDebut = 20
      if (sauvegarde.smcLiqAsieHeureFin   === 7)  sauvegarde.smcLiqAsieHeureFin   = 1
      return { ...INDICATEURS_DEFAUT, ...sauvegarde }
    }
  } catch {
    // données corrompues — on repart des valeurs par défaut
  }
  return { ...INDICATEURS_DEFAUT }
}

export const useSettingsStore = defineStore('settings', () => {
  const capitalDepart = ref<number>(
    Number(localStorage.getItem(CLE_CAPITAL)) || CAPITAL_DEFAUT
  )
  const assetActif = ref<string>(localStorage.getItem(CLE_ASSET) || 'BTC')
  const timeframeActif = ref<string>(localStorage.getItem(CLE_TIMEFRAME) || 'M15')
  const indicateurs = ref<PrefsIndicateurs>(chargerIndicateurs())

  watch(capitalDepart, (val) => {
    if (val > 0) localStorage.setItem(CLE_CAPITAL, String(val))
  })
  watch(assetActif, (val) => localStorage.setItem(CLE_ASSET, val))
  watch(timeframeActif, (val) => localStorage.setItem(CLE_TIMEFRAME, val))
  watch(indicateurs, (val) => localStorage.setItem(CLE_INDICATEURS, JSON.stringify(val)), { deep: true })

  function definirCapital(valeur: number) {
    if (valeur > 0) capitalDepart.value = valeur
  }

  function definirAsset(asset: string) {
    assetActif.value = asset
  }

  function definirTimeframe(tf: string) {
    timeframeActif.value = tf
  }

  return {
    capitalDepart,
    assetActif,
    timeframeActif,
    indicateurs,
    definirCapital,
    definirAsset,
    definirTimeframe,
  }
})

