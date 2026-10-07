/// Fenêtre Graphiques indépendante (owner 07/10) — ouvre la page des
/// graphiques dans une fenêtre SÉPARÉE du dashboard.
///
/// V1 : window.open — simple, universel (Tauri webview + navigateur),
/// aucune dépendance à l'API Tauri. La fenêtre s'ouvre plein écran via
/// les features du navigateur. Si déjà ouverte (même URL), le navigateur
/// la réutilise ou en ouvre une nouvelle selon sa politique.
/// V2 (quand stable) : Tauri WebviewWindow natif — voir git history.

/// Ouvre la page Graphiques dans une nouvelle fenêtre plein écran.
export function ouvrirFenetreGraphiques(route: string): 'fenetre' {
  const url = `${window.location.origin}/#${route}`
  window.open(url, '_blank', 'fullscreen=yes,menubar=no,toolbar=no,location=no')
  return 'fenetre'
}
