/**
 * Helpers de formatage temporel — convention applicative unifiée :
 * **stockage UTC, affichage Europe/Paris** (CET/CEST auto via base IANA).
 *
 * Tous les affichages "wall-clock" doivent passer par ces fonctions plutôt que
 * par des `toLocale*` / `Intl.DateTimeFormat` nus (qui utiliseraient le fuseau
 * du navigateur/serveur). Les timestamps numériques sont en **secondes Unix**
 * (convention backend) ; pour des millisecondes, passer un `Date`.
 */

const TZ_PARIS = 'Europe/Paris' as const

/** Formate une date/heure en heure de Paris. `number` = secondes Unix. */
export function formatParis(date: Date | number, opts?: Intl.DateTimeFormatOptions): string {
  const d = typeof date === 'number' ? new Date(date * 1000) : date
  return new Intl.DateTimeFormat('fr-FR', { timeZone: TZ_PARIS, ...opts }).format(d)
}

/** "HH:MM" en heure de Paris depuis des secondes Unix. */
export function heureParis(ts: number): string {
  return formatParis(ts, { hour: '2-digit', minute: '2-digit' })
}

/** Date + heure courtes en heure de Paris ("JJ/MM/AAAA HH:MM"). */
export function dateHeureParis(ts: number): string {
  return formatParis(ts, { dateStyle: 'short', timeStyle: 'short' })
}

/**
 * Jour de la semaine en heure de Paris, convention lundi-based :
 * 0=Lundi ... 6=Dimanche (convention applicative creneaux/signaux).
 */
export function jourSemaineParis(ts: number): number {
  const d = new Date(ts * 1000)
  const parisDay = new Intl.DateTimeFormat('en-US', { timeZone: TZ_PARIS, weekday: 'short' }).format(d)
  const map: Record<string, number> = { Mon: 0, Tue: 1, Wed: 2, Thu: 3, Fri: 4, Sat: 5, Sun: 6 }
  return map[parisDay] ?? 0
}

/**
 * Décalage UTC→Paris actuel en heures entières : 1 (CET/hiver) ou 2 (CEST/été).
 * Calculé dynamiquement à chaque appel (DST géré par la base IANA).
 */
export function offsetParisHeures(): number {
  const maintenant = new Date()
  const hParis = Number(new Intl.DateTimeFormat('en-US', { timeZone: TZ_PARIS, hour: 'numeric', hour12: false }).format(maintenant))
  const hUtc = Number(new Intl.DateTimeFormat('en-US', { timeZone: 'UTC', hour: 'numeric', hour12: false }).format(maintenant))
  return (((hParis - hUtc) % 24) + 24) % 24 || 1
}
