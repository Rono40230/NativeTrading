import { TickMarkType } from 'lightweight-charts'

function formatDateJourMois(date: Date): string {
  return new Intl.DateTimeFormat('fr-FR', {
    day: '2-digit',
    month: '2-digit',
  }).format(date)
}

function formatHeureMinute(date: Date): string {
  return new Intl.DateTimeFormat('fr-FR', {
    hour: '2-digit',
    minute: '2-digit',
  }).format(date)
}

export function tickMarkFormatterEquity(ts: number, markType: TickMarkType): string {
  void markType
  const date = new Date(ts * 1000)
  return formatDateJourMois(date)
}

export function tickMarkFormatterMl(ts: number): string {
  const date = new Date(ts * 1000)
  return `${formatDateJourMois(date)} ${formatHeureMinute(date)}`
}