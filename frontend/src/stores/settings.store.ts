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
  // SMC couleurs & opacités
  // Imbalance (FVG + OG)
  // Range session Asie
  // Structure SMC : BOS / CHoCH
  // Range session asiatique (bouton standalone)
  // Tendance EMA Multi-TF
  kasperTendance: boolean
  kasperPeriodeRapide: number
  kasperPeriodeLente: number
  kasperModeCalcul: 'bougie_cloturee' | 'bougie_en_cours'

  // ── SMC v12 (overlay useSmcV12Overlay) ── bascules ON/OFF par indicateur.
  // Structure & price action
  v12Structure: boolean   // étiquettes HH/HL/LH/LL
  v12Bos: boolean         // ligne continue + label
  v12Mss: boolean         // ligne dashed + label
  v12Choch: boolean       // ligne solid épaisse + label
  v12Sweeps: boolean      // label sur bougie
  v12EqhEql: boolean      // lignes Equal High / Equal Low
  v12Tendance: boolean    // bgcolor vertical de tendance
  // Order Blocks & FVG
  v12Ob: boolean          // bloc + label "OB x/10"
  v12Fvg: boolean         // bloc
  v12Breaker: boolean
  v12Propulsion: boolean     // bloc breaker
  v12Imbalance: boolean   // bloc imbalance
  // Signaux & zones
  v12ZoneCoeur: boolean   // bloc zone d'achat/vente
  v12Signals: boolean     // box trade SL/TP + label BUY/SELL
  // Technique avancé (bgcolor)
  v12Volume: boolean      // bgcolor volume
  v12Impulsion: boolean   // bgcolor impulsions fortes
  // Sessions & niveaux clés
  v12SessionAsie: boolean
  v12SessionLondres: boolean
  v12SessionNy: boolean
  v12AsianHl: boolean     // Asian high / Low
  v12NiveauxCles: boolean // PDH / PDL / PWH / PWL
  v12Ndog: boolean        // bloc NDOG
  v12Nwog: boolean        // bloc NWOG
  // Multi-TF & OTE
  v12Premium: boolean     // bgcolor premium / discount
  v12Equilibrium: boolean // ligne equilibrium
  v12ObH1: boolean
  v12ObH4: boolean
  v12ObW1: boolean
  v12ObMn: boolean
  v12Ote: boolean         // zone OTE
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
  // SMC couleurs & opacités
  // Imbalance
  // Range session Asie
  // Structure SMC : BOS / CHoCH
  // Tendance EMA Multi-TF
  kasperTendance: false,
  kasperPeriodeRapide: 9,
  kasperPeriodeLente: 21,
  kasperModeCalcul: 'bougie_cloturee',

  // ── SMC v12 ── ON par défaut pour les indicateurs réellement retournés par
  // l'API (/api/smc/v12/analyse) et déjà dessinés ; OFF pour les autres
  // (données non encore exposées par le backend ou bgcolor distractants).
  v12Structure: true,
  v12Bos: true,
  v12Mss: true,
  v12Choch: true,
  v12Sweeps: true,
  v12EqhEql: false,
  v12Tendance: false,
  v12Ob: true,
  v12Fvg: true,
  v12Breaker: false,
  v12Propulsion: false,
  v12Imbalance: false,
  v12ZoneCoeur: false,
  v12Signals: true,
  v12Volume: false,
  v12Impulsion: false,
  v12SessionAsie: false,
  v12SessionLondres: false,
  v12SessionNy: false,
  v12AsianHl: false,
  v12NiveauxCles: false,
  v12Ndog: false,
  v12Nwog: false,
  v12Premium: false,
  v12Equilibrium: false,
  v12ObH1: false,
  v12ObH4: false,
  v12ObW1: false,
  v12ObMn: false,
  v12Ote: false,
}

function chargerIndicateurs(): PrefsIndicateurs {
  try {
    const raw = localStorage.getItem(CLE_INDICATEURS)
    if (raw) {
      return { ...INDICATEURS_DEFAUT, ...JSON.parse(raw) }
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

