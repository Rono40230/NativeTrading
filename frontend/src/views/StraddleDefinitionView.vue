<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">

    <!-- En-tête : identité + état du registre -->
    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">⚡ Straddle</h1>
      <span class="text-white text-base hidden sm:inline">Volatilité événementielle — ordre miroir deux jambes</span>
      <span
        v-if="reglages"
        class="ml-auto text-[11px] font-semibold px-2.5 py-1 rounded-full border"
        :class="badgeClasse"
      >{{ reglages.etat }}</span>
    </div>

    <!-- Onglets (gabarit vertical : 4 sections, pas de lexique) -->
    <div class="flex gap-1 border-b border-white/10 shrink-0">
      <button
        v-for="t in onglets"
        :key="t"
        class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px"
        :class="onglet === t ? 'text-white border-amber-400' : 'text-white border-transparent hover:text-white/70'"
        @click="onglet = t"
      >{{ t }}</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1">

      <!-- ═══ DÉFINITION ═══ -->
      <div v-if="onglet === 'Définition'" class="flex flex-col gap-3">
        <carte titre="Concept">
          Le straddle capture les mouvements de volatilité VIOLENTS ET BREFS déclenchés par les
          événements de marché : autour d'une annonce forte, deux ordres miroir attendent la
          cassure — la jambe du bon côté entre, quelle que soit la direction.
          L'autre est annulée. Pas d'anticipation directionnelle : la stratégie achète la
          volatilité elle-même.
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Le déclencheur">
            Périmètre acté : XAU, BTC, NAS100 et SP500 sur les <b>annonces US fortes</b>
            (impact High du calendrier économique) ; DAX sur <b>l'ouverture européenne</b>
            (9h00 Paris). XAU et BTC sont armés en temps réel ; NAS100, SP500 et DAX
            attendent le branchement MT5 (phase 5).
          </carte>
          <carte titre="La fenêtre">
            À T-30 minutes, le moteur entre en préparation : observation du range et
            chauffage de l'ATR14. À T-10 secondes (réglable), les deux jambes sont posées
            au prix courant E et armées immédiatement — le premier franchissement de E,
            même avant l'annonce, remplit la jambe dans son sens.
          </carte>
        </div>
      </div>

      <!-- ═══ DÉCISION D'ENTRÉE ═══ -->
      <div v-if="onglet === 'Décision d\u2019entrée'" class="flex flex-col gap-3">
        <carte titre="L'ordre miroir">
          <ul class="space-y-1.5">
            <li>Les deux jambes entrent au <b>même prix E</b> : prix courant à T-10 secondes.</li>
            <li>Buy-stop à E : se remplit si le prix passe au-dessus.</li>
            <li>Sell-stop à E : se remplit si le prix passe en dessous.</li>
            <li><b>OCO</b> : la première jambe remplie annule l'autre instantanément.</li>
          </ul>
        </carte>
        <carte titre="Le risque R">
          R, l'unité de risque, vaut la distance du stop : <b>R = facteur SL × ATR14</b>
          (réglable dans la carte paramètres — le lot en découle : 1R = capital × risque).
          Tous les niveaux de la stratégie s'expriment en R : SL à ±1R, TP1 à 1R, TP2 à 2R.
        </carte>
        <carte titre="Expiration">
          Si aucun francissement ne survient, les deux jambes expirent 30 minutes après
          l'annonce — la volatilité attendue n'est pas venue, on repart sans position.
        </carte>
      </div>

      <!-- ═══ GESTION DES TRADES OUVERTS ═══ -->
      <div v-if="onglet === 'Gestion des trades ouverts'" class="flex flex-col gap-3">
        <carte titre="Le cycle de la jambe survivante">
          <ol class="list-decimal ml-5 space-y-1.5">
            <li><b>Remplissage</b> — la jambe est entrée à E, stop à E ∓ 1R.</li>
            <li><b>TP1 = 1R touché</b> — le stop remonte à l'entrée : break-even garanti.</li>
            <li><b>TP2 = 2R touché</b> — le stop remonte à TP1 et le <b>trailing stop démarre</b>.</li>
            <li><b>Trailing</b> — le stop suit le prix <b>au tick</b>, à distance réglable
            (en ×R), jamais vers l'arrière : il verrouille le mouvement violent pendant
            qu'il dure.</li>
            <li><b>Sortie</b> — sur le trailing touché (TS), sur retour au stop (BE/SL),
            ou au time-stop de 60 minutes après le remplissage (TimeStop, au prix courant).</li>
          </ol>
        </carte>
        <carte titre="Verdicts">
          Chaque passe clôturée reçoit son verdict et son R réel : SL (-1R), BE (0R),
          TS (trailing, R variable &gt; 1R), TimeStop (R au prix de sortie). Ils alimentent
          la courbe de trades du bloc Straddle. En Observation : journalisé, silencieux
          sur Telegram.
        </carte>
      </div>

      <!-- ═══ MONEY MANAGEMENT ═══ -->
      <div v-if="onglet === 'Money management'" class="flex flex-col gap-3">
        <carte titre="Les trois couches (communes aux stratégies)">
          <div class="flex flex-col gap-2">
            <p><b class="text-white">1. Conventions par actif</b> (onglet gestion du risque) — taille et valeur du pip, lot min/max.</p>
            <p><b class="text-white">2. Allocation par stratégie</b> — capital dédié et risque 1 à 3 % par passe, dans Paramètres › stratégies › Straddle.</p>
            <p><b class="text-white">3. Calcul au signal</b> — lot = (capital × risque) ÷ (stop en pips × valeur du pip), stop = 1R = facteur SL × ATR14.</p>
          </div>
        </carte>
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <valeur etiquette="Capital alloué" :valeur="reglageStr('capital')" />
          <valeur etiquette="Risque par passe" :valeur="reglageStr('risque')" />
          <valeur etiquette="1R représente" :valeur="reglageStr('unR')" />
        </div>
        <carte titre="Une passe à la fois">
          Le moteur ne gère qu'une position par annonce et par actif — jamais de
          cumul : la volatilité événementielle se joue un événement à la fois.
        </carte>
      </div>

      <!-- ═══ ENRICHISSEMENT IA ═══ -->
      <div v-if="onglet === 'Enrichissement IA'" class="flex flex-col gap-3">
        <carte titre="Rôle de l'IA dans la stratégie">
          <div class="flex flex-col gap-2">
            <p><b class="text-white">1. Repérer les événements qui font bouger chaque actif</b> —
            au-delà du simple calendrier « impact fort » : mesurer quelles annonces déplacent
            réellement XAU, BTC, NAS100, SP500, et construire l'agenda pertinent par actif.</p>
            <p><b class="text-white">2. Affiner l'heure d'entrée</b> — le T-10 s par défaut est
            réglable : l'IA proposera un minutage par type d'événement et par actif, sur
            preuve historique.</p>
            <p><b class="text-white">3. Ouvert</b> — d'autres usages seront définis par le
            propriétaire au fil des observations.</p>
          </div>
        </carte>
        <carte titre="Fonctionnement">
          L'IA tourne <b>en local</b> (Ollama), hors du temps réel. Sa matière première :
          les <b>passes journalisées</b> — chaque annonce, son range, son remplissage, son
          verdict en R (l'Observation les enregistre dès maintenant). C'est en accumulant
          ces passes que l'IA apprendra quels événements valent le coup. Les textes des
          prompts se règlent dans Outils IA › Prompts IA.
        </carte>
        <carte titre="Objectifs">
          <ul class="space-y-1.5">
            <li>Un agenda par actif pondéré par l'impact réel mesuré — pas seulement le libellé « High » du calendrier.</li>
            <li>Un minutage d'entrée recommandé par événement et par actif.</li>
            <li>Étape 6 : benchmark du meilleur modèle local utilisable pour ce travail.</li>
          </ul>
        </carte>
        <carte titre="Garde-fous">
          <b>L'IA n'ouvre jamais de trade.</b> Le moteur applique la définition figée de
          cette page : deux jambes à E, OCO, trailing. Les propositions de l'IA (agenda,
          minutage) ne prennent effet qu'après validation du propriétaire dans les
          réglages. Aucune autonomie sur l'exécution.
        </carte>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, defineComponent, h, onMounted } from 'vue'
import { http } from '@/services/http.client'

interface ReglagesStrategie {
  etat: string; capital: number; risque_pct: number
}

// ── Mini-composants locaux (gabarit SMC) ─────────────────────────────────────
const Carte = defineComponent({
  props: { titre: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-5 py-4' }, [
      h('div', {
        class: 'text-xs font-semibold text-amber-400 uppercase tracking-widest mb-2.5',
        innerHTML: props.titre,
      }),
      h('div', {
        class: 'text-white text-sm leading-relaxed [&_b]:text-white [&_ol]:list-decimal [&_ol]:ml-5 [&_ul]:space-y-1 [&_p]:mb-2 [&_p:last-child]:mb-0',
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
      h('div', { class: 'text-[10px] text-white uppercase tracking-widest' }, p.etiquette),
      h('div', { class: 'text-lg font-bold text-white mt-1 font-mono' }, p.valeur),
    ]),
})
const valeur = Valeur

const onglets = ['Définition', 'Décision d\u2019entrée', 'Gestion des trades ouverts', 'Money management', 'Enrichissement IA'] as const
const onglet = ref<(typeof onglets)[number]>('Définition')

const reglages = ref<ReglagesStrategie | null>(null)
onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    const s = (res.data as { id: string; etat: string; capital: number; risque_pct: number }[])
      .find(x => x.id === 'straddle')
    if (s) reglages.value = s
  } catch { /* registre indisponible */ }
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
export default { name: 'StraddleDefinitionView' }
</script>
