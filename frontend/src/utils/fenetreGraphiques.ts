/// Fenêtre Graphiques indépendante (owner 06/10) — ouvre la page des
/// graphiques dans une FENÊTRE TAURI NATIVE plein écran, séparée du
/// dashboard. Si la fenêtre existe déjà : ramenée au premier plan (pas de
/// doublon). Repli navigateur (dev/test) : navigation normale.
///
/// Utilisé par le clic « Graphiques » du pedestal dashboard.

const LABEL_FENETRE = 'Graphiques — Native Trading AI'

interface TauriWindow {
  WebviewWindow: new (label: string, options: Record<string, unknown>) => Promise<{ focus: () => void; onCloseRequested: (cb: () => void) => void }>
  getAll: () => Promise<{ label: string; focus: () => void }[]>
}

/// Vrai si on tourne dans la fenêtre Tauri (pas un navigateur).
function dansTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function apiTauri(): Promise<TauriWindow | null> {
  if (!dansTauri()) return null
  try {
    return (await import('@tauri-apps/api/window')) as unknown as TauriWindow
  } catch { return null }
}

/// Ouvre (ou focus) la fenêtre Graphiques plein écran.
/// `route` : la route interne à charger (ex: '/smc/graphiques').
export async function ouvrirFenetreGraphiques(route: string): Promise<'fenetre' | 'navigation'> {
  const tauri = await apiTauri()
  if (!tauri) return 'navigation'

  // Déjà ouverte ? → focus, pas de doublon.
  const fenetres = await tauri.getAll()
  const existante = fenetres.find((f: { label: string }) => f.label === LABEL_FENETRE)
  if (existante) {
    await existante.focus()
    return 'fenetre'
  }

  // Création : plein écran (maximized), URL = même serveur + route.
  const url = `${window.location.origin}/#${route}`
  await new tauri.WebviewWindow(LABEL_FENETRE, {
    url,
    title: LABEL_FENETRE,
    width: 1280,
    height: 800,
    maximized: true,
    center: true,
    resizable: true,
    fullscreen: false,
  })
  return 'fenetre'
}
