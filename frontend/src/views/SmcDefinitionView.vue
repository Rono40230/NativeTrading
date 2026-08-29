<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">

    <!-- En-tête : identité + état du registre -->
    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">📐 SMC</h1>
      <span class="text-gray-500 text-base hidden sm:inline">Smart Money Concepts — indicateur v12</span>
      <span
        v-if="reglages"
        class="ml-auto text-[11px] font-semibold px-2.5 py-1 rounded-full border"
        :class="badgeClasse"
      >{{ reglages.etat }}</span>
    </div>

    <!-- Barre d'onglets (décision étape 3 : Définition / Décision d'entrée /
         Gestion des trades ouverts / Money management / Lexique en onglet) -->
    <div class="flex gap-1 border-b border-white/10 shrink-0">
      <button
        v-for="t in onglets"
        :key="t"
        class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px"
        :class="onglet === t ? 'text-white border-blue-400' : 'text-gray-400 border-transparent hover:text-white/70'"
        @click="onglet = t"
      >{{ t }}</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1">

      <!-- ═══ ONGLET DÉFINITION ═══ -->
      <div v-if="onglet === 'Définition'" class="flex flex-col gap-3">
        <carte titre="Concept">
          SMC trade les mouvements de l'argent institutionnel : la structure de marché révèle
          l'intention (accumulation → manipulation → distribution), les zones où les ordres
          institutionnels sont posés (Order Blocks) servent de points d'entrée au retest, et les
          liquidités (sommets/creux apparents, niveaux de la veille et de la semaine) sont chassées
          avant les impulsions. La stratégie v12 clone l'indicateur Pine éponyme, barre confirmée
          après barre confirmée, avec évaluation intrabar pour les annonces.
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="La structure de marché">
            Les pivots (swing length calibré par actif) dessinent la succession sommets/creux :
            HH+HL = tendance haussière, LH+LL = baissière. Le BOS (break of structure) confirme la
            continuation ; le MSS/CHOCH (changement de caractère) annonce le retournement. Le sens
            de la structure cadre toute décision — tendance indécise = aucun signal.
          </carte>
          <carte titre="Les liquidités et la chasse">
            PDH/PDL (plus haut/bas de la veille), PWH/PWL (semaine), EQH/EQL (épaules égales) sont
            les cibles visibles. Le sweep — mèche qui perce un niveau puis referme — signale la
            manipulation : c'est le carburant des entrées en contre-attaque. Tout niveau touché sur
            barre confirmée (sweep ou cassure) est consommé et disparaît.
          </carte>
          <carte titre="Les zones d'entrée">
            Order Blocks (dernière bougie avant l'impulsion, lifecycle en 3 états), FVG/imbalance
            (gap de prix non distribué), IFVG (imbalance inversée par BOS), Breaker (OB cassé
            devenant résistance/support). La zone cœur croise OB ∩ OTE ∩ FVG — la zone
            institutionnelle la plus dense.
          </carte>
          <carte titre="Le contexte">
            Premium/Discount (moitié haute/basse du range), OTE (retrace 62-79 % de l'impulsion,
            zone d'entrée optimale), Kill Zones de Londres (7h-10h UTC) et New York (13h-16h UTC),
            Asian High/Low (session de Paris), NDOG/NWOG (gaps d'ouverture journalière/hebdo) et
            confluences multi-timeframe (H1/H4/W1/MN amorcées sur l'historique, comme TradingView).
          </carte>
        </div>

        <carte titre="Les deux moteurs de signaux">
          <div class="flex flex-col gap-2">
            <p><b class="text-white">Moteur v11-OB</b> — Order Blocks : score enrichi (fraîcheur +
            proximité) par zone, barème calibré par actif et timeframe. Un OB signalé ne re-signalera
            jamais (règle de l'un-signal : le premier OB signalé du carnet est écarté).</p>
            <p><b class="text-white">Moteur BSZones</b> — parcours Sweep → Dispersion → OB : zones
            reconstituées après la chasse de liquidité, score propre, seuil dédié.</p>
            <p class="text-gray-400">Un seul trade est créé par barre confirmée, tous moteurs confondus.</p>
          </div>
        </carte>
      </div>

      <!-- ═══ ONGLET DÉCISION D'ENTREE ═══ -->
      <div v-if="onglet === 'Décision d\u2019entrée'" class="flex flex-col gap-3">
        <carte titre="Le parcours d'un signal">
          <ol class="list-decimal ml-5 space-y-1.5">
            <li>Le prix revient au-dessus d'un Order Block haussier (ou sous un OB baissier), zone non signalée, état non profond, à moins de 8×ATR.</li>
            <li>Le score de la zone atteint le seuil de trade et la force minimale (4/10, i_forceMin v11).</li>
            <li>La qualité de zone est validée (zone cœur / confluences MTF / Fibonacci OTE selon barème).</li>
            <li>La porte de trade unique est ouverte — sinon, rien (voir règle ci-dessous).</li>
            <li>Annonce d'imminence immédiate sur Telegram (intrabar, dès la qualification), confirmation du trade à la clôture de la barre (barstate.isconfirmed).</li>
          </ol>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="La porte de trade unique (f_tradeBloquant)">
            Interdiction d'ouvrir un nouveau trade tant qu'un trade existant est rempli et n'a pas
            atteint TP1 (ou son break-even). Complétée par : un seul trade poussé par barre confirmée,
            et la règle de l'un-signal sur les zones. Un ordre en attente (non rempli) ne bloque pas —
            comme sur le graphique TradingView.
          </carte>
          <carte titre="Les niveaux consommés">
            Tout niveau de liquidité touché sur barre confirmée — par sweep (mèche qui perce et
            referme) ou par cassure franche — est consommé : il disparaît du carnet et ne peut plus
            servir de cible ni de confluence. Décision de la passe « décisions trading ».
          </carte>
        </div>

        <carte titre="Exécution — retest limite">
          L'entrée est placée au bord de la zone (order block haut pour un achat, bas pour une vente),
          en ordre limite : le trade n'est REMPLI que si le prix revient toucher l'entrée. Ce modèle
          « Retest (limite) » a gagné l'A/B 15/15 contre l'entrée au marché à la cassure — il est
          figé dans la v12.
        </carte>
      </div>

      <!-- ═══ ONGLET GESTION DES TRADES OUVERTS ═══ -->
      <div v-if="onglet === 'Gestion des trades ouverts'" class="flex flex-col gap-3">
        <carte titre="Construction des niveaux">
          <ul class="space-y-1.5">
            <li><b class="text-white">Stop Loss</b> — bord opposé de la zone ± offset ATR réduit de 25 % (décision étape 4 du 29/08 : replay +239R), distance clampée entre slMin et slMax (multiplicateurs ×ATR calibrés par actif : BTC 0,8-2,5, or 0,5-1,5, NAS/DAX 0,5-1,5, argent 0,6-1,8).</li>
            <li><b class="text-white">TP1 / TP2</b> — +0,6R / +2R (décision étape 4 du 29/08 : TP1 à 0,6R, replay +239R ; R = distance entrée-stop après clamp).</li>
            <li><b class="text-white">TP3</b> — la liquidité la plus proche au-delà de l'entrée (EQH/PDH/PWH/Asian High pour un achat) ; repli sur +3R si aucune cible ou monotonie brisée.</li>
          </ul>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Le cycle de vie">
            <ol class="list-decimal ml-5 space-y-1.5">
              <li><b>En attente</b> — ordre limite au bord de la zone, pas encore touché.</li>
              <li><b>Rempli</b> — le prix est revenu toucher l'entrée (fill réel au retest).</li>
              <li><b>TP1 touché</b> — le stop remonte à l'entrée : le trade est neutralisé (break-even garanti).</li>
              <li><b>TP2 armé</b> — si le prix repasse sous TP1, sortie à break-even (le TP2 est encaissé).</li>
              <li><b>TP3 touché</b> — clôture complète.</li>
            </ol>
          </carte>
          <carte titre="Sorties anticipées et expiration">
            <ul class="space-y-1.5">
              <li><b class="text-white">SL</b> — stop touché avant TP1 : -1R.</li>
              <li><b class="text-white">BE forcé</b> — BOS opposé pendant le trade : stop ramené à l'entrée, même sans TP1.</li>
              <li><b class="text-white">Annulation</b> — ordre en attente + BOS opposé : l'ordre est retiré.</li>
              <li><b class="text-white">Expiration</b> — âge du trade > 4h en intraday (8h en H1, 32h en H4, 4 jours en D1), ou TP3 non atteint dans le délai après TP2.</li>
            </ul>
          </carte>
        </div>

        <carte titre="Verdicts">
          Chaque trade clôturé reçoit son verdict — TP1, TP2, TP3, SL, BE (break-even) ou Expire —
          écrit en base avec le prix de sortie et le R réel. C'est cette historisation qui alimente
          la courbe de trades du dashboard. Aucun message de clôture sur Telegram : seule
          l'imminence parle.
        </carte>
      </div>

      <!-- ═══ ONGLET MONEY MANAGEMENT ═══ -->
      <div v-if="onglet === 'Money management'" class="flex flex-col gap-3">
        <carte titre="Les trois couches (décision étape 2)">
          <div class="flex flex-col gap-2">
            <p><b class="text-white">1. Conventions par actif</b> (onglet gestion du risque) — taille
            du pip, valeur du pip, lot min/max : la grammaire commune de tous les calculs.</p>
            <p><b class="text-white">2. Allocation par stratégie</b> — capital dédié et risque de 1
            à 3 % par trade, réglés dans Paramètres › stratégies › SMC.</p>
            <p><b class="text-white">3. Calcul à l'émission</b> — lot = (capital × risque) ÷ (stop
            en pips × valeur du pip). Calculé au moment du signal, jamais avant.</p>
          </div>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <valeur etiquette="Capital alloué" :valeur="reglageStr('capital')" />
          <valeur etiquette="Risque par trade" :valeur="reglageStr('risque')" />
          <valeur etiquette="1R représente" :valeur="reglageStr('unR')" />
        </div>

        <carte titre="Le R, unité de compte">
          Tous les trades se mesurent en multiples du risque initial (R) : SL = -1R, BE = 0,
          TP1 = +0,6R, TP2 = +2R, TP3 = sa distance réelle (décision étape 4 du 29/08). La performance de la stratégie se lit en
          R cumulé — indépendante du capital et homogène entre actifs — et se convertit en
          évolution du capital via le risque par trade.
        </carte>
      </div>

      <!-- ═══ ONGLET LEXIQUE ═══ -->
      <LexiquePanel v-if="onglet === 'Lexique'" />

      <!-- ═══ ONGLET ENRICHISSEMENT IA ═══ -->
      <div v-if="onglet === 'Enrichissement IA'" class="flex flex-col gap-3">
        <carte titre="Rôle de l'IA dans la stratégie">
          <div class="flex flex-col gap-2">
            <p><b class="text-white">Analyse stratégique</b> (active aujourd'hui — bouton
            « Analyse SMC » du graphique) : lit les signaux clôturés, évalue la performance
            par type de confluence et par contexte, et produit des recommandations lisibles.</p>
            <p><b class="text-white">Rôles à cadrer à l'étape 6</b> : filtre temps réel et
            monitoring par stratégie — le cahier des charges sera discuté et acté avant tout
            branchement.</p>
          </div>
        </carte>
        <carte titre="Fonctionnement">
          L'IA tourne <b>en local</b> (Ollama). Elle reçoit la définition canonique de la
          stratégie — dérivée de cette page, source unique — et l'historique des trades
          clôturés. Elle intervient <b>hors du temps réel</b> : sur demande pour l'analyse,
          jamais dans la boucle de décision d'un signal. Les textes des prompts se règlent
          dans Outils IA › Prompts IA.
        </carte>
        <carte titre="Objectifs">
          <ul class="space-y-1.5">
            <li>Expliquer la performance : quelles confluences gagnent, sur quels actifs, quelles plages horaires.</li>
            <li>Détecter les dérives du moteur par rapport à sa définition (signaux hors définition, contextes perdants récurrents).</li>
            <li>À l'étape 6 : évaluer la pertinence d'un filtre temps réel — seulement si la preuve le justifie.</li>
          </ul>
        </carte>
        <carte titre="Garde-fous">
          <b>L'IA n'ouvre jamais de trade.</b> Le moteur v12 applique la définition figée —
          l'étalon est le Pine. L'IA conseille et explique ; toute modification de réglage
          est un acte du propriétaire dans les Paramètres. Aucune autonomie sur les seuils,
          les signaux ou l'exécution.
        </carte>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, defineComponent, h, onMounted } from 'vue'
import { http } from '@/services/http.client'
import LexiquePanel from '@/components/common/LexiquePanel.vue'

interface ReglagesStrategie {
  etat: string; capital: number; risque_pct: number
}

// ── Mini-composants locaux (carte titrée + vignette de valeur) ───────────────
const Carte = defineComponent({
  props: { titre: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-5 py-4' }, [
      h('div', {
        class: 'text-xs font-semibold text-blue-400 uppercase tracking-widest mb-2.5',
        innerHTML: props.titre,
      }),
      h('div', {
        class: 'text-gray-300 text-sm leading-relaxed [&_b]:text-white [&_ol]:list-decimal [&_ol]:ml-5 [&_ul]:space-y-1 [&_p]:mb-2 [&_p:last-child]:mb-0',
      }, slots.default?.()),
    ])
  },
})
const carte = Carte

const Valeur = defineComponent({
  props: {
    etiquette: { type: String, required: true },
    valeur: { type: String, required: true },
  },
  setup: (p: { etiquette: string; valeur: string }) => () =>
    h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-4 py-3' }, [
      h('div', { class: 'text-[10px] text-gray-500 uppercase tracking-widest' }, p.etiquette),
      h('div', { class: 'text-lg font-bold text-white mt-1 font-mono' }, p.valeur),
    ]),
})
const valeur = Valeur

// ── Onglets (décision étape 3 : Définition première page + Lexique en onglet)
const onglets = ['Définition', 'Décision d\u2019entrée', 'Gestion des trades ouverts', 'Money management', 'Lexique', 'Enrichissement IA'] as const
const onglet = ref<(typeof onglets)[number]>('Définition')

// ── Réglages live de la stratégie (registre) ─────────────────────────────────
const reglages = ref<ReglagesStrategie | null>(null)
onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    const smc = (res.data as { id: string; etat: string; capital: number; risque_pct: number }[])
      .find(s => s.id === 'SMC')
    if (smc) reglages.value = smc
  } catch { /* registre indisponible — badge masqué */ }
})

const badgeClasse = computed(() =>
  reglages.value?.etat === 'Officielle'
    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
    : 'bg-amber-500/10 text-amber-400 border-amber-500/30')

function reglageStr(champ: 'capital' | 'risque' | 'unR'): string {
  const r = reglages.value
  if (!r) return '—'
  if (champ === 'capital') return r.capital > 0 ? `${r.capital.toLocaleString('fr-FR')} $` : 'à renseigner'
  if (champ === 'risque') return `${r.risque_pct} %`
  return r.capital > 0 ? `${(r.capital * r.risque_pct / 100).toLocaleString('fr-FR')} $` : '—'
}
</script>

<script lang="ts">
export default { name: 'SmcDefinitionView' }
</script>
