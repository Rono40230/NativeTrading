/// Rapprochement timestamp → nom d'événement de la taxonomie straddle
/// (owner 14/10 : colonne « Événement » de l'historique). La taxonomie
/// est embarquée côté front pour éviter un aller-retour API par ligne.
/// Miroir de evenements.rs : structurels + types calendrier.

interface EvenementTs {
  /** Heure d'origine en epoch UTC, pour un jour typique (lundi). */
  heureUtc: number
  nom: string
  /** Les types calendrier (cal_*) n'ont pas d'heure fixe : rattachés par
   *  l'heure de l'annonce réelle (8:30 NY ≈ 12:30/13:30 UTC). */
  plageUtc?: [number, number]
}

const TZ_PARIS = 'Europe/Paris'

/** Décode le timestamp d'une cle_moteur straddle → nom d'événement. */
export function evenementNom(ts: number): string {
  const d = new Date(ts * 1000)
  const hhmm = parseInt(
    new Intl.DateTimeFormat('fr-FR', { hour: '2-digit', minute: '2-digit', hour12: false, timeZone: 'America/New_York' }).format(d).replace(':', ''),
    10
  )
  const hhmmParis = parseInt(
    new Intl.DateTimeFormat('fr-FR', { hour: '2-digit', minute: '2-digit', hour12: false, timeZone: TZ_PARIS }).format(d).replace(':', ''),
    10
  )
  const jour = new Intl.DateTimeFormat('fr-FR', { weekday: 'short', timeZone: TZ_PARIS }).format(d)

  // 1. Ouvertures/clôtures/fixes structurels (fuseau d'origine)
  if (hhmm === 930 && jour !== 'dim.' && jour !== 'sam.') return 'Ouverture NYSE'
  if (hhmm === 1600 && jour !== 'dim.' && jour !== 'sam.') return 'Clôture NYSE'
  if (hhmmParis === 900 && jour !== 'dim.' && jour !== 'sam.') return 'Ouverture Francfort+Londres'
  if (hhmmParis === 1730 && jour !== 'dim.' && jour !== 'sam.') return 'Clôture Londres'
  if (hhmmParis >= 0 && hhmmParis < 100 && jour !== 'dim.') return 'Réouverture CME'
  if (hhmm === 1800 && jour !== 'dim.' && jour !== 'sam.') return 'Réouverture CME'
  if (hhmmParis >= 100 && hhmmParis < 900 && hhmmParis !== 900) {
    // 2h Paris été / 1h hiver = Tokyo 9h
    if (hhmmParis <= 300) return 'Ouverture Tokyo'
  }
  if (hhmmParis === 1130 && jour !== 'dim.' && jour !== 'sam.') return 'Fix LBMA matin'
  if (hhmmParis === 1600 && jour !== 'dim.' && jour !== 'sam.') return 'Fix LBMA PM + annonces 10:00'
  if (jour === 'dim.' && hhmmParis >= 2250) return 'Réouverture week-end'

  // 2. Types calendrier (annonces US 8:30 = 12:30 UTC été / 13:30 hiver)
  if ((hhmm === 830 || hhmm === 831) && jour !== 'dim.' && jour !== 'sam.') return 'Annonce US 8:30'
  if (hhmm === 1000 && jour !== 'dim.' && jour !== 'sam.') return 'Annonce US 10:00'
  if (hhmm === 1400 && jour !== 'dim.' && jour !== 'sam.') return 'FOMC'

  return '—'
}
