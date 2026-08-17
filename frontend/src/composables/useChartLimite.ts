/** Calcul du nombre de bougies à charger selon le timeframe (logique par jours). */

const BOUGIES_PAR_JOUR: Record<string, number> = {
  M1: 1440, M5: 288, M15: 96, M30: 48, H1: 24, H4: 6, D1: 1, W1: 1,
}

const JOURS_PAR_TF: Record<string, number> = {
  M1: 1, M5: 3, M15: 4, M30: 7, H1: 10, H4: 30, D1: 90, W1: 104,
}

/** Minimum garanti pour que ATR200 et les algos SMC aient assez de données. */
const MINIMUM_BOUGIES = 300

/** Aligné sur TradingView Basic (gratuit, 2026-08-17) : 5 000 barres
 *  intraday ; daily et au-delà = tout l'historique disponible (le plafond
 *  backend est aussi 5 000 par requête). */
const LIMITE_TV_BASIC = 5_000

/** H1 et au-delà : plafond fixe (tout l'historique jusqu'à la limite). */
const TF_PLAFOND = new Set(['H1', 'H4', 'D1', 'W1'])

export function limitPourTimeframe(tf: string): number {
  if (TF_PLAFOND.has(tf)) return LIMITE_TV_BASIC
  const parJour = BOUGIES_PAR_JOUR[tf] ?? 96
  const jours   = JOURS_PAR_TF[tf]    ?? 4
  return Math.min(Math.max(parJour * jours, MINIMUM_BOUGIES), LIMITE_TV_BASIC)
}
