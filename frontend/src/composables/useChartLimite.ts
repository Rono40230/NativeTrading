/** Nombre de bougies chargées pour le chart principal. */

/**
 * Aligné sur TradingView Basic (gratuit, vérifié 2026-08-17) :
 * 5 000 barres affichables, quel que soit le timeframe.
 *
 * L'ancienne logique « par jours » (M15 = 384 bougies ≈ 4 jours)
 * tronquait artificiellement la fenêtre malgré l'historique de 2 ans
 * disponible en DB — c'est elle qui donnait l'impression d'un chart
 * limité à ~3 jours.
 */
export const LIMITE_TV_BASIC = 5_000

/** Même limite pour tous les TFs — la signature reste (tf) pour les appelants. */
export function limitPourTimeframe(_tf: string): number {
  return LIMITE_TV_BASIC
}
