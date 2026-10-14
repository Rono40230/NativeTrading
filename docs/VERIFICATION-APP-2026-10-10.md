# Vérification intégrale de l'app — RESTE À FAIRE

> Feuille allégée le **14/10/2026** : parcours 1-13 validés, écarts 1-19 et
> 23-30 clos, revue des 11 prompts terminée, nouveaux chantiers IA (briefing,
> autopsie, prompts presse) livrés et validés à l'écran, doctrine événements
> appliquée (structurels vs calendrier), créneaux génériques retirés.
> Historique complet : `git show HEAD:docs/VERIFICATION-APP-2026-10-10.md`
> ou les commits d'octobre. Récapitulatif 14/10 en bas de page.
>
> **Il ne reste QUE les décisions de clôture ci-dessous.**

## Clôture — décisions à prendre

1. **Commit/push de l'arbre** — 44 fichiers modifiés (+931 / −2 599), tout
   testé (123-126 verts par crate) et validé à l'écran :
   - écarts 23-30 (prompts 8→17, réactivation asset, tooltip conviction,
     page ML purgée, rapports, page Prompts par rôle, presse externalisée) ;
   - revue des 11 prompts (4 enrichis, 2 doctrine pure + injection, 3
     conformes, conviction gardée, écart 29 corrigé) ;
   - nouveaux analystes : 📋 Briefing du jour (cockpit) + 🧾 Autopsie de la
     semaine (analyses) ;
   - doctrine événements + retrait des créneaux génériques 8:30/10:00
     (Medium non tradés) ;
   - OTE fractal v2 (parqué D4) + docs.
   Découpage proposé : ① écarts+prompts+nouveautés IA · ② OTE v2 · ③ docs.
2. **D4 — OTE swing ≢ TV** : parqué (13/10). Reprise = captures côte à côte
   horodatées. Le code v2 est dans l'arbre — à commiter en l'état ou à
   réverter (au moment du commit).
3. **D3 — sort de la page heatmap orpheline** (ancien, non bloquant).

---

## Récapitulatif du chantier octobre (pour mémoire, tout est CLOS)

- **Parcours 1-13** : DÉMARRAGE, dashboard, 4 stratégies, graphiques,
  laboratoire, analyses & IA, presse & calendrier, données & santé,
  synchronisation/robustesse (27/27 réinscriptions 26 s, surcharge = ses
  couples seuls, vécu immuable), extinction/redémarrage (réglages et
  positions conservés bit à bit) — TOUT validé.
- **29 écarts corrigés** dont : matrice croisée straddle, rail unifié
  calendrier, tout-TP1, bougies MT5 réparées, réglages par asset,
  camemberts→dashboard, pages rapports 3 colonnes, prompts 8→17 classés par
  rôle (📖 définitions / 🔄 arrière-plan / 📊 analystes), réactivation asset,
  tooltip conviction CSS, purge boucle feedback ML morte (~1 500 lignes),
  blocs rapport empilés, définitions réellement injectées + system prompt
  neutre, paramètres moteur en direct dans les contextes.
- **Revue de fond des 11 prompts** : KTA (droit de dire rien, sobriété 1-3 à
  levier, chiffres obligatoires) ; conviction gardée en observation
  (AUC 0,477) — re-test après vécu des réglages par asset.
- **Nouveaux analystes IA** : 📋 Briefing du jour (badge cockpit : programme
  de déclenchement complet structurels + cal_* armés × annonces réelles,
  positions avec ±R flottant et $, presse FR 10 titres multi-thèmes,
  synthèse « ce qui compte » 3 lignes) · 🧾 Autopsie de la semaine
  (descriptif 7 jours, compteurs factuels, « semaine non concluante »).
- **Doctrine événements (14/10)** : structurels = catalogue × matrice ;
  annonces = calendrier High SEUL (Medium non tradés) ; créneaux génériques
  8:30/10:00 retirés (taxonomie + matrice + mécanique gate morte) ;
  matrice = 17 événements (8 structurels + 9 types 📅), 68/68 cases armées.
- **Incidents clos** : migration 122 orpheline (app morte → réparée +
  REINDEX bougies), bougies MT5 tronquées, OTE fractal v2 parqué.

> Rappel : **décrire avant de corriger** — chaque écart passe par écrit
> avant toute modification. Les correctifs sont validés un à un.
