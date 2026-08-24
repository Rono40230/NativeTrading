<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">

    <div class="flex items-center gap-3 shrink-0">
      <h1 class="text-2xl font-bold text-white">🚀 Rockets</h1>
      <span class="text-gray-500 text-base hidden sm:inline">Momentum breakout — VCP × Rocket Hunter</span>
      <span v-if="reglages" class="ml-auto text-[11px] font-semibold px-2.5 py-1 rounded-full border" :class="badgeClasse">{{ reglages.etat }}</span>
    </div>

    <div class="flex gap-1 border-b border-white/10 shrink-0 overflow-x-auto">
      <button v-for="t in onglets" :key="t" class="px-4 py-2 text-sm font-medium whitespace-nowrap transition-colors border-b-2 -mb-px"
        :class="onglet === t ? 'text-white border-blue-400' : 'text-gray-400 border-transparent hover:text-white/70'"
        @click="onglet = t">{{ t }}</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1">

      <!-- ═══ DÉFINITION ═══ -->
      <div v-if="onglet === 'Définition'" class="flex flex-col gap-3">
        <carte titre="Concept">
          Rockets capte les mouvements de volatilité VIOLENTS ET BREFS nés d'une compression :
          après une base où la volatilité et les volumes s'assèchent, la cassure du pivot sur
          un chandelier marubozu à fort volume déclenche la fusée. La stratégie chasse la
          sortie de base d'actifs déjà en tendance forte, qui surperforment le marché.
        </carte>
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Le classement — 10 points, 4 piliers">
            <ul class="space-y-1.5">
              <li><b class="text-white">Fondamental (3)</b> — sentiment de marché (BTC haussier + secteur/écosystème en tendance), contexte (sortie de large base, 1ère base), news catalyseur.</li>
              <li><b class="text-white">Technique (3)</b> — tendance (prix &gt; MM50 &gt; MM200 empilées, à moins de 25 % du plus haut 52 sem.), volatilité (compression Bollinger puis expansion), intérêt (volumes asséchés puis explosés).</li>
              <li><b class="text-white">Chartisme (2)</b> — figure de continuation (VCP / tasse avec anse, contractions décroissantes, micro-base serrée), pas de gros gaps.</li>
              <li><b class="text-white">Chandeliers (2)</b> — cassure marubozu (corps ≥ 80 %, +3-5 % au-delà du pivot, volume ≥ 150 % MM50), pas de mèches excessives.</li>
            </ul>
          </carte>
          <carte titre="La classification et le véto">
            <ul class="space-y-1.5">
              <li><b class="text-white">9-10 : ROCKET ALPHA</b> — trading neutre/offensif</li>
              <li><b class="text-white">7-8 : ROCKET</b> — trading neutre</li>
              <li><b class="text-white">&lt; 7 : ÉLIMINÉ</b></li>
            </ul>
            <p class="mt-2">Véto éliminatoire : un déverrouillage de tokens majeur (≥ 1-2 % de la supply flottante)
            dans les 30 prochains jours élimine le candidat, quel que soit son classement — l'étude
            de 16 000 unlocks montre 90 % de pression vendeuse.</p>
          </carte>
        </div>
        <carte titre="Le périmètre">
          Crypto uniquement à la naissance : scan quotidien du top 100 Binance en volume
          (blacklist des paires figées), détection sur bougies quotidiennes. Actions US et ETF
          prévus via MT5 (phase 5) — le classement ETF dispose de ses propres profils de risque.
        </carte>
      </div>

      <!-- ═══ DÉCISION D'ENTRÉE ═══ -->
      <div v-if="onglet === 'Décision d\u2019entrée'" class="flex flex-col gap-3">
        <carte titre="Le parcours d'un signal">
          <ol class="list-decimal ml-5 space-y-1.5">
            <li>Le scanner quotidien classe l'univers après la clôture D1 (00h40 UTC).</li>
            <li>Les candidats ≥ 5 points sont journalisés et suivis (page Scanner).</li>
            <li>La cassure du pivot sur la bougie D1 — décisive (+3 % minimum), marubozu, volume ≥ 150 % de la MM50 — déclenche le signal si le classement ≥ 7.</li>
            <li>Ordre <b>stop-limit</b> : achat au-delà du pivot, plafond à la limite (+3 %) pour contenir le slippage d'une cassure violente.</li>
          </ol>
        </carte>
        <carte titre="La force relative">
          Sans surperformance, pas de point Tendance : l'actif doit battre BTC sur 4 semaines
          (proxy v1 du « secteur en tendance » — le vrai découpage par écosystème viendra avec
          l'IA, étape 6). C'est le critère commun d'O'Neil (RS ≥ 80) et Minervini.
        </carte>
        <carte titre="Ce qui manque encore (honnête)">
          Le point « News » (1/10) et le véto unlocks demandent des sources externes et de la
          lecture — réservés à l'enrichissement IA (étape 6). Le classement v1 est donc noté
          sur 9 chiffrables ; le seuil d'élimination reste 7.
        </carte>
      </div>

      <!-- ═══ GESTION ═══ -->
      <div v-if="onglet === 'Gestion des trades ouverts'" class="flex flex-col gap-3">
        <carte titre="Le cycle de vie (logique du Journal de Trading)">
          <ol class="list-decimal ml-5 space-y-1.5">
            <li><b>Entrée</b> — stop-limit au pivot ; invalidation sous la dernière contraction (−1R).</li>
            <li><b>R1 atteint</b> (entrée + 1R) — <b>vendre 50 %</b> de la position (fixe) et poser le <b>trailing stop</b> à X % du prix (défaut 5 %, réglable).</li>
            <li><b>Trailing</b> — suit le prix à la clôture de chaque bougie, jamais vers l'arrière.</li>
            <li><b>Sortie</b> — le prix touche le trailing : le solde est vendu. P&amp;L = 50 % à R1 + solde à la sortie.</li>
            <li><b>Sortie sèche</b> — invalidation touchée avant R1 : −1R.</li>
          </ol>
        </carte>
        <carte titre="Verdicts">
          SL (−1R) ou TS (R mixte : 0,5 R sécurisé à R1 + 0,5 × R de sortie) — écrits en base
          avec le R réel, ils alimentent la courbe de trades du bloc. En Observation :
          journalisé, silencieux sur Telegram.
        </carte>
      </div>

      <!-- ═══ MONEY MANAGEMENT ═══ -->
      <div v-if="onglet === 'Money management'" class="flex flex-col gap-3">
        <carte titre="Profils de risque (du Journal de Trading)">
          <ul class="space-y-1.5">
            <li><b class="text-white">Peu Risqué</b> — 0,5 % du capital par rocket (ETF : 2 %)</li>
            <li><b class="text-white">Neutre</b> — 1 % (ETF : 3 %)</li>
            <li><b class="text-white">Risqué</b> — 2 % (ETF : 4 %)</li>
          </ul>
          <p class="mt-2">Le profil est un <b>choix du propriétaire</b> dans les paramètres (comme au journal),
          jamais déduit du classement — décision actée. Quantité = capital × profil ÷ |entrée − stop|,
          <b>plafonnée à 5 % du capital</b> en montant par position.</p>
        </carte>
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <valeur etiquette="Capital alloué" :valeur="reglageStr('capital')" />
          <valeur etiquette="Risque par rocket" :valeur="reglageStr('risque')" />
          <valeur etiquette="1R représente" :valeur="reglageStr('unR')" />
        </div>
      </div>

      <!-- ═══ SCANNER (spécifique Rockets — remplace le lexique) ═══ -->
      <div v-if="onglet === 'Scanner'" class="flex flex-col gap-3">
        <carte titre="Le scanner">
          Chaque jour après la clôture quotidienne (00h40 UTC), le top 100 Binance en volume
          est classé. Les candidats ≥ 5 points vivent ici — en attente de leur pivot — et la
          page dédiée (menu Rockets › Scanner) détaille chaque critère du classement.
        </carte>
      </div>

      <!-- ═══ ENRICHISSEMENT IA ═══ -->
      <div v-if="onglet === 'Enrichissement IA'" class="flex flex-col gap-3">
        <carte titre="Rôle de l'IA dans la stratégie">
          <div class="flex flex-col gap-2">
            <p><b class="text-white">1. Évaluer le catalyseur « news »</b> — le point manquant du classement :
            flux ETF, annonces de listing, réglementation — la lecture qui complète les critères chiffrables.</p>
            <p><b class="text-white">2. Ranker les faux pivots</b> — conviction sur les candidats détectés,
            pour écarter les cassures qui n'en sont pas.</p>
            <p><b class="text-white">3. Analyser la performance par pilier</b> — quels critères du classement
            gagnent réellement, pour le recalibrer.</p>
          </div>
        </carte>
        <carte titre="Fonctionnement">
          IA locale (Ollama), hors temps réel. Matière première : les candidats journalisés
          avec leur détail point par point, et les trades clôturés avec leur verdict en R.
          Les textes des prompts se règlent dans Outils IA › Prompts IA.
        </carte>
        <carte titre="Garde-fous">
          <b>L'IA n'ouvre jamais de trade.</b> Le moteur applique la définition figée de cette
          page. Ses propositions (catalyseur, véto unlocks, ranking) ne prennent effet
          qu'après validation du propriétaire. Aucune autonomie sur l'exécution.
        </carte>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, defineComponent, h, onMounted } from 'vue'
import { http } from '@/services/http.client'

interface ReglagesStrategie { etat: string; capital: number; risque_pct: number }

const Carte = defineComponent({
  props: { titre: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-5 py-4' }, [
      h('div', { class: 'text-xs font-semibold text-blue-400 uppercase tracking-widest mb-2.5', innerHTML: props.titre }),
      h('div', { class: 'text-gray-300 text-sm leading-relaxed [&_b]:text-white [&_ol]:list-decimal [&_ol]:ml-5 [&_ul]:space-y-1 [&_p]:mb-2 [&_p:last-child]:mb-0' }, slots.default?.()),
    ])
  },
})
const carte = Carte
const Valeur = defineComponent({
  props: { etiquette: { type: String, required: true }, valeur: { type: String, required: true } },
  setup: (p: { etiquette: string; valeur: string }) => () =>
    h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-4 py-3' }, [
      h('div', { class: 'text-[10px] text-gray-500 uppercase tracking-widest' }, p.etiquette),
      h('div', { class: 'text-lg font-bold text-white mt-1 font-mono' }, p.valeur),
    ]),
})
const valeur = Valeur

const onglets = ['Définition', 'Décision d\u2019entrée', 'Gestion des trades ouverts', 'Money management', 'Scanner', 'Enrichissement IA'] as const
const onglet = ref<(typeof onglets)[number]>('Définition')

const reglages = ref<ReglagesStrategie | null>(null)
onMounted(async () => {
  try {
    const res = await http.get('/api/strategies')
    const s = (res.data as { id: string; etat: string; capital: number; risque_pct: number }[]).find(x => x.id === 'rockets')
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
  if (champ === 'risque') return `${r.risque_pct} % (réglable par profil)`
  return r.capital > 0 ? `${(r.capital * r.risque_pct / 100).toLocaleString('fr-FR')} $` : '—'
}
</script>

<script lang="ts">
export default { name: 'RocketsDefinitionView' }
</script>
