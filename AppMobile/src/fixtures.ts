/**
 * Fixtures de démonstration — ?demo=1 dans l'URL (affordance dev,
 * documentée) : permettent de valider l'écran SANS relance du backend et
 * SANS attendre de vraies alertes. Aucun appel réseau n'est fait.
 */
import type { Alerte, AlerteComplete, Aujourdhui } from './types'

const min = 60

export const demoJournal: Alerte[] = [
  { id: 5, ts: Date.now() / 1000 - 3 * min, type: 'smc_approche', titre: 'Approche zone ACHAT · XAUUSD M15', asset: 'XAUUSD', tf: 'M15', png: true },
  { id: 4, ts: Date.now() / 1000 - 25 * min, type: 'digest', titre: 'Briefing du matin 17/10', asset: null, tf: null, png: false },
  { id: 3, ts: Date.now() / 1000 - 2 * 3600, type: 'evenement', titre: 'T-10 min · CPI US 14:30', asset: null, tf: null, png: false },
  { id: 2, ts: Date.now() / 1000 - 5 * 3600, type: 'presse', titre: 'La Fed ouvre la porte à une baisse de taux', asset: null, tf: null, png: false },
  { id: 1, ts: Date.now() / 1000 - 9 * 3600, type: 'sante', titre: 'BTC H4 — retard 10h20', asset: 'BTC', tf: 'H4', png: false },
]

export const demoAujourdhui: Aujourdhui = {
  date_paris: '17/10/2026',
  annonces: [
    { ts: Date.now() / 1000 + 90 * min, heure_paris: '14:30', devise: 'USD', titre: 'CPI m/m' },
    { ts: Date.now() / 1000 + 4 * 3600, heure_paris: '18:00', devise: 'USD', titre: 'FOMC — Minutes' },
  ],
  presse: [
    { titre: 'La Fed ouvre la porte à une baisse de taux', score: 100, theme: 'macro' },
    { titre: 'L\'or franchit un sommet historique', score: 100, theme: 'metaux' },
  ],
  nuit: [
    { ts: Date.now() / 1000 - 11 * 3600, type: 'smc_contact', titre: '🎯 Contact zone VENTE · NAS100 M15', asset: 'NAS100', tf: 'M15' },
  ],
  synthese: {
    resume: 'Journée CPI : volatilité attendue à 14h30 sur USD et or. Les zones M15 XAU restent la priorité.',
    ce_qui_compte: ['CPI 14:30 — XAUUSD/NAS100 armés', 'Zone ACHAT XAU M15 à 0,4 ATR'],
    confiance: 80,
  },
}

export function demoAlerte(id: number): AlerteComplete {
  const base = demoJournal.find(a => a.id === id) ?? demoJournal[0]
  const payloads: Record<number, AlerteComplete['payload']> = {
    5: { achat: true, zone_haut: 2665.4, zone_bas: 2658.2, distance_atr: 0.42, prix: 2661.1 },
    4: { annonces: demoAujourdhui.annonces, presse: demoAujourdhui.presse, synthese: demoAujourdhui.synthese, nuit: demoAujourdhui.nuit, date_paris: demoAujourdhui.date_paris },
    3: { heure_paris: '14:30', rappel_min: 10, devise: 'USD' },
    2: { score: 100 },
    1: { retard_min: 620, seuil_min: 540, marche_ouvert: true },
  }
  return { ...base, payload: payloads[base.id] ?? {} }
}

/// Brief presse démo (markdown LLM) — miroir de la Revue de presse de
/// l'app principale pour le bouton « ⚡ Générer ».
export const demoBriefPresse = {
  id: 99,
  nb_articles: 15,
  contenu: [
    '**Synthèse des dernières 24 h**',
    '',
    'Le ton macro reste dominé par la perspective d\'un assouplissement de la Fed : deux membres au FOMC ont évoqué une baisse de taux « d\'ici la fin de l\'année », ce qui pousse l\'or à de nouveaux sommets historiques.',
    '',
    '**À surveiller aujourd\'hui**',
    '- CPI US à 14h30 (USD, or) — le point culminant de la journée ;',
    '- Pétrole : l\'OPEP+ maintient ses quotas, le Brent reste sous tension ;',
    '- Crypto : flux institutionnels positifs sur BTC, momentum intact sur H4.',
    '',
    '**Risque principal** : un CPI supérieur aux attentes retarderait le scénario de baisse de taux et déclencherait un repli rapide de l\'or et des indices.',
  ].join('\n'),
}
