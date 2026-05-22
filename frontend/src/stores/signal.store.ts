import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiService, type Signal, type PredictionML, type ScoreSmc, type RequeteAnalyseIA } from '@/services/api.service'
import { useSettingsStore } from '@/stores/settings.store'

export const useSignalStore = defineStore('signals', () => {
  const signaux = ref<Signal[]>([])
  const prediction = ref<PredictionML | null>(null)
  const scoreSmc = ref<ScoreSmc | null>(null)
  const analyseIaTexte = ref<string | null>(null)
  const analyseIaChargement = ref(false)
  const slAnalyse = ref<number | null>(null)
  const tp1Analyse = ref<number | null>(null)
  const tp2Analyse = ref<number | null>(null)
  const chargement = ref(false)
  const erreur = ref<string | null>(null)

  async function chargerSignaux(limit = 20) {
    chargement.value = true
    erreur.value = null
    try {
      signaux.value = await apiService.getSignaux(limit)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur réseau'
      erreur.value = msg
    } finally {
      chargement.value = false
    }
  }

  async function chargerPrediction(asset: string, timeframe = 'M15') {
    try {
      prediction.value = await apiService.predictML(asset, timeframe)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur ML'
      erreur.value = msg
    }
  }

  async function chargerScoreSmc(asset: string, timeframe = 'M15') {
    try {
      scoreSmc.value = await apiService.analyseSmc(asset, timeframe)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Erreur SMC'
      erreur.value = msg
    }
  }

  async function chargerAnalyseIA(asset: string, timeframe: string, prixEntree: number, confianceMl: number, atr = 0) {
    analyseIaChargement.value = true
    analyseIaTexte.value = null
    slAnalyse.value = null
    tp1Analyse.value = null
    tp2Analyse.value = null
    erreur.value = null
    try {
      const score = await apiService.analyseSmc(asset, timeframe)
      scoreSmc.value = score
      const isLong = score.direction.toLowerCase().includes('long') || score.direction.toLowerCase().includes('buy')
      const sl = atr > 0 ? (isLong ? prixEntree - 2.0 * atr : prixEntree + 2.0 * atr) : 0
      const tp1 = atr > 0 ? (isLong ? prixEntree + 2.0 * atr : prixEntree - 2.0 * atr) : 0
      const tp2 = atr > 0 ? (isLong ? prixEntree + 3.0 * atr : prixEntree - 3.0 * atr) : 0
      slAnalyse.value = sl || null
      tp1Analyse.value = tp1 || null
      tp2Analyse.value = tp2 || null
      const requete: RequeteAnalyseIA = {
        asset, timeframe,
        direction: score.direction,
        score_smc: score.total,
        prix_entree: prixEntree,
        stop_loss: sl,
        take_profit_1: tp1,
        take_profit_2: tp2 || undefined,
        tendance: score.tendance,
        order_block: score.order_block,
        imbalance: score.imbalance,
        ifvg: score.ifvg,
        fibonacci: score.fibonacci,
        confiance_ml: confianceMl,
      }
      const reponse = await apiService.analyserIA(requete)
      analyseIaTexte.value = reponse.analyse
    } catch (err: unknown) {
      erreur.value = err instanceof Error ? err.message : 'Erreur analyse IA'
    } finally {
      analyseIaChargement.value = false
    }
  }

  /** Injecte un signal reçu en temps réel (WebSocket Signal Engine) en tête de liste. */
  function ajouterSignalTempsReel(signal: Signal) {
    signaux.value = [signal, ...signaux.value].slice(0, 50)
  }

  return {
    signaux,
    prediction,
    scoreSmc,
    analyseIaTexte,
    analyseIaChargement,
    slAnalyse,
    tp1Analyse,
    tp2Analyse,
    chargement,
    erreur,
    chargerSignaux,
    chargerPrediction,
    chargerScoreSmc,
    chargerAnalyseIA,
    ajouterSignalTempsReel,
  }
})
