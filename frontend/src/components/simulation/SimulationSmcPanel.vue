<template>
  <div class="flex flex-col gap-4">
    <!-- Limite méthodologique -->
    <p class="text-[11px] text-white/70 border-l-2 border-amber-400/50 pl-3">
      La simulation rejoue les <b>trades réellement pris</b> avec d'autres niveaux — elle ne peut pas
      inventer les trades qu'un autre réglage aurait fait naître. Toute conclusion sur une tranche
      de moins de 30 trades n'est pas significative (règle du projet).
    </p>

    <!-- ═══ Rangée 1 : vécu officiel | paramètres virtuels ═══ -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- 1 — Résultats en direct (avant modification) -->
      <section class="glass-card p-4 flex flex-col gap-3">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">1 · Résultats en direct — vécu officiel</h2>
        <div v-if="vecu" class="grid grid-cols-3 gap-2">
          <div class="kpi"><p class="kpi-label">Capital</p>
            <p class="kpi-valeur" :class="rendementVecu >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollars(vecu.capital_actuel) }}</p>
            <p class="kpi-sous">{{ fmtPct(rendementVecu) }}</p></div>
          <div class="kpi"><p class="kpi-label">Σ R encaissé (simulé)</p>
            <p class="kpi-valeur" :class="vecu.r_total >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR2(vecu.r_total) }}</p>
            <p class="kpi-sous">{{ vecu.nb_trades }} clôtures</p></div>
          <div class="kpi"><p class="kpi-label">WR ($ &gt; 0)</p>
            <p class="kpi-valeur text-white">{{ (vecu.taux_reussite * 100).toFixed(0) }} %</p>
            <p class="kpi-sous">pire creux {{ fmtDollars(creuxVecu) }}</p></div>
        </div>
        <p v-else class="text-xs text-white/60 py-4 text-center">Chargement du vécu…</p>
      </section>

      <!-- 2 — Paramètres ayant une incidence -->
      <section class="glass-card p-4 flex flex-col gap-3">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">2 · Paramètres (virtuels)</h2>
        <div class="grid grid-cols-2 gap-2 text-xs">
          <label class="flex flex-col gap-1">TP1 (× R)
            <input v-model.number="params.tp1_mult" type="number" step="0.1" min="0.2" max="1.5" class="champ" /></label>
          <label class="flex flex-col gap-1">TP2 (× R)
            <input v-model.number="params.tp2_mult" type="number" step="0.1" min="1" max="4" class="champ" /></label>
          <label class="flex flex-col gap-1 col-span-2">TP3 — mode
            <select v-model="params.tp3_mode" class="champ">
              <option value="lointaine">Liquidité lointaine</option>
              <option value="rfixe">R fixe</option>
            </select></label>
          <label class="flex flex-col gap-1">TP3 R fixe
            <input v-model.number="params.tp3_rfixe" type="number" step="0.5" min="3" max="10" class="champ" /></label>
          <label class="flex flex-col gap-1">Trailing après TP2 (× R)
            <input v-model.number="params.tp3_trailing_r" type="number" step="0.1" min="0.1" max="1" class="champ" :disabled="!params.tp3_trailing" /></label>
          <label class="flex items-center gap-2 col-span-2 text-white">
            <input v-model="params.tp3_trailing" type="checkbox" class="accent-teal-400" /> Activer le trailing</label>
          <p class="col-span-2 text-white/50 text-[10px] -mt-1">Fractions du lot : la somme est normalisée à 100 % à l'envoi.</p>
          <div class="col-span-2 grid grid-cols-3 gap-2">
            <label class="flex flex-col gap-1">f1 vendue à TP1 (%)
              <input v-model.number="pct1" type="number" step="5" min="0" max="100" class="champ" /></label>
            <label class="flex flex-col gap-1">f2 vendue à TP2 (%)
              <input v-model.number="pct2" type="number" step="5" min="0" max="100" class="champ" /></label>
            <label class="flex flex-col gap-1">f3 = solde (%)
              <input v-model.number="pct3" type="number" step="5" min="0" max="100" class="champ" /></label>
          </div>
        </div>
        <!-- Périmètre de simulation (virtuel — ne touche pas à l'armement) -->
        <div class="flex flex-col gap-1.5">
          <p class="text-[10px] font-semibold uppercase tracking-wider text-white/60">Paires simulées <span class="font-normal normal-case text-white/40">— vide = toutes les armées</span></p>
          <div class="flex flex-wrap gap-1">
            <button v-for="a in assetsDispo" :key="'pa' + a"
                    class="text-[10px] px-1.5 py-0.5 rounded border font-mono transition-colors"
                    :class="filtreAssets.includes(a) ? 'border-teal-400/50 bg-teal-500/20 text-white' : 'border-white/10 bg-white/[0.03] text-white/50 hover:border-white/25'"
                    @click="basculer(filtreAssets, a)">{{ a }}</button>
          </div>
          <p class="text-[10px] font-semibold uppercase tracking-wider text-white/60 mt-1">Timeframes simulés</p>
          <div class="flex flex-wrap gap-1">
            <button v-for="tf in ['M1','M5','M15','M30']" :key="'pt' + tf"
                    class="text-[10px] px-1.5 py-0.5 rounded border font-mono transition-colors"
                    :class="filtreTfs.includes(tf) ? 'border-teal-400/50 bg-teal-500/20 text-white' : 'border-white/10 bg-white/[0.03] text-white/50 hover:border-white/25'"
                    @click="basculer(filtreTfs, tf)">{{ tf }}</button>
          </div>
        </div>
        <div class="flex gap-2 flex-wrap">
          <button class="btn-action bg-teal-500/20 text-teal-300 hover:bg-teal-500/30 disabled:opacity-40"
                  :disabled="enCours" @click="lancer">
            {{ enCours ? '⏳ Re-jeu en cours… (~35 s)' : '▶ Lancer la simulation' }}</button>
          <button class="btn-action bg-white/10 text-white hover:bg-white/20 disabled:opacity-40"
                  :disabled="enCoursBalayage" @click="balayer">
            {{ enCoursBalayage ? '⏳ Balayage…' : '📊 Balayer les fractions' }}</button>
          <button class="btn-action bg-white/10 text-white hover:bg-white/20 disabled:opacity-40"
                  :disabled="enCoursTrailing" :title="'Active le trailing (après TP2) et balaye son k × R — re-jeu exact du moteur, ~100 s. Un trailing ATR roulant SMC serait une déviation de l\'étalon Pine : à voter séparément.'"
                  @click="balayerTrailing">
            {{ enCoursTrailing ? '⏳ Balayage trailing… (~100 s)' : '📊 Balayer le k du trailing' }}</button>
          <button class="btn-action bg-emerald-600/20 text-emerald-300 hover:bg-emerald-600/30 disabled:opacity-40"
                  :disabled="!modifie" :title="modifie ? 'Écrit ces réglages dans la config réelle (futurs signaux) — le re-jeu officiel relance en fond' : 'Aucune modification par rapport aux réglages actuels'"
                  @click="appliquer">✅ Appliquer ces réglages</button>
        </div>
        <p v-if="messageApplique" class="text-[11px] text-emerald-300">{{ messageApplique }}</p>
      </section>
    </div>

    <!-- ═══ 3 — Résultats après re-jeu : comparatif côte à côte ═══ -->
    <section v-if="sim" class="glass-card p-4 flex flex-col gap-3">
      <div class="flex items-center gap-2 flex-wrap">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">3 · Résultats après re-jeu</h2>
        <span class="text-[10px] text-white/60">calculé en {{ (sim.duree_ms / 1000).toFixed(1) }} s</span>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-white/60 uppercase tracking-wider border-b border-white/10">
              <th class="text-left py-1.5 pr-3">Métrique</th>
              <th class="text-right py-1.5 px-2">Vécu officiel</th>
              <th class="text-right py-1.5 px-2">Simulation</th>
              <th class="text-right py-1.5 pl-2">Écart</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="l in comparatif" :key="l.label" class="border-b border-white/5">
              <td class="py-1.5 pr-3 text-white">{{ l.label }}</td>
              <td class="py-1.5 px-2 text-right font-mono text-white/80">{{ l.vecu }}</td>
              <td class="py-1.5 px-2 text-right font-mono font-bold" :class="l.classe">{{ l.sim }}</td>
              <td class="py-1.5 pl-2 text-right font-mono" :class="l.classe">{{ l.ecart }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div>
        <p class="text-[10px] uppercase tracking-wider text-white/60 mb-1">Verdicts de la simulation</p>
        <div class="flex flex-wrap gap-2">
          <span v-for="v in sim.verdicts" :key="v.label"
                class="text-[11px] px-2 py-1 rounded-full border font-mono"
                :class="v.n < 30 ? 'border-white/10 bg-white/5 text-white/60' : 'border-white/20 bg-white/10 text-white'"
                :title="v.n < 30 ? `${v.n} trades — non significatif (règle des 30)` : `${v.n} trades`">
            {{ v.label }} × {{ v.n }} <span :class="v.r >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR2(v.r) }}</span>
          </span>
        </div>
      </div>
    </section>

    <!-- ═══ 4 — Balayage des fractions ═══ -->
    <section v-if="balayage" class="glass-card p-4 flex flex-col gap-3">
      <div class="flex items-center gap-2 flex-wrap">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">4 · Balayage des fractions</h2>
        <span class="text-[10px] text-white/60">{{ balayage.nb_clotures }} clôtures vécues · verdicts inchangés, seule la découpe du lot varie</span>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-white/60 uppercase tracking-wider border-b border-white/10">
              <th class="text-right py-1.5 pr-2">#</th>
              <th class="text-right py-1.5 px-2">f1 · TP1</th>
              <th class="text-right py-1.5 px-2">f2 · TP2</th>
              <th class="text-right py-1.5 px-2">f3 · solde</th>
              <th class="text-right py-1.5 px-2">Capital</th>
              <th class="text-right py-1.5 px-2">Rendement</th>
              <th class="text-right py-1.5 px-2">Pire creux</th>
              <th class="text-right py-1.5 pl-2">ΣR pondéré</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(l, i) in balayage.configurations.slice(0, 12)" :key="`${l.f1}-${l.f2}`"
                class="border-b border-white/5" :class="l.actuel ? 'bg-teal-500/10' : ''">
              <td class="py-1.5 pr-2 text-right text-white/50">{{ i + 1 }}</td>
              <td class="py-1.5 px-2 text-right font-mono text-white">{{ Math.round(l.f1 * 100) }} %</td>
              <td class="py-1.5 px-2 text-right font-mono text-white">{{ Math.round(l.f2 * 100) }} %</td>
              <td class="py-1.5 px-2 text-right font-mono text-white">{{ Math.round(l.f3 * 100) }} %</td>
              <td class="py-1.5 px-2 text-right font-mono font-bold" :class="l.capital >= balayage.capital_depart ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollars(l.capital) }}</td>
              <td class="py-1.5 px-2 text-right font-mono" :class="l.rendement >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtPct(l.rendement) }}</td>
              <td class="py-1.5 px-2 text-right font-mono text-white/70">{{ fmtDollars(l.capital_minimum) }}</td>
              <td class="py-1.5 pl-2 text-right font-mono" :class="l.r_total_pondere >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtR2(l.r_total_pondere) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="text-[10px] text-white/50">Classement sur les 231 configurations (pas 0,05) ; ligne verte = réglage actuel.</p>
    </section>

    <!-- ═══ 4-bis — Balayage du k de trailing (re-jeu exact) ═══ -->
    <section v-if="balayageTrailingRes" class="glass-card p-4 flex flex-col gap-3">
      <div class="flex items-center gap-2 flex-wrap">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">4-bis · Balayage du k de trailing (×R, après TP2)</h2>
        <span class="text-[10px] text-white/60">re-jeu exact du moteur — le trailing est INACTIF en production aujourd'hui</span>
      </div>
      <div v-if="balayageTrailingRes.moteur_actuel" class="text-xs text-white/70 mb-1">
        Moteur actuel (sans trailing) : <span class="font-mono" :class="balayageTrailingRes.moteur_actuel.rendement >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtPct(balayageTrailingRes.moteur_actuel.rendement) }}</span> · {{ balayageTrailingRes.moteur_actuel.clotures }} clôtures
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-white/60 uppercase tracking-wider border-b border-white/10">
              <th class="text-right py-1.5 pr-2">#</th>
              <th class="text-right py-1.5 px-2">k (×R)</th>
              <th class="text-right py-1.5 px-2">Clôtures</th>
              <th class="text-right py-1.5 px-2">Capital</th>
              <th class="text-right py-1.5 px-2">Rendement</th>
              <th class="text-right py-1.5 px-2">Pire creux</th>
              <th class="text-right py-1.5 pl-2">ΣR pondéré</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(l, i) in balayageTrailingRes.configurations" :key="l.k" class="border-b border-white/5">
              <td class="py-1.5 pr-2 text-right text-white/50">{{ i + 1 }}</td>
              <td class="py-1.5 px-2 text-right font-mono text-white">{{ l.k.toFixed(1) }}</td>
              <td class="py-1.5 px-2 text-right font-mono text-white/70">{{ l.clotures }}</td>
              <td class="py-1.5 px-2 text-right font-mono font-bold" :class="l.capital >= balayageTrailingRes.capital_depart ? 'text-emerald-400' : 'text-red-400'">{{ fmtDollars(l.capital) }}</td>
              <td class="py-1.5 px-2 text-right font-mono" :class="l.rendement >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ fmtPct(l.rendement) }}</td>
              <td class="py-1.5 px-2 text-right font-mono text-white/70">{{ fmtDollars(l.capital_minimum) }}</td>
              <td class="py-1.5 pl-2 text-right font-mono" :class="l.r_total_pondere >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ l.r_total_pondere >= 0 ? '+' : '−' }}{{ Math.abs(l.r_total_pondere).toFixed(2) }} R</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="text-[10px] text-white/50">Chaque k = re-jeu complet du moteur (cache 30 min). Un trailing ATR-roulant SMC exigerait de dévier de l'étalon Pine — non inclus.</p>
    </section>

    <!-- ═══ 5 — Bibliothèque des essais ═══ -->
    <section class="glass-card p-4 flex flex-col gap-3">
      <div class="flex items-center gap-2">
        <h2 class="text-sm font-bold text-white uppercase tracking-wider">5 · Bibliothèque des essais</h2>
        <span class="ml-auto text-[10px] text-white/60">clic = recharge les paramètres · 🗑 = supprime</span>
      </div>
      <div v-if="essais.length" class="flex flex-col gap-1">
        <div v-for="e in essais" :key="e.id"
             class="flex items-center gap-3 text-xs rounded-lg px-3 py-2 bg-white/5 border border-white/10 hover:border-teal-400/40 cursor-pointer"
             @click="rechargerParams(e.params)">
          <span class="text-white/60 font-mono shrink-0">{{ dateCourte(e.cree_le) }}</span>
          <span class="text-white truncate">TP1 {{ e.params.tp1_mult }}R · TP2 {{ e.params.tp2_mult }}R · f {{ fmtFrac(e.params) }}</span>
          <span class="ml-auto font-mono shrink-0" :class="(e.resultat?.capital_actuel ?? 0) >= (e.resultat?.capital_depart ?? 1) ? 'text-emerald-400' : 'text-red-400'">
            {{ fmtDollars(e.resultat?.capital_actuel ?? 0) }} · {{ fmtR2(e.resultat?.r_total_pondere ?? 0) }}
          </span>
          <button class="text-white/50 hover:text-red-400 shrink-0" title="Supprimer l'essai"
                  @click.stop="supprimerEssai(e.id)">🗑</button>
        </div>
      </div>
      <p v-else class="text-xs text-white/60 py-2 text-center">Aucun essai encore — lance une simulation, elle sera conservée ici.</p>
    </section>
  </div>

    <ModaleConfirmation
      :ouverte="confirmationOuverte"
      titre="⚙️ Appliquer ces réglages ?"
      sous-titre="Écriture dans la config RÉELLE — effet sur les futurs signaux uniquement."
      :lignes="lignesConfirmation"
      avertissement="Le vécu ne change jamais. Le re-jeu officiel relance en fond."
      label-confirmer="✅ Appliquer"
      @confirmer="executerApplication(); confirmationOuverte = false"
      @annuler="confirmationOuverte = false"
    />
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'
import ModaleConfirmation from '@/components/common/ModaleConfirmation.vue'
import { useAlerteStore } from '@/stores/alerte.store'
import { chargerAnalyse, type AnalyseStrategie } from '@/composables/useAnalyses'

const alerteStore = useAlerteStore()

interface ParamsSmc {
  tp1_mult: number; tp2_mult: number; tp3_mode: string; tp3_rfixe: number
  tp3_trailing: boolean; tp3_trailing_r: number
  frac_tp1: number; frac_tp2: number; frac_tp3: number
}
interface VerdictSim { label: string; n: number; r: number }
interface ResultatSim {
  nb_trades: number; taux_reussite: number; r_total: number; r_total_pondere: number
  capital_depart: number; capital_actuel: number; capital_minimum: number
  verdicts: VerdictSim[]; duree_ms: number
}
interface LigneBalayage {
  f1: number; f2: number; f3: number; capital: number; rendement: number
  capital_minimum: number; r_total_pondere: number; actuel: boolean
}
interface Essai { id: string; params: ParamsSmc; resultat: ResultatSim; cree_le: number }

const DEFAUTS: ParamsSmc = {
  tp1_mult: 0.6, tp2_mult: 2.0, tp3_mode: 'lointaine', tp3_rfixe: 3,
  tp3_trailing: false, tp3_trailing_r: 0.5,
  frac_tp1: 0.5, frac_tp2: 0.3, frac_tp3: 0.2,
}

const vecu = ref<AnalyseStrategie | null>(null)
const creuxVecu = ref(0)
const params = ref<ParamsSmc>({ ...DEFAUTS })
const paramsActuels = ref<ParamsSmc>({ ...DEFAUTS })
const sim = ref<ResultatSim | null>(null)
const balayage = ref<{ capital_depart: number; nb_clotures: number; configurations: LigneBalayage[] } | null>(null)
interface LigneTrailing { k: number; capital: number; rendement: number; capital_minimum: number; r_total_pondere: number; clotures: number }
const balayageTrailingRes = ref<{ capital_depart: number; configurations: LigneTrailing[]; moteur_actuel: LigneTrailing | null } | null>(null)
const enCoursTrailing = ref(false)
const essais = ref<Essai[]>([])
const filtreAssets = ref<string[]>([])
const filtreTfs = ref<string[]>([])
const assetsDispo = ref<string[]>([])

function basculer(liste: string[], v: string) {
  const i = liste.indexOf(v)
  if (i >= 0) liste.splice(i, 1)
  else liste.push(v)
}
const enCours = ref(false)
const enCoursBalayage = ref(false)
const confirmationOuverte = ref(false)
const messageApplique = ref('')

/// Saisie des fractions en % — stockage 0-1 inchangé (backend, essais).
const pct1 = computed({
  get: () => Math.round(params.value.frac_tp1 * 100),
  set: (v: number) => { params.value.frac_tp1 = (Number.isFinite(v) ? v : 0) / 100 },
})
const pct2 = computed({
  get: () => Math.round(params.value.frac_tp2 * 100),
  set: (v: number) => { params.value.frac_tp2 = (Number.isFinite(v) ? v : 0) / 100 },
})
const pct3 = computed({
  get: () => Math.round(params.value.frac_tp3 * 100),
  set: (v: number) => { params.value.frac_tp3 = (Number.isFinite(v) ? v : 0) / 100 },
})

const rendementVecu = computed(() =>
  vecu.value && vecu.value.capital_depart > 0
    ? vecu.value.capital_actuel / vecu.value.capital_depart - 1 : 0)

/// Diff paramètres virtuels vs réglages réels (clés de config à écrire).
const CLES: Record<string, (p: ParamsSmc) => string | number> = {
  smc_tp1_mult: p => p.tp1_mult,
  smc_tp2_mult: p => p.tp2_mult,
  smc_tp3_mode: p => p.tp3_mode,
  smc_tp3_rfixe: p => p.tp3_rfixe,
  smc_tp3_trailing: p => (p.tp3_trailing ? 1 : 0),
  smc_tp3_trailing_r: p => p.tp3_trailing_r,
  smc_frac_tp1: p => p.frac_tp1,
  smc_frac_tp2: p => p.frac_tp2,
  smc_frac_tp3: p => p.frac_tp3,
}
const modifie = computed(() =>
  (Object.keys(CLES) as string[]).some(k => String(CLES[k](params.value)) !== String(CLES[k](paramsActuels.value))))

async function chargerVecu() {
  vecu.value = await chargerAnalyse('SMC')
  try {
    const c = await http.get<{ capital_depart: number; points: { capital_apres: number }[] }>('/api/strategies/SMC/capital')
    creuxVecu.value = c.data.points.reduce((m, p) => Math.min(m, p.capital_apres), c.data.capital_depart)
  } catch { creuxVecu.value = vecu.value?.capital_depart ?? 0 }
}

async function chargerReglages() {
  const entrees = await Promise.all(Object.keys(CLES).map(async cle => {
    try {
      const r = await http.get<{ cle: string; valeur: string | null }>(`/api/config`, { params: { cle } })
      return [cle, r.data.valeur] as const
    } catch { return [cle, null] as const }
  }))
  const p: ParamsSmc = { ...DEFAUTS }
  for (const [cle, valeur] of entrees) {
    if (valeur === null || valeur === '') continue
    switch (cle) {
      case 'smc_tp1_mult': p.tp1_mult = Number(valeur); break
      case 'smc_tp2_mult': p.tp2_mult = Number(valeur); break
      case 'smc_tp3_mode': p.tp3_mode = valeur; break
      case 'smc_tp3_rfixe': p.tp3_rfixe = Number(valeur); break
      case 'smc_tp3_trailing': p.tp3_trailing = valeur === '1'; break
      case 'smc_tp3_trailing_r': p.tp3_trailing_r = Number(valeur); break
      case 'smc_frac_tp1': p.frac_tp1 = Number(valeur); break
      case 'smc_frac_tp2': p.frac_tp2 = Number(valeur); break
      case 'smc_frac_tp3': p.frac_tp3 = Number(valeur); break
    }
  }
  params.value = { ...p }
  paramsActuels.value = { ...p }
}

async function lancer() {
  enCours.value = true
  messageApplique.value = ''
  try {
    const r = await http.post<{ resultat: ResultatSim }>('/api/strategies/SMC/simulation',
      { ...params.value, assets: filtreAssets.value, tfs: filtreTfs.value }, { timeout: 150_000 })
    sim.value = r.data.resultat
    await chargerEssais()
  } catch (e) {
    sim.value = null
    alerteStore.afficherErreur(`Simulation : ${(e as Error).message}`)
  }
  enCours.value = false
}

async function balayer() {
  enCoursBalayage.value = true
  try {
    const r = await http.post<{ capital_depart: number; nb_clotures: number; configurations: LigneBalayage[] }>(
      '/api/strategies/SMC/simulation/balayage', { assets: filtreAssets.value, tfs: filtreTfs.value }, { timeout: 60_000 })
    balayage.value = r.data
  } catch (e) {
    balayage.value = null
    alerteStore.afficherErreur(`Balayage : ${(e as Error).message}`)
  }
  enCoursBalayage.value = false
}

async function balayerTrailing() {
  enCoursTrailing.value = true
  try {
    const r = await http.post<{ capital_depart: number; configurations: LigneTrailing[]; moteur_actuel: LigneTrailing | null }>(
      '/api/strategies/SMC/simulation/balayage', { cible: 'trailing', assets: filtreAssets.value, tfs: filtreTfs.value }, { timeout: 300_000 })
    balayageTrailingRes.value = r.data
  } catch (e) {
    balayageTrailingRes.value = null
    alerteStore.afficherErreur(`Balayage trailing : ${(e as Error).message}`)
  }
  enCoursTrailing.value = false
}

async function chargerEssais() {
  try {
    const r = await http.get<{ essais: Essai[] }>('/api/strategies/SMC/simulation/essais')
    essais.value = r.data.essais ?? []
  } catch { essais.value = [] }
}

async function supprimerEssai(id: string) {
  try { await http.delete(`/api/strategies/SMC/simulation/essais/${id}`) } catch { /* liste inchangée */ }
  await chargerEssais()
}

function rechargerParams(p: ParamsSmc) {
  params.value = { ...p }
  window.scrollTo({ top: 0, behavior: 'smooth' })
}

const lignesConfirmation = computed(() => {
  if (!modifie.value) return []
  const l: string[] = []
  for (const cle of Object.keys(CLES)) {
    const ancien = CLES[cle](paramsActuels.value)
    const nouveau = CLES[cle](params.value)
    if (String(ancien) !== String(nouveau)) {
      l.push(`  ${cle} : ${ancien} → ${nouveau}`)
    }
  }
  return l
})

function appliquer() {
  confirmationOuverte.value = true
}

async function executerApplication() {
  const clesModifiees = (Object.keys(CLES) as string[])
    .filter(k => String(CLES[k](params.value)) !== String(CLES[k](paramsActuels.value)))
  if (!clesModifiees.length) return
  const lignes = clesModifiees.map(k => `  ${k} : ${CLES[k](paramsActuels.value)} → ${CLES[k](params.value)}`).join('\n')
  confirmationOuverte.value = true
  return
  for (const cle of clesModifiees) {
    try {
      await http.post('/api/config', { cle, valeur: String(CLES[cle](params.value)) })
    } catch {
        messageApplique.value = '❌ Échec d\'écriture d\'un réglage — vérifiez le backend'
        return
      }
  }
  paramsActuels.value = { ...params.value }
  messageApplique.value = `✓ ${clesModifiees.length} réglage(s) appliqué(s) — effet sur les futurs signaux, le vécu reste immuable.`
}

/// Comparatif vécu vs simulation.
const comparatif = computed(() => {
  const v = vecu.value
  const s = sim.value
  if (!v || !s) return []
  const creuxSimPct = s.capital_depart > 0 ? s.capital_minimum / s.capital_depart - 1 : 0
  const simPct = s.capital_depart > 0 ? s.capital_actuel / s.capital_depart - 1 : 0
  return [
    {
      label: 'Capital final', vecu: fmtDollars(v.capital_actuel), sim: fmtDollars(s.capital_actuel),
      ecart: fmtPct(simPct - rendementVecu.value), classe: simPct >= rendementVecu.value ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Rendement', vecu: fmtPct(rendementVecu.value), sim: fmtPct(simPct),
      ecart: '', classe: simPct >= rendementVecu.value ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Pire creux', vecu: fmtDollars(creuxVecu.value), sim: fmtDollars(s.capital_minimum),
      ecart: fmtPct(creuxSimPct), classe: s.capital_minimum >= creuxVecu.value ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Clôtures', vecu: String(v.nb_trades), sim: String(s.nb_trades),
      ecart: String(s.nb_trades - v.nb_trades), classe: 'text-white',
    },
    {
      label: 'WR ($ > 0)', vecu: `${(v.taux_reussite * 100).toFixed(0)} %`, sim: `${(s.taux_reussite * 100).toFixed(0)} %`,
      ecart: `${((s.taux_reussite - v.taux_reussite) * 100).toFixed(0)} pts`, classe: s.taux_reussite >= v.taux_reussite ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Σ R encaissé (conversion — réglages)', vecu: fmtR2(v.r_total), sim: fmtR2(s.r_total_pondere),
      ecart: fmtR2(s.r_total_pondere - v.r_total), classe: s.r_total_pondere >= v.r_total ? 'text-emerald-400' : 'text-red-400',
    },
    {
      label: 'Σ R encaissé simulé (composé)', vecu: '—', sim: fmtR2(s.r_total_pondere),
      ecart: '', classe: s.r_total_pondere >= 0 ? 'text-emerald-400' : 'text-red-400',
    },
  ]
})

function fmtDollars(v: number): string {
  const n = Math.round(Math.abs(v)).toLocaleString('fr-FR')
  return `${v < 0 ? '−' : ''}${n} $`
}
function fmtPct(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v * 100).toFixed(1)} %`
}
function fmtR2(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2)} R`
}
function fmtFrac(p: ParamsSmc): string {
  return `${Math.round(p.frac_tp1 * 100)}/${Math.round(p.frac_tp2 * 100)}/${Math.round(p.frac_tp3 * 100)} %`
}
function dateCourte(ts: number): string {
  return new Date(ts * 1000).toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
}

onMounted(async () => {
  await Promise.all([chargerVecu(), chargerReglages(), chargerEssais(), chargerAssetsArmes()])
})

/// Paires disponibles = assets armés SMC (source : couples armés).
async function chargerAssetsArmes() {
  try {
    const r = await http.get<Record<string, string[]>>('/api/smc/couples')
    assetsDispo.value = Object.keys(r.data ?? {}).sort()
  } catch { assetsDispo.value = [] }
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.kpi        { @apply bg-white/5 rounded-lg p-2.5 border border-white/10 text-center; }
.kpi-label  { @apply text-[9px] uppercase tracking-wider text-white/60; }
.kpi-valeur { @apply text-lg font-bold font-mono mt-0.5; }
.kpi-sous   { @apply text-[10px] text-white/50 mt-0.5; }
.champ      { @apply bg-white/10 border border-white/15 rounded-lg px-2 py-1.5 text-white font-mono focus:border-teal-400/50 outline-none disabled:opacity-40; }
.btn-action { @apply text-xs px-3 py-1.5 rounded-lg font-semibold border border-white/10 transition-colors; }
</style>
