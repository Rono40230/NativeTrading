import { describe, it, expect } from 'vitest'
import { palierMax, labelPalierMax, formatR, classePalierMax, formatDuree } from '@/composables/useSignalFormat'
import { fmtDollars, fmtR } from '@/composables/useAnalyses'

const base = {
  strategie: 'SMC',
  direction: 'Long',
  prix_entree: 100,
  stop_loss: 99, // risque 1
  take_profit: [100.6, 102, 103],
}

describe('palierMax — libellé du palier (le R officiel est servi backend)', () => {
  // Harmonisation 15/09 : la conversion verdict → R-distance vit UNIQUEMENT
  // côté backend (r_reference_palier, testé dans crates/db) et arrive dans
  // Signal.r_distance. palierMax ne fournit plus que le LIBELLÉ du palier.
  it('SL / TP1 / TP2 / TP3 / BE / Expire', () => {
    expect(palierMax({ ...base, verdict: 'sl' }).palier).toBe('SL')
    expect(palierMax({ ...base, verdict: 'tp1+be' }).palier).toBe('TP1')
    expect(palierMax({ ...base, verdict: 'tp2+be' }).palier).toBe('TP2')
    expect(palierMax({ ...base, verdict: 'tp3' }).palier).toBe('TP3')
    expect(palierMax({ ...base, verdict: 'be' }).palier).toBe('BE')
    expect(palierMax({ ...base, verdict: 'expire' }).palier).toBe('Expiré')
  })
  it('verdict inconnu → null', () => {
    expect(palierMax({ ...base, verdict: null }).palier).toBeNull()
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

describe('formatDuree — vie de la position (remplissage → fermeture)', () => {
  const T = 1_788_558_780 // 04/09 23:52:30 — remplissage du BTC du 04/09
  it('31 secondes (le BTC mort au SL une minute après son remplissage)', () => {
    expect(formatDuree(T, T + 31)).toBe('31 s')
  })
  it('minutes + secondes, puis minutes pleines', () => {
    expect(formatDuree(T, T + 150)).toBe('2 mn 30 s')
    expect(formatDuree(T, T + 120)).toBe('2 mn')
  })
  it('heures + minutes, puis heures pleines ; jours + heures', () => {
    expect(formatDuree(T, T + 2 * 3600 + 13 * 60)).toBe('2 h 13 mn')
    expect(formatDuree(T, T + 2 * 3600)).toBe('2 h')
    expect(formatDuree(T, T + 27 * 3600)).toBe('1 j 3 h')
  })
  it('non rempli (null/undefined) ou non fermé → tiret', () => {
    expect(formatDuree(null, T + 31)).toBe('—')
    expect(formatDuree(undefined, T + 31)).toBe('—')
    expect(formatDuree(T, null)).toBe('—')
  })
  it('fin avant début (horloge bosselée) → 0 s, jamais négatif', () => {
    expect(formatDuree(T, T - 5)).toBe('0 s')
  })
})
