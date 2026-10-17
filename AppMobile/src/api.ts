/**
 * Client API du relais. Base : vide = même origine (dev via proxy Vite) ;
 * en APK, Réglages pose l'URL du serveur (Tailscale). Token optionnel
 * (Bearer) si le propriétaire en a posé un côté serveur.
 */
import type { Alerte, AlerteComplete, Aujourdhui, EtatServeur } from './types'

export const reglages = {
  get serveur(): string {
    return localStorage.getItem('nt_serveur') ?? ''
  },
  set serveur(v: string) {
    localStorage.setItem('nt_serveur', v.replace(/\/+$/, ''))
  },
  get token(): string {
    return localStorage.getItem('nt_token') ?? ''
  },
  set token(v: string) {
    localStorage.setItem('nt_token', v)
  },
}

async function appel<T>(chemin: string, init?: RequestInit): Promise<T> {
  const entetes: Record<string, string> = {}
  if (reglages.token) entetes['Authorization'] = `Bearer ${reglages.token}`
  const rep = await fetch(`${reglages.serveur}${chemin}`, { ...init, headers: entetes })
  if (!rep.ok) throw new Error(`HTTP ${rep.status}`)
  return rep.json() as Promise<T>
}

export const api = {
  etat: () => appel<EtatServeur>('/api/mobile/etat'),
  journal: (avantId?: number) =>
    appel<{ alertes: Alerte[] }>(`/api/mobile/journal?limite=60${avantId ? `&avant_id=${avantId}` : ''}`),
  aujourdhui: () => appel<Aujourdhui>('/api/mobile/aujourdhui'),
  alerte: (id: number) => appel<AlerteComplete>(`/api/mobile/alerte/${id}`),
  urlPng: (id: number) => `${reglages.serveur}/api/mobile/alerte/${id}/png`,
  genererBrief: () =>
    appel<{ id: number; contenu: string; nb_articles: number }>('/api/mobile/brief', { method: 'POST' }),
}

/// Muette 21h→8h Paris (spec owner) — le client ne sonne jamais dedans.
export function muetteActive(d = new Date()): boolean {
  const h = d.getHours()
  return h >= 21 || h < 8
}
