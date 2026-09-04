import { describe, it, expect } from 'vitest'
import { palierMax, labelPalierMax, formatR, classePalierMax } from '@/composables/useSignalFormat'
import { fmtDollars, fmtR } from '@/composables/useAnalyses'

const base = {
  strategie: 'SMC',
  direction: 'Long',
  prix_entree: 100,
  stop_loss: 99, // risque 1
  take_profit: [100.6, 102, 103],
}

describe('palierMax — verdicts et R de référence', () => {
  it('SL → −1R', () => {
    expect(palierMax({ ...base, verdict: 'sl' })).toMatchObject({ palier: 'SL', rReference: -1 })
  })
  it('TP1+BE → R réel du TP1 (0,6R), pas 1R en dur', () => {
    const p = palierMax({ ...base, verdict: 'tp1+be' })
    expect(p.palier).toBe('TP1')
    expect(p.rReference).toBeCloseTo(0.6)
  })
  it('TP2+BE → 2R ; TP3 → 3R ; BE → 0 ; Expire → null', () => {
    expect(palierMax({ ...base, verdict: 'tp2+be' }).rReference).toBeCloseTo(2)
    expect(palierMax({ ...base, verdict: 'tp3' }).rReference).toBeCloseTo(3)
    expect(palierMax({ ...base, verdict: 'be' }).rReference).toBe(0)
    expect(palierMax({ ...base, verdict: 'expire' }).rReference).toBeNull()
  })
  it('pénalité straddle : les paliers TP coûtent la jambe morte (−1R)', () => {
    const s = { ...base, strategie: 'straddle', verdict: 'tp1+be' }
    expect(palierMax(s).rReference).toBeCloseTo(-0.4) // 0.6 − 1
    const s2 = { ...base, strategie: 'straddle', verdict: 'tp2+be' }
    expect(palierMax(s2).rReference).toBeCloseTo(1) // 2 − 1
  })
  it('SL straddle : pénalité NON appliquée (déjà −1R)', () => {
    const s = { ...base, strategie: 'straddle', verdict: 'sl' }
    expect(palierMax(s).rReference).toBe(-1)
  })
})

describe('labels et classes de palier', () => {
  it('labels canoniques', () => {
    expect(labelPalierMax('TP3')).toContain('TP3')
    expect(labelPalierMax('Expiré')).toContain('Expiré')
  })
  it('classes de couleur par palier', () => {
    expect(classePalierMax('TP3')).toBe('badge-green')
    expect(classePalierMax('SL')).toBe('badge-red')
    expect(classePalierMax('TP1')).toBe('badge-blue')
  })
})

describe('formateurs — jamais de « -0.0 » ni de « 0.0R » signé faux', () => {
  it('formatR (composable) : format compact sans espace, jamais « -0.0R »', () => {
    expect(formatR(4.71)).toBe('+4.71R')
    expect(formatR(-0.04)).toBe('-0.04R') // 2 décimales : le piège −0.0 n'existe pas ici
    expect(formatR(0)).toBe('0.00R')
    expect(formatR(-1.5)).toBe('-1.50R')
  })
  it('fmtR du rapport : +4.7 R / −1.0 R / 0.0 R', () => {
    expect(fmtR(4.71)).toBe('+4.7 R')
    expect(fmtR(0)).toBe('0.0 R')
    expect(fmtR(-1.04)).toBe('−1.0 R')
  })
  it('fmtDollars : groupage français (espace fine U+202F sous Node) et − typographique', () => {
    expect(fmtDollars(2093)).toBe('2\u202f093 $')
    expect(fmtDollars(-93)).toBe('−93 $')
  })
})
