/**
 * Lexique Straddle — même format que le lexique SMC (gabarit de référence
 * propriétaire : abréviation, nom, catégorie, définition, schéma SVG
 * optionnel). Design et affichage portés par LexiquePanel.
 */
export type Categorie = 'mecanique' | 'niveaux' | 'gestion' | 'timing'

export interface TermeStraddle { abrev: string; nom: string; cat: Categorie; def: string; svg?: string }

export const CAT_LABELS: Record<Categorie, { label: string; couleur: string }> = {
  mecanique: { label: 'Mécanique', couleur: 'text-amber-300 bg-amber-500/15' },
  niveaux:   { label: 'Niveaux',   couleur: 'text-blue-300 bg-blue-500/15' },
  gestion:   { label: 'Gestion',   couleur: 'text-emerald-300 bg-emerald-500/15' },
  timing:    { label: 'Timing',    couleur: 'text-purple-300 bg-purple-500/15' },
}

export const TERMES: TermeStraddle[] = [
  { abrev: 'Straddle', nom: 'Position à deux jambes opposées', cat: 'mecanique',
    def: "Stratégie d'annonce : à l'heure d'entrée E, deux positions symétriques (LONG et SHORT) sont ouvertes au même prix. Celle qui part dans le sens du mouvement devient la gagnante, l'autre paie son stop — le straddle attrape la direction sans la prédire." },
  { abrev: 'Jambe', nom: 'Une des deux positions du straddle', cat: 'mecanique',
    def: 'Le straddle vit en deux jambes parallèles : jambe LONG et jambe SHORT, ouvertes à E au même prix, chacune avec ses niveaux symétriques. La jambe gagnante est celle que le mouvement valide ; la jambe perdante voit son stop touché.' },
  { abrev: 'E', nom: 'Heure d\'entrée', cat: 'timing',
    def: "L'instant exact de l'ouverture des deux jambes — fixé à T-10 secondes de l'annonce (décision propriétaire du 24/08). C'est le TIMER qui décide de l'entrée, jamais le prix." },
  { abrev: 'T-10 s', nom: 'Dix secondes avant l\'annonce', cat: 'timing',
    def: "Marge d'armement : les deux jambes sont posées 10 secondes avant l'heure officielle de l'annonce pour éviter les premiers ticks de spread élargi." },
  { abrev: 'SL', nom: 'Stop-loss (−1R)', cat: 'niveaux',
    def: "Stop d'une jambe à 1R contre elle (R = 0,5 × ATR14). Particularité du straddle : le SL de la jambe perdante est exactement au niveau du TP1 de la jambe gagnante — quand la perdante paie, la gagnante vient de toucher TP1." },
  { abrev: 'TP1', nom: 'Premier objectif (+1R)', cat: 'niveaux',
    def: "Premier palier de la jambe gagnante à ±1R de l'entrée. Touché, le SL de la gagnante est resserré à un tampon sous E — le gain n'est pas encore verrouillé, mais la jambe survit au rebond typique." },
  { abrev: 'TP2', nom: 'Deuxième objectif (+2R)', cat: 'niveaux',
    def: "Palier à ±2R. Touché, le SL remonte à TP1 (gain verrouillé) et le trailing au tick démarre. Dans la comptabilité de référence, une passe « TP2 » vaut +1R net (2R du palier − 1R de la jambe perdante)." },
  { abrev: 'Tampon', nom: 'SL resserré à E ∓ 0,5R', cat: 'niveaux',
    def: "Après TP1, le stop de la gagnante ne va pas à l'entrée (BE sec) mais à 0,5R en deçà : le rebond typique d'ouverture/annonce qui touche E ne tue pas la jambe (décision du 27/08)." },
  { abrev: 'Trailing', nom: 'Suivi du prix au tick', cat: 'gestion',
    def: "À partir de TP2, le stop suit le meilleur prix atteint à distance de 1R, recalculé à chaque tick, jamais vers l'arrière. C'est ce qui transforme une extension en gains supplémentaires — ou rend la quasi-totalité de l'excédent si le mouvement meurt juste après TP2." },
  { abrev: 'BE', nom: 'Break-even (sortie à l\'entrée)', cat: 'gestion',
    def: "Sortie au prix d'entrée : 0R sur la jambe. Pour une passe, « BE » net = TP1 de la gagnante (+1R) moins le SL de la perdante (−1R) = 0R net." },
  { abrev: 'Time-stop', nom: 'Clôture horaire (60 min)', cat: 'gestion',
    def: "Une passe qui n'a rien décidé au bout de 60 minutes est refermée au prix courant : une annonce sans mouvement ne doit pas devenir un trade d'opinion." },
  { abrev: 'Passe', nom: 'Un cycle complet d\'annonce', cat: 'mecanique',
    def: "De l'armement à la clôture des deux jambes. Le R d'une passe = somme NETTE des deux jambes (gagnante + perdante). Les passes sont journalisées avec verdict et R pour la gate 3." },
  { abrev: 'Annonce HIGH', nom: 'Événement économique à fort impact', cat: 'timing',
    def: "Annonce US de tier 1 (NFP, CPI, PCE, FOMC…) qui arme un straddle. Le calendrier fournit l'heure E ; seules les annonces HIGH impact déclenchent des passes." },
]
