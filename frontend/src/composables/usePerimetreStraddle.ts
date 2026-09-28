/// Périmètre straddle partagé (28/09) — la sélection de la modale
/// « 🎯 Choix des Assets & créneaux » pilote l'affichage des blocs de la
/// modale Volatilité : événements prévisibles (chips atténuées hors
/// périmètre, tri par meilleure réaction DANS le périmètre) et créneaux
/// moyens (cartes du périmètre en tête). Les deux blocs montent ensemble
/// à l'ouverture de la modale : un seul fetch partagé, cache 30 s — chaque
/// réouverture remonte les composants et rafraîchit.
import { http } from '@/services/http.client'

let cache: { ts: number; promesse: Promise<string[]> } | null = null

export async function lirePerimetreStraddle(): Promise<string[]> {
  if (cache && Date.now() - cache.ts < 30_000) return cache.promesse
  const promesse = http
    .get<{ assets: string[] }>('/api/straddle/perimetre')
    .then(r => r.data.assets ?? [])
    .catch(() => [] as string[])
  cache = { ts: Date.now(), promesse }
  return promesse
}
