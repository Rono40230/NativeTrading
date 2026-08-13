import { TickMarkType } from 'lightweight-charts'
import { formatParis } from '@/utils/date'

export function tickMarkFormatterEquity(ts: number, markType: TickMarkType): string {
  void markType
  return formatParis(ts, { day: '2-digit', month: '2-digit' })
}

export function tickMarkFormatterMl(ts: number, markType: TickMarkType): string {
  void markType
  return formatParis(ts, { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
}
