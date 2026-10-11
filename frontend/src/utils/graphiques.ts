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

/// Ouvre (ou ramène au premier plan) la FENÊTRE Graphiques indépendante
/// (owner 07/10), en ciblant le premier slot sur un asset. décorations
/// désactivées : UNE SEULE barre — celle de l'app, contrôles ─ □ ✕
/// fonctionnels (écart 18 du 11/10 : la fenêtre avait SA barre native en
/// plus). Le re-ciblage d'une fenêtre déjà ouverte passe par l'événement
/// `storage` (localStorage partagé même origine) — la page l'écoute.
export async function ouvrirFenetreGraphiques(asset?: string, timeframe?: string): Promise<void> {
  if (asset) ciblerPremierSlot(asset, timeframe)
  const estTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
  if (!estTauri) return // repli : l'appelant navigue (router.push)
  try {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow')
    const existante = await WebviewWindow.getByLabel('Graphiques')
    if (existante) {
      // L'événement storage déclenche le re-ciblage dans la fenêtre ouverte.
      localStorage.setItem(CLE_SLOTS, localStorage.getItem(CLE_SLOTS) ?? '[]')
      await existante.setFocus()
      return
    }
    new WebviewWindow('Graphiques', {
      url: `${window.location.origin}/smc/graphiques`,
      title: 'Graphiques',
      width: 1280,
      height: 800,
      maximized: true,
      center: true,
      decorations: false,
    })
  } catch {
    // repli silencieux : l'appelant peut naviguer en interne
  }
}
