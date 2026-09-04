import { describe, it, expect } from 'vitest'
import {
  repartition, classement, lignesClassement, totalParts, decallage,
  couleurTf, couleurAsset, PALETTE,
} from '@/composables/useCamemberts'

describe('repartition — nombre de trades par catégorie', () => {
  it('compte et trie par effectif décroissant', () => {
    const trades = [
      { tf: 'M1' }, { tf: 'M1' }, { tf: 'M15' },
    ]
    const r = repartition(trades, t => t.tf)
    expect(r).toHaveLength(2)
    expect(r[0]).toMatchObject({ label: 'M1', n: 2 })
    expect(r[1].n).toBe(1)
  })

  it('parts en % sommant à 100', () => {
    const r = repartition([{ a: 'x' }, { a: 'x' }, { a: 'x' }, { a: 'y' }], t => t.a)
    expect(r[0].part).toBeCloseTo(75)
    expect(r[1].part).toBeCloseTo(25)
  })

  it('liste vide → une part à 0 % (pas de division par zéro)', () => {
    const r = repartition([], () => '')
    expect(r).toEqual([])
  })
})

describe('classement — Σ $ signés par catégorie', () => {
  it('trie par valeur décroissante, négatifs sans tranche', () => {
    const trades = [
      { asset: 'XAU', v: 100 }, { asset: 'XAU', v: 50 },
      { asset: 'BTC', v: -80 }, { asset: 'XAG', v: 30 },
    ]
    const c = classement(trades, t => t.asset, t => t.v)
    expect(c.map(x => x.label)).toEqual(['XAU', 'XAG', 'BTC'])
    expect(c[0].valeur).toBe(150)
    // part = 150 / (150+30) des gains totaux
    expect(c[0].part).toBeCloseTo(150 / 180 * 100)
    expect(c[2].part).toBe(0)
  })
})

describe('lignesClassement — légendes qui somment au centre', () => {
  it('au-delà de 4 catégories, une ligne « autres » absorbe les cachées', () => {
    const parts = [1, 2, 3, 4, 5, 6].map((v, i) => ({ label: `c${i}`, valeur: v, part: 0 }))
    const lignes = lignesClassement(parts, 4)
    expect(lignes).toHaveLength(5) // 4 affichées + autres
    expect(lignes[4].label).toBe('autres')
    expect(lignes[4].valeur).toBe(11) // 5 + 6
  })

  it('arrondi par plus grand reste : la somme des lignes = la somme exacte', () => {
    // 1.5×3 = 4.5 ; arrondis bruts 2+2+2 = 6 ; le plus grand reste rabogne à 5
    const parts = [1.5, 1.5, 1.5].map((v, i) => ({ label: `c${i}`, valeur: v, part: 0 }))
    const lignes = lignesClassement(parts, 4)
    const somme = lignes.reduce((s, l) => s + l.valeur, 0)
    expect(somme).toBe(5)
  })

  it('ligne « autres » à zéro filtrée', () => {
    const parts = [{ label: 'a', valeur: 10, part: 0 }, { label: 'b', valeur: 0, part: 0 }]
    const lignes = lignesClassement(parts, 1)
    expect(lignes.some(l => l.label === 'autres')).toBe(false)
  })
})

describe('totalParts / decallage — géométrie des donuts', () => {
  it('totalParts somme les effectifs', () => {
    expect(totalParts([{ label: 'a', n: 3, part: 60 }, { label: 'b', n: 2, part: 40 }])).toBe(5)
  })
  it('decallage cumule les parts précédentes', () => {
    const parts = [{ part: 10 }, { part: 20 }, { part: 30 }]
    expect(decallage(parts, 0)).toBe(0)
    expect(decallage(parts, 1)).toBeCloseTo(10)
    expect(decallage(parts, 2)).toBeCloseTo(30)
  })
})

describe('couleurs stables', () => {
  it('couleurTf déterministe par TF', () => {
    expect(couleurTf('M1')).toBe(couleurTf('M1'))
    expect(PALETTE).toContain(couleurTf('M15'))
  })
  it('couleurAsset déterministe et dans la palette', () => {
    expect(couleurAsset('XAUUSD')).toBe(couleurAsset('XAUUSD'))
    expect(PALETTE).toContain(couleurAsset('NAS100'))
  })
})
