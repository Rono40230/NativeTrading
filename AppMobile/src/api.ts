/**
 * Client API du relais. Base : vide = même origine (dev via proxy Vite) ;
 * en APK, Réglages pose l'URL du serveur (Tailscale). Token optionnel
 * (Bearer) si le propriétaire en a posé un côté serveur.
 */
import type { ActifMarche, Activite, Alerte, EvenementCal, ZoneBande, AlerteComplete, Aujourdhui, EtatServeur, Presse, PrixPage, AlertePrixPrincipale } from './types'

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
  journal: (avantId?: number, type?: string, fenetreMin?: number) =>
    appel<{ alertes: Alerte[] }>(
      `/api/mobile/journal?limite=60${avantId ? `&avant_id=${avantId}` : ''}${type ? `&type=${type}` : ''}${fenetreMin ? `&fenetre=${fenetreMin}` : ''}`,
    ),
  aujourdhui: () => appel<Aujourdhui>('/api/mobile/aujourdhui'),
  presse: () => appel<Presse>('/api/mobile/presse'),
  activite: () => appel<Activite>('/api/mobile/activite'),
  marche: () => appel<{ actifs: ActifMarche[] }>('/api/mobile/marche'),
  calendrier: () => appel<{ evenements: EvenementCal[] }>('/api/mobile/calendrier'),
  zones: () => appel<{ en_bande: ZoneBande[] }>('/api/smc/zones-approche'),
  prix: () => appel<PrixPage>('/api/mobile/prix'),
  /// Endpoint de l'app principale — repli « niveaux armés » tant que le
  /// binaire mobile /api/mobile/prix n'est pas déployé (relance run.sh).
  alertesPrixPrincipales: () => appel<AlertePrixPrincipale[]>('/api/alertes-prix'),
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
