<template>
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">

    <!-- En-tête : identité + état du registre -->
    <div class="flex items-center gap-3 shrink-0">
      <RouterLink to="/kdj"
        class="text-[11px] px-2.5 py-1 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors whitespace-nowrap"
        title="Retour à la stratégie KDJ/Halftrend"
      >← KDJ/Halftrend</RouterLink>
      <h1 class="text-2xl font-bold text-white">📈 Les caractéristiques de la stratégie KDJ/Halftrend</h1>
      <span
        v-if="reglages"
        class="ml-auto text-[11px] font-semibold px-2.5 py-1 rounded-full border"
        :class="badgeClasse"
      >{{ reglages.etat }}</span>
    </div>

    <!-- Barre d'onglets -->
    <div class="flex gap-1 border-b border-white/10 shrink-0">
      <button
        v-for="t in onglets"
        :key="t"
        class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px"
        :class="onglet === t ? 'text-white border-cyan-400' : 'text-white border-transparent hover:text-white/70'"
        @click="onglet = t"
      >{{ t }}</button>
    </div>

    <div class="flex-1 min-h-0 overflow-y-auto pr-1">

      <!-- ═══ ONGLET DÉFINITION ═══ -->
      <div v-if="onglet === 'Définition'" class="flex flex-col gap-3">
        <carte titre="Concept">
          <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
            <!-- La baisse, le creux, la remontée : le V -->
            <polyline points="20,30 70,45 120,58 170,66 200,70 240,60 290,42 340,26 410,18"
                      fill="none" stroke="#e5e7eb" stroke-width="1.4" stroke-linejoin="round" />
            <!-- L'EMA200 sous le prix : le sol de la tendance -->
            <polyline points="20,48 90,55 170,62 250,64 330,54 410,44"
                      fill="none" stroke="#a78bfa" stroke-width="1" stroke-dasharray="4 3" />
            <text x="380" y="40" fill="#a78bfa" font-size="7" font-weight="700">EMA200</text>
            <!-- La flèche HalfTrend au creux -->
            <polygon points="200,60 195,70 205,70" fill="#22d3ee" />
            <text x="212" y="70" fill="#22d3ee" font-size="7.5" font-weight="700">flèche HalfTrend</text>
            <!-- Le point d'entrée à l'open suivant -->
            <circle cx="240" cy="60" r="2.8" fill="#ffffff" />
            <text x="250" y="55" fill="#ffffff" font-size="7" font-weight="700">entrée (open suivant)</text>
          </svg>
          La stratégie capture les REPRISES de tendance : après un repli dans une tendance franche,
          le retournement du HalfTrend rallume l'entrée — confirmé par le KDJ et le filtre
          EMA200. Elle ne prévoit rien : elle <b class="text-white">suit</b>. Sur le screening et
          le rejeu 24 mois, elle vit sur les actifs en tendance (métaux en tête) et meurt en
          range — d'où le scanner qui sélectionne les tendances franches.
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Le déclencheur">
            Périmètre armé : <b class="text-white">H1, tous les actifs collectés</b> (XAUUSD,
            XAGUSD, NAS100, SP500, DAX, BTC). La flèche HalfTrend n'existe qu'à la barre de
            retournement (1→0 haussier, 0→1 baissier) — un événement rare (~1 à 3 par mois et
            par actif en H1) : le moteur ne trade qu'à ces instants.
          </carte>
          <carte titre="La fenêtre">
            Tout se joue <b class="text-white">à la clôture de la bougie H1</b> : conditions et
            flèche évaluées sur la bougie fermée, ordre exécuté à l'open de la bougie suivante
            (modèle TradingView). Aucune décision intrabar, aucun repaint — la convention
            anti-repaint de la définition.
          </carte>
        </div>
      </div>

      <!-- ═══ ONGLET LEXIQUE ═══ -->
      <div v-if="onglet === 'Lexique'" class="flex flex-col gap-3">
        <LexiquePanel source="kdj" />
      </div>

      <!-- ═══ ONGLET DÉCISION D'ENTRÉE ═══ -->
      <div v-if="onglet === 'Décision d\u2019entrée'" class="flex flex-col gap-3">
        <carte titre="Les trois conditions — et la flèche">
          <svg viewBox="0 0 440 70" class="w-full aspect-[440/70] mb-3">
            <rect x="14" y="22" width="88" height="24" rx="4" fill="rgba(34,211,238,0.08)" stroke="#22d3ee" stroke-width="1" />
            <text x="58" y="37.5" text-anchor="middle" fill="#22d3ee" font-size="8" font-weight="700">Flèche HalfTrend</text>
            <text x="58" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">le déclencheur</text>
            <rect x="154" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="198" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">KDJ : J &gt; D</text>
            <text x="198" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">momentum repris</text>
            <rect x="294" y="22" width="88" height="24" rx="4" fill="rgba(167,139,250,0.08)" stroke="#a78bfa" stroke-width="1" />
            <text x="338" y="37.5" text-anchor="middle" fill="#a78bfa" font-size="8" font-weight="700">close &gt; EMA200</text>
            <text x="338" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">et SMA100 &gt; EMA200</text>
            <line x1="106" y1="34" x2="146" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="150,34 144,31.5 144,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="246" y1="34" x2="286" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="290,34 284,31.5 284,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Long</div>
              <p>SMA100 &gt; EMA200, J &gt; D, close &gt; EMA200 — flèche haussière.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Short (miroir strict)</div>
              <p>SMA100 &lt; EMA200, J &lt; D, close &lt; EMA200 — flèche baissière.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Filtrées</div>
              <p>Une flèche sans conditions = rien. Une condition sans flèche = rien : le
              croisement des DEUX seulement entre.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">④ ADX (option)</div>
              <p>Réglable (−1 = off, fidélité étalon) : n'entrer que si ADX(14) ≥ seuil —
              l'étude montre qu'il aide les métaux, pas les indices.</p>
            </div>
          </div>
        </carte>

        <carte titre="Les quatre réglages de l'étalon">
          <div class="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-3">
            <valeur etiquette="period" :valeur="String(params.period)" />
            <valeur etiquette="signal" :valeur="String(params.signal)" />
            <valeur etiquette="amplitude" :valeur="String(params.amplitude)" />
            <valeur etiquette="RatioRisk" :valeur="params.ratio_risk.toFixed(1)" />
          </div>
          <p>Les valeurs du screening et du rejeu validés (20 · 7 · 2 · 2,0). Modifiables dans
          la carte du dashboard › Paramètres moteur — effet au prochain redémarrage.</p>
        </carte>
      </div>

      <!-- ═══ ONGLET GESTION DES TRADES OUVERTS ═══ -->
      <div v-if="onglet === 'Gestion des trades ouverts'" class="flex flex-col gap-3">
        <carte titre="L'échelle d'un trade long — figée à la barre d'entrée">
          <svg viewBox="0 0 440 110" class="w-full aspect-[440/110] mb-2">
            <line x1="68" y1="22" x2="290" y2="22" stroke="#34d399" stroke-width="0.9" stroke-dasharray="4 3" />
            <text x="298" y="25" fill="#34d399" font-size="7.5" font-weight="700">TP · +RatioRisk × R</text>
            <line x1="68" y1="52" x2="290" y2="52" stroke="#ffffff" stroke-width="0.9" stroke-dasharray="4 3" />
            <text x="298" y="55" fill="#ffffff" font-size="7.5" font-weight="700">E · l'entrée (open)</text>
            <line x1="68" y1="88" x2="290" y2="88" stroke="#f87171" stroke-width="0.9" stroke-dasharray="4 3" />
            <text x="298" y="91" fill="#f87171" font-size="7.5" font-weight="700">SL · EMA200 (barre d'entrée)</text>
            <text x="20" y="55" fill="#e5e7eb" font-size="7" font-weight="700">R = E − EMA200</text>
            <polyline points="30,80 60,86 90,78 130,60 170,48 220,38 255,24" fill="none" stroke="#34d399" stroke-width="1.6" stroke-linejoin="round" />
            <circle cx="30" cy="80" r="2.6" fill="#ffffff" />
          </svg>
          <p>Tout est figé à la barre d'entrée : E = son open, EMA200 = sa valeur à la clôture
          de cette même barre (le <b class="text-white">valuewhen</b> du Pine). Le TP vaut
          E ± RatioRisk × la distance E→EMA200 ; le SL est l'EMA200 elle-même. R, l'unité de
          risque, vaut cette distance.</p>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Aucun ordre serveur — le piège n°1">
            <p>Le Pine étalon ne place <b class="text-white">jamais</b> d'ordres limit/stop :
            TP et SL sont des <b class="text-white">seuils de détection</b>, testés par
            CROISEMENTS à la clôture. La sortie se fait au marché, à l'open de la bougie
            suivante. Le P&L réel vaut open(sortie) − open(entrée), jamais le niveau.</p>
          </carte>
          <carte titre="Les croisements — le piège n°4">
            <p>Un low déjà SOUS le niveau ne « cross » pas : tant que le prix n'est pas repassé
            au-dessus puis redescendu, le SL ne se déclenche pas. Ces trades « zombies »
            existent dans l'étalon, le rejeu et le moteur live — reproduits à l'identique,
            jamais « réparés ».</p>
          </carte>
        </div>

        <carte titre="Le cycle de vie">
          <svg viewBox="0 0 560 70" class="w-full aspect-[560/70] mb-3">
            <rect x="6" y="22" width="88" height="24" rx="4" fill="rgba(255,255,255,0.05)" stroke="rgba(255,255,255,0.55)" stroke-width="1" />
            <text x="50" y="37.5" text-anchor="middle" fill="#ffffff" font-size="7.5" font-weight="700">Flèche + conditions</text>
            <text x="50" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6">à la clôture N</text>
            <rect x="146" y="22" width="88" height="24" rx="4" fill="rgba(34,211,238,0.08)" stroke="#22d3ee" stroke-width="1" />
            <text x="190" y="37.5" text-anchor="middle" fill="#22d3ee" font-size="8" font-weight="700">Entrée</text>
            <text x="190" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">open de N+1 · fige</text>
            <rect x="286" y="22" width="88" height="24" rx="4" fill="rgba(96,165,250,0.08)" stroke="#60a5fa" stroke-width="1" />
            <text x="330" y="37.5" text-anchor="middle" fill="#60a5fa" font-size="8" font-weight="700">Surveillance</text>
            <text x="330" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">crosses à chaque clôture</text>
            <rect x="426" y="22" width="88" height="24" rx="4" fill="rgba(52,211,153,0.08)" stroke="#34d399" stroke-width="1" />
            <text x="470" y="37.5" text-anchor="middle" fill="#34d399" font-size="8" font-weight="700">Sortie</text>
            <text x="470" y="63" text-anchor="middle" fill="#e5e7eb" font-size="6.5">open de la barre suivante</text>
            <line x1="98" y1="34" x2="138" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="142,34 136,31.5 136,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="238" y1="34" x2="278" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="282,34 276,31.5 276,36.5" fill="rgba(255,255,255,0.4)" />
            <line x1="378" y1="34" x2="418" y2="34" stroke="rgba(255,255,255,0.4)" stroke-width="1.2" />
            <polygon points="422,34 416,31.5 416,36.5" fill="rgba(255,255,255,0.4)" />
          </svg>
          <div class="flex flex-wrap gap-2 mb-2">
            <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-emerald-400 border-emerald-400/40 bg-emerald-400/10">TP</span>
            <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-red-400 border-red-400/40 bg-red-400/10">SL</span>
            <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-amber-400 border-amber-400/40 bg-amber-400/10">Retournement</span>
            <span class="px-2.5 py-1 rounded-full border text-xs font-semibold text-white border-white/40 bg-white/10">Ouvert (fin de données)</span>
          </div>
          <p>Le <b class="text-white">Retournement</b> : un signal inverse complet ferme la
          position et ouvre l'inverse au MÊME open — le seul cas de double mouvement. Si TP et
          SL croisent à la même clôture, le verdict est TP (ordre de l'expression dans
          l'étalon). Position encore ouverte en fin de données = verdict Ouvert, exclu des
          stats.</p>
        </carte>
      </div>

      <!-- ═══ ONGLET MONEY MANAGEMENT ═══ -->
      <div v-if="onglet === 'Money management'" class="flex flex-col gap-3">
        <carte titre="La formule du lot">
          <div class="flex flex-wrap items-center justify-center gap-x-3 gap-y-2 font-mono text-xl py-3">
            <span class="text-violet-400 font-bold">lot</span>
            <span class="text-white">=</span>
            <span class="text-white">(</span>
            <span class="font-bold">capital</span>
            <span class="text-white">×</span>
            <span class="text-blue-400 font-bold">risque %</span>
            <span class="text-white">) ÷ (</span>
            <span class="text-amber-400 font-bold">stop</span>
            <span class="text-white">×</span>
            <span class="text-emerald-400 font-bold">valeur du pip</span>
            <span class="text-white">)</span>
          </div>
          <p class="text-center">Le stop vaut la distance entrée→EMA200 de la barre d'entrée :
          le lot s'adapte à la largeur de la tendance, le risque en euros reste le même.</p>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-3">
          <valeur etiquette="Capital alloué" :valeur="reglageStr('capital')" />
          <valeur etiquette="Risque par trade" :valeur="reglageStr('risque')" />
          <valeur etiquette="1R représente" :valeur="reglageStr('unR')" />
        </div>

        <carte titre="Le R, unité de compte">
          <p>R = distance entre l'entrée et l'EMA200 de la barre d'entrée. Le TP vaut
          RatioRisk × R (2,0 par défaut) : un gagnant complet rapporte ~2R, un perdant perd
          ~1R — au prix d'open près. Attention métrologie (leçon 7.B) : une entrée collée à
          l'EMA200 fabrique un R minuscule et un R/trade explosif sans signification — le
          juge historique de la stratégie est le % du prix par trade ; le R sert au
          dimensionnement réel.</p>
        </carte>
      </div>

      <!-- ═══ ONGLET PREUVES & GARDE-FOUS ═══ -->
      <div v-if="onglet === 'Preuves et garde-fous'" class="flex flex-col gap-3">
        <carte titre="La chaîne de preuves">
          <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3">
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">① Screening TV</div>
              <p>8 actifs × 3 TF, frais inclus : 4 porteurs (XAU, XAG, NAS100, DAX) — l'actif
              de naissance (ETH) exclu et non concluant.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">② Rejeu Rust 24 mois</div>
              <p>Recoupe TV sur 5 actifs en % du prix : XAU +0,12→0,38 %/trade, XAG
              +0,19→0,66, DAX/BTC/SP conformes.</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">③ Parité MQ5</div>
              <p>Au centime : 6/6 trades, 0 divergence, sur les bougies exactes du Strategy
              Tester (diff_kdj_3voies).</p>
            </div>
            <div class="rounded-lg border border-white/10 bg-black/20 px-3.5 py-3">
              <div class="font-semibold mb-1">④ Moteur live</div>
              <p>Parité avec le rejeur prouvée par test : même trade, même R, au centime —
              par construction (mêmes indicateurs, mêmes conventions).</p>
            </div>
          </div>
        </carte>

        <div class="grid grid-cols-1 lg:grid-cols-2 gap-3">
          <carte titre="Aucune IA dans la boucle">
            <p>Le moteur est <b class="text-white">déterministe pur</b> : KDJ, HalfTrend et
            filtres EMA — aucune couche IA, aucun llm, rien à interpréter. La constitution
            s'applique tout de même : l'IA (moi) lit, juge, propose et explique ; les moteurs
            décident ; toi seul règles — armement, réglages, périmètre.</p>
          </carte>
          <carte titre="L'étalon et la mesure">
            <p>Pine v4 figé (md5 <span class="font-mono text-cyan-300">be3343ed…</span>),
            définition de référence avec les 4 pièges :
            <span class="font-mono">docs/reference/definition_kdj_halftrend.md</span>.
            Toute divergence entre Pine, Rust et MQ5 est un bug. La règle du projet
            s'applique : juger à 30 trades par couple, pas avant.</p>
          </carte>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import LexiquePanel from '@/components/common/LexiquePanel.vue'
import { ref, computed, defineComponent, h, onMounted } from 'vue'
import { http } from '@/services/http.client'

interface ReglagesStrategie {
  etat: string; capital: number; risque_pct: number
}
interface ParamsMoteur {
  period: number; signal: number; amplitude: number; ratio_risk: number; adx_min: number
}

// ── Mini-composants locaux (gabarit Straddle) ────────────────────────────────
const Carte = defineComponent({
  props: { titre: { type: String, required: true } },
  setup(props, { slots }) {
    return () => h('div', { class: 'rounded-xl border border-white/10 bg-white/5 px-5 py-4' }, [
      h('div', {
        class: 'text-xs font-semibold text-cyan-400 uppercase tracking-widest mb-2.5',
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

// ── Onglets (gabarit projet) ─────────────────────────────────────────────────
const onglets = ['Définition', 'Lexique', 'Décision d\u2019entrée', 'Gestion des trades ouverts', 'Money management', 'Preuves et garde-fous'] as const
const onglet = ref<(typeof onglets)[number]>('Définition')

const reglages = ref<ReglagesStrategie | null>(null)
const params = ref<ParamsMoteur>({ period: 20, signal: 7, amplitude: 2, ratio_risk: 2.0, adx_min: -1 })
onMounted(async () => {
  try {
    const [reg, par] = await Promise.all([
      http.get('/api/strategies'),
      http.get<ParamsMoteur>('/api/kdj/params'),
    ])
    const s = (reg.data as { id: string; etat: string; capital: number; risque_pct: number }[])
      .find(x => x.id === 'kdj_halftrend')
    if (s) reglages.value = s
    params.value = par.data
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
export default { name: 'KdjDefinitionView' }
</script>
