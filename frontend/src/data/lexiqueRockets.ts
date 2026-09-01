/**
 * Lexique Rockets — même format que le lexique SMC (gabarit de référence
 * propriétaire). Design et affichage portés par LexiquePanel.
 */
export type Categorie = 'structure' | 'classement' | 'detection' | 'gestion' | 'univers'

export interface TermeRockets { abrev: string; nom: string; cat: Categorie; def: string; svg?: string }

export const CAT_LABELS: Record<Categorie, { label: string; couleur: string }> = {
  structure: { label: 'Structure',  couleur: 'text-blue-300 bg-blue-500/15' },
  classement:{ label: 'Classement', couleur: 'text-amber-300 bg-amber-500/15' },
  detection: { label: 'Détection',  couleur: 'text-red-300 bg-red-500/15' },
  gestion:   { label: 'Gestion',    couleur: 'text-emerald-300 bg-emerald-300/15' },
  univers:   { label: 'Univers',    couleur: 'text-purple-300 bg-purple-500/15' },
}

export const TERMES: TermeRockets[] = [
  { abrev: 'VCP', nom: 'Volatility Contraction Pattern', cat: 'structure',
    def: "Le setup signature de Minervini : une consolidation où chaque repli successif est moins profond que le précédent (ex. −25 %, −15 %, −8 %, −3 %). L'offre s'épuise, la base se resserre — l'explosion suit." },
  { abrev: 'Base', nom: 'Zone de consolidation', cat: 'structure',
    def: 'La période latérale pendant laquelle le titre digère son mouvement précédent. Une bonne base est travaillée (durée), serrée (contractions) et sèche en volume.' },
  { abrev: 'Contraction', nom: 'Repli au sein de la base', cat: 'structure',
    def: "Chaque creux intermédiaire de la base. Le classement exige au moins 2 contractions de profondeur décroissante — c'est le cœur du VCP." },
  { abrev: 'Pivot', nom: "Point d'entrée (plus haut des 60 jours)", cat: 'structure',
    def: "Le plus haut des 60 séances précédant la bougie de cassure. C'est le niveau d'achat : le signal ne naît qu'au franchissement décisif de ce pivot." },
  { abrev: 'Cassure', nom: 'Franchissement décisif du pivot', cat: 'detection',
    def: "Clôture D1 à +3 % minimum au-delà du pivot, en bougie Marubozu, sur volume ≥ 150 % de la MM50. En dessous de ces exigences, ce n'est pas une cassure." },
  { abrev: 'Marubozu', nom: 'Bougie à corps plein', cat: 'detection',
    def: 'Bougie dont la mèche haute fait ≤ 25 % de l\'étendue totale : les acheteurs contrôlent la séance de bout en bout. Exigée à la cassure — une bougie à longue mèche au-dessus du pivot signale une distribution.' },
  { abrev: 'RS', nom: 'Force relative (O\'Neil)', cat: 'classement',
    def: "Performance du titre contre son marché de référence sur 4 semaines : BTC pour la crypto, QQQ pour les actions. Le point sentiment exige de battre la référence — c'est le signe du leadership." },
  { abrev: 'Trend template', nom: 'Pré-screen Minervini (8 conditions)', cat: 'classement',
    def: "Filtre d'ENTRÉE dans le périmètre actions (pas un point du /10) : prix > MM50 > MM150 > MM200, MM200 en hausse sur un mois, à ≤ 25 % du plus haut 52 semaines, ≥ 30 % au-dessus du bas, surperformance de la référence." },
  { abrev: 'Classement /10', nom: 'Le score du scanner', cat: 'classement',
    def: "Dix critères (sentiment, contexte, news, tendance, volatilité, intérêt, figure, gaps, cassure, liquidité) — un point chacun, seuil de candidature à 5, seuil de signal à 7. Même barème crypto et actions." },
  { abrev: 'News catalyseur', nom: 'Le point news (1/10)', cat: 'classement',
    def: "L'analyste IA lit les dépêches du candidat : un catalyseur identifié (résultats, contrat, homologation, listing) vaut le point — POUR/CONTRE/NEUTRE avec conviction. Il n'est ni un déclencheur ni un veto : la cassure décide seule." },
  { abrev: 'Ranker', nom: 'Seconde opinion IA à la cassure', cat: 'gestion',
    def: "Au moment d'une cassure, l'analyste note la conviction (0-100) que ce pivot est VRAI — il traque les faux signaux (volume sans corps, fin de tendance, news contraire). Sous 40/100, le signal est écarté." },
  { abrev: 'R1', nom: 'Premier objectif (+1R)', cat: 'gestion',
    def: "À R1 atteint : 50 % de la position est vendue et un trailing % prend le relais. Le risque initial est annulé par la partielle." },
  { abrev: 'Invalidation', nom: 'Stop du setup', cat: 'gestion',
    def: "Niveau de rejet du candidat (structure de la base). Si le prix le touche avant la cassure, le setup est éliminé." },
  { abrev: 'Éliminé', nom: 'Sortie du suivi', cat: 'gestion',
    def: "Un candidat non re-confirmé par 2 scans consécutifs (ou invalidé) est marqué éliminé : la ligne reste visible dans le Scanner avec sa date, grisée — l'historique des setups se conserve." },
  { abrev: 'Univers', nom: 'Crypto + Actions US', cat: 'univers',
    def: "Crypto : top 300 Binance en volume. Actions US : répertoire NASDAQ Trader (~5 667 titres communs), prix Tiingo en volume réel, référence QQQ — même classement /10 des deux côtés." },
  { abrev: 'Observation', nom: 'État silencieux de la verticale', cat: 'univers',
    def: "Les détections sont journalisées (candidats, cassures, scores) mais AUCUN signal n'est publié ni exécuté — jusqu'à décision du propriétaire sur preuve. Les actions US vivent en Observation depuis le 01/09." },
  { abrev: 'Earnings', nom: 'Résultats trimestriels (badge 📊)', cat: 'univers',
    def: "Date de prochains résultats extraite des dépêches si mentionnée : affichée en badge orange sur le candidat (risque de gap). Avertissement seulement — pas de veto (décision v1 du 31/08)." },
]
