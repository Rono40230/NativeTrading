import type { ISeriesApi, SeriesMarker, Time } from 'lightweight-charts'
import { filtrerSignaux, FORCE_ORDRE } from './chartSignauxTypes'
import type { SignalIndicateur, FiltreSignaux } from './chartSignauxTypes'

// ── Conversion signal → marqueur LW-Charts ────────────────────────────────────

function signalVersMarqueur(s: SignalIndicateur): SeriesMarker<Time> {
  const taille = FORCE_ORDRE[s.force] as 1 | 2 | 3
  const couleur =
    s.direction === 'bullish' ? '#10b981'
    : s.direction === 'bearish' ? '#ef4444'
    : '#94a3b8'
  return {
    time: s.timestamp as unknown as Time,
    position: s.direction === 'bullish' ? 'belowBar'
      : s.direction === 'bearish' ? 'aboveBar'
      : 'inBar',
    color: couleur,
    shape: s.direction === 'bullish' ? 'arrowUp'
      : s.direction === 'bearish' ? 'arrowDown'
      : 'circle',
    text: s.source,
    size: taille,
    id: `${s.source}_${s.type_signal}_${s.timestamp}`,
  }
}

// ── Déduplification : un seul marqueur bullish + un seul bearish par timestamp ─

function deduplicerParTimestamp(signaux: SignalIndicateur[]): SignalIndicateur[] {
  const parTimestamp = new Map<number, Map<string, SignalIndicateur>>()
  for (const s of signaux) {
    const directions = parTimestamp.get(s.timestamp) ?? new Map<string, SignalIndicateur>()
    const existant = directions.get(s.direction)
    // Garder le signal le plus fort pour chaque direction à ce timestamp
    if (!existant || FORCE_ORDRE[s.force] > FORCE_ORDRE[existant.force]) {
      directions.set(s.direction, s)
    }
    parTimestamp.set(s.timestamp, directions)
  }
  const resultat: SignalIndicateur[] = []
  for (const directions of parTimestamp.values()) {
    resultat.push(...directions.values())
  }
  return resultat
}

// ── API publique ──────────────────────────────────────────────────────────────

/**
 * Applique les marqueurs de signaux sur la série candlestick.
 * Filtre selon `filtre`, déduplique par timestamp et trie par temps.
 */
export function rendreSurSerie(
  serie: ISeriesApi<'Candlestick'>,
  signaux: SignalIndicateur[],
  filtre: FiltreSignaux,
): void {
  const filtres = filtrerSignaux(signaux, filtre)
  const dedupliques = deduplicerParTimestamp(filtres)

  const marqueurs: SeriesMarker<Time>[] = dedupliques.map(signalVersMarqueur)
  // LW-Charts exige un tri croissant par time
  marqueurs.sort((a, b) => (a.time as number) - (b.time as number))
  serie.setMarkers(marqueurs)
}

/** Efface tous les marqueurs de la série */
export function effacerMarqueurs(serie: ISeriesApi<'Candlestick'> | null): void {
  serie?.setMarkers([])
}
