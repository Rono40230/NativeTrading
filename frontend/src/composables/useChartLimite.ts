/** Calcul du nombre de bougies à charger selon le timeframe (logique par jours). */

const BOUGIES_PAR_JOUR: Record<string, number> = {
  M1: 1440, M5: 288, M15: 96, M30: 48, H1: 24, H4: 6, D1: 1, W1: 1,
}

const JOURS_PAR_TF: Record<string, number> = {
  M1: 1, M5: 3, M15: 4, M30: 7, H1: 10, H4: 30, D1: 90, W1: 104,
}

/** Minimum garanti pour que ATR200 et les algos SMC aient assez de données. */
const MINIMUM_BOUGIES = 300

export function limitPourTimeframe(tf: string): number {
  const parJour = BOUGIES_PAR_JOUR[tf] ?? 96
  const jours   = JOURS_PAR_TF[tf]    ?? 4
  return Math.max(parJour * jours, MINIMUM_BOUGIES)
}
