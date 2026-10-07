/// Fenêtre Graphiques indépendante (owner 06/10) — ouvre la page des
/// graphiques dans une FENÊTRE TAURI NATIVE plein écran, séparée du
/// dashboard. Si la fenêtre existe déjà : ramenée au premier plan (pas de
/// doublon). Repli navigateur (dev/test) : navigation normale.
///
/// Utilisé par le clic « Graphiques » du pedestal dashboard.

const LABEL_FENETRE = 'Graphiques — Native Trading AI'

/// Vrai si on tourne dans la fenêtre Tauri (pas un navigateur).
function dansTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/// Ouvre (ou focus) la fenêtre Graphiques plein écran.
/// `route` : la route interne à charger (ex: '/smc/graphiques').
export async function ouvrirFenetreGraphiques(route: string): Promise<'fenetre' | 'navigation'> {
  if (!dansTauri()) return 'navigation'

  try {
    const { WebviewWindow, getAllWebviewWindows } = await import('@tauri-apps/api/webviewWindow')

    // Déjà ouverte ? → focus, pas de doublon.
    const fenetres = await getAllWebviewWindows()
    const existante = fenetres.find((f: { label: string }) => f.label === LABEL_FENETRE)
    if (existante) {
      await existante.setFocus()
      return 'fenetre'
    }

    // Création : plein écran (maximized), URL = même serveur + route.
    const url = `${window.location.origin}/#${route}`
    const fenetre = new WebviewWindow(LABEL_FENETRE, {
      url,
      title: LABEL_FENETRE,
      width: 1280,
      height: 800,
      maximized: true,
      center: true,
      resizable: true,
      fullscreen: false,
    })
    // La création est asynchrone : attendre l'événement de fin (ou erreur
    // silencieuse — la fenêtre s'ouvre quand même).
    await new Promise<void>((resoudre) => {
      fenetre.once('tauri:created', () => resoudre())
      fenetre.once('tauri:error', () => resoudre())
      setTimeout(resoudre, 3000)
    })
    return 'fenetre'
  } catch {
    return 'navigation'
  }
}
