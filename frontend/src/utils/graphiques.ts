/**
 * Ciblage du premier graphique de la page Graphiques.
 * La page restaure sa grille depuis localStorage (cle `trading_slots_graphiques`)
 * — pour OUVRIR un asset précis (alerte prix, signal), il faut écrire dans
 * ce premier slot avant de naviguer.
 */
export const CLE_SLOTS = 'trading_slots_graphiques'

export function ciblerPremierSlot(asset: string, timeframe?: string) {
  try {
    const slots = JSON.parse(localStorage.getItem(CLE_SLOTS) ?? '[]') as {
      asset: string
      timeframe: string
    }[]
    if (Array.isArray(slots) && slots.length > 0) {
      slots[0] = { ...slots[0], asset, ...(timeframe ? { timeframe } : {}) }
      localStorage.setItem(CLE_SLOTS, JSON.stringify(slots))
    }
  } catch {
    // grille illisible : la page retombera sur les réglages actifs
  }
}
