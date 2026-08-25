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

/**
 * Limite par TF — l'historique Axi couvre 24 mois, le chart en montre
 * ~2 ans là où c'est raisonnable en poids (les TF minute restent à
 * 5 000 : 2 ans de M1 = 720 000 bougies serait ingérable dans le
 * navigateur — la profondeur M1 vit dans les replays moteurs et le
 * tableau de couverture).
 */
export function limitPourTimeframe(tf: string): number {
  switch (tf) {
    case 'M1':
    case 'M5':
      return LIMITE_TV_BASIC            // 5 000 (~3,5 j / ~17 j)
    case 'M15':
      return 50_000                      // ~2 ans
    case 'M30':
      return 35_000                      // ~2 ans
    case 'H1':
      return 17_500                      // ~2 ans
    case 'H4':
      return 8_000                       // ~4,5 ans
    case 'D1':
      return 2_000                       // ~8 ans
    case 'W1':
      return 600
    default:
      return LIMITE_TV_BASIC
  }
}
