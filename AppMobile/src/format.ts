/** Prix des assets à 2 décimales partout (owner 20/10). */
export function prix2(v?: number | null): string {
  return v === undefined || v === null ? '—' : v.toFixed(2)
}
