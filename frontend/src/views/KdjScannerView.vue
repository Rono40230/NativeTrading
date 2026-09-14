<template>
  <!-- 🔭 Scanner KDJ/Halftrend — gabarit Rockets (14/09, décision
       propriétaire : tableau dense en place des mini-graphiques). Classe
       l'univers par force de tendance (ADX Wilder 14, H1 = le TF du moteur,
       D1 = contexte) : le scanner est un SÉLECTEUR d'actifs — privilégier
       les tendances franches, ignorer les ranges (étude 7.E). -->
  <div class="flex flex-col gap-4 p-4 lg:p-6 h-full w-full overflow-hidden">
    <div class="flex items-center gap-3 shrink-0 flex-wrap">
      <RouterLink
        to="/kdj"
        class="text-[11px] px-2.5 py-1 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-colors whitespace-nowrap"
        title="Retour à la stratégie KDJ/Halftrend"
      >← KDJ/Halftrend</RouterLink>
      <h1 class="text-2xl font-bold text-white">🔭 Scanner KDJ/Halftrend</h1>
      <span class="text-white text-base hidden sm:inline">qui est en tendance, qui est en range — H1 partout</span>
      <span class="text-[10px] text-white/60 whitespace-nowrap"
        :title="`${nbFranches} actifs en tendance franche (ADX ≥ 25) sur ${lignes.length}`"
      >{{ nbFranches }} franche{{ nbFranches > 1 ? 's' : '' }} · {{ lignes.length }} actif{{ lignes.length > 1 ? 's' : '' }}</span>
      <div class="flex gap-1 ml-2">
        <button
          v-for="f in filtresType" :key="f.val"
          class="filtre-btn" :class="{ 'filtre-btn-actif': filtreType === f.val }"
          @click="filtreType = f.val"
        >{{ f.label }}</button>
      </div>
      <div class="flex gap-1 ml-auto">
        <button
          v-for="f in filtresEtat" :key="f.val"
          class="filtre-btn" :class="{ 'filtre-btn-actif': filtreEtat === f.val }"
          :title="f.titre"
          @click="filtreEtat = f.val"
        >{{ f.label }}</button>
      </div>
    </div>

    <p class="text-[11px] text-white/70 -mt-2 shrink-0">
      ADX(14) mesuré à la clôture H1 (moteur) et D1 (contexte) — seuils : ≥ 25 tendance
      franche · 20-25 modérée · &lt; 20 range. La colonne « Prêt » dit si toutes les
      conditions d'entrée sont réunies : il ne manque alors que la flèche HalfTrend.
      Actualisation toutes les 45 s.
    </p>

    <div class="flex-1 min-h-0 overflow-y-auto glass-card">
      <div v-if="chargement && !lignes.length" class="text-center text-white py-10 text-sm">Chargement…</div>
      <div v-else-if="!lignesAffichees.length" class="text-center text-white py-10 text-sm">
        Aucun actif dans ce filtre — univers : {{ lignes.length }} actif(s) collecté(s).
      </div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-white text-xs uppercase border-b border-white/10">
            <th class="px-3 py-2.5 text-left">#</th>
            <th class="px-3 py-2.5 text-left cursor-pointer select-none hover:text-white" @click="trierPar('asset')">Actif <span class="text-[9px]">{{ iconeTri('asset') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('type')">Type <span class="text-[9px]">{{ iconeTri('type') }}</span></th>
            <th class="px-3 py-2.5 text-right cursor-pointer select-none hover:text-white" @click="trierPar('adx_h1')" title="ADX(14) à la clôture H1 — le TF du moteur">ADX H1 <span class="text-[9px]">{{ iconeTri('adx_h1') }}</span></th>
            <th class="px-3 py-2.5 text-right cursor-pointer select-none hover:text-white" @click="trierPar('adx_d1')" title="ADX(14) à la clôture D1 — contexte de fond">ADX D1 <span class="text-[9px]">{{ iconeTri('adx_d1') }}</span></th>
            <th class="px-3 py-2.5 text-right cursor-pointer select-none hover:text-white" @click="trierPar('force')" title="Le plus élevé des deux ADX — le critère du verdict">Force <span class="text-[9px]">{{ iconeTri('force') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('direction')" title="SMA100 vs EMA200 en H1">Direction <span class="text-[9px]">{{ iconeTri('direction') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('au_dessus')" title="close vs EMA200 en H1">close/EMA <span class="text-[9px]">{{ iconeTri('au_dessus') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('j')" title="KDJ(20,7) à la clôture H1">K · D · J <span class="text-[9px]">{{ iconeTri('j') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('pret')" title="Conditions long/short toutes réunies — il ne manque que la flèche HalfTrend">Prêt <span class="text-[9px]">{{ iconeTri('pret') }}</span></th>
            <th class="px-3 py-2.5 text-center cursor-pointer select-none hover:text-white" @click="trierPar('verdict')">Verdict <span class="text-[9px]">{{ iconeTri('verdict') }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(l, i) in lignesAffichees" :key="l.asset" class="border-b border-white/5 hover:bg-white/5">
            <td class="px-3 py-2.5 text-white">{{ i + 1 }}</td>
            <td class="px-3 py-2.5 font-semibold text-white font-mono whitespace-nowrap">{{ l.asset }}</td>
            <td class="px-3 py-2.5 text-center">
              <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full border"
                :class="CLASSES_TYPE[l.type] ?? CLASSES_TYPE.autre">{{ LIBELLES_TYPE[l.type] ?? l.type }}</span>
            </td>
            <td class="px-3 py-2.5 text-right font-mono" :class="classeAdx(l.adx_h1)">{{ l.adx_h1.toFixed(1) }}</td>
            <td class="px-3 py-2.5 text-right font-mono" :class="classeAdx(l.adx_d1)">{{ Number.isFinite(l.adx_d1) ? l.adx_d1.toFixed(1) : '—' }}</td>
            <td class="px-3 py-2.5 text-right font-mono font-bold" :class="classeAdx(force(l))">{{ force(l).toFixed(1) }}</td>
            <td class="px-3 py-2.5 text-center" :class="l.direction === 'Hausse' ? 'text-emerald-400' : l.direction === 'Baisse' ? 'text-red-400' : 'text-white'">
              {{ l.direction === 'Hausse' ? '▲' : l.direction === 'Baisse' ? '▼' : '•' }} {{ l.direction }}
            </td>
            <td class="px-3 py-2.5 text-center text-white">{{ l.au_dessus_ema ? 'au-dessus' : 'en dessous' }}</td>
            <td class="px-3 py-2.5 text-center font-mono text-white whitespace-nowrap">
              <template v-if="l.kdj">
                {{ l.kdj[0].toFixed(0) }} · {{ l.kdj[1].toFixed(0) }} ·
                <span :class="l.kdj[2] > l.kdj[1] ? 'text-emerald-400' : 'text-red-400'">{{ l.kdj[2].toFixed(0) }} ({{ l.kdj[2] > l.kdj[1] ? 'J>D' : 'J<D' }})</span>
              </template>
              <template v-else>—</template>
            </td>
            <td class="px-3 py-2.5 text-center">
              <span v-if="pret(l)" class="text-[10px] font-semibold px-2 py-0.5 rounded-full border"
                :class="pret(l) === 'LONG'
                  ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
                  : 'bg-red-500/10 text-red-400 border-red-500/30'"
                :title="`Conditions ${pret(l)} réunies à la dernière clôture H1 — le moteur n'attend plus que la flèche HalfTrend`">{{ pret(l) }}</span>
              <span v-else class="text-[10px] text-white" title="Conditions incomplètes — pas d'entrée possible à la prochaine clôture">—</span>
            </td>
            <td class="px-3 py-2.5 text-center">
              <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full border" :class="classeVerdict(l.label)">{{ l.label }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'

interface Ligne {
  asset: string
  type: string
  adx_h1: number
  adx_d1: number
  direction: string
  au_dessus_ema: boolean
  label: string
  kdj: [number, number, number] | null
  sma_au_dessus: boolean
}

const lignes = ref<Ligne[]>([])
const chargement = ref(true)
let minuteur: ReturnType<typeof setInterval> | null = null

// ── Filtres type + état + tri par colonne (gabarit Rockets) ──
const filtreType = ref('')
const filtresType = [
  { val: '', label: 'Tous' },
  { val: 'metal', label: 'Métaux' },
  { val: 'crypto', label: 'Crypto' },
  { val: 'forex', label: 'Forex' },
  { val: 'indice', label: 'Indices' },
]
const filtreEtat = ref('')
const filtresEtat = [
  { val: '', label: 'Toutes', titre: 'Aucun filtre de tendance' },
  { val: 'franche', label: 'Franches', titre: 'ADX ≥ 25 (H1 ou D1) — les actifs que la stratégie aime' },
  { val: 'moderee', label: 'Modérées', titre: 'ADX 20-25 — à la limite' },
  { val: 'range', label: 'Ranges', titre: 'ADX < 20 — la stratégie y meurt (screening TV : EURUSD 0/3)' },
]
const triColonne = ref('force')
const triDir = ref<'asc' | 'desc'>('desc')

function trierPar(col: string) {
  if (triColonne.value === col) {
    triDir.value = triDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    triColonne.value = col
    triDir.value = 'desc'
  }
}
function iconeTri(col: string): string {
  if (triColonne.value !== col) return '\u21c5'
  return triDir.value === 'asc' ? '\u2191' : '\u2193'
}

const LIBELLES_TYPE: Record<string, string> = { metal: 'Métal', crypto: 'Crypto', forex: 'Forex', indice: 'Indice' }
const CLASSES_TYPE: Record<string, string> = {
  metal: 'bg-amber-500/10 text-amber-300 border-amber-500/30',
  crypto: 'bg-blue-500/10 text-blue-300 border-blue-500/30',
  forex: 'bg-violet-500/10 text-violet-300 border-violet-500/30',
  indice: 'bg-cyan-500/10 text-cyan-300 border-cyan-500/30',
  autre: 'bg-white/5 text-white border-white/10',
}

const force = (l: Ligne): number => Math.max(l.adx_h1, Number.isFinite(l.adx_d1) ? l.adx_d1 : -1)
const nbFranches = computed(() => lignes.value.filter((l) => l.label === 'Tendance franche').length)

/// Conditions long/short toutes réunies à la dernière clôture (le moteur
/// n'attend plus que la flèche du retournement HalfTrend).
function pret(l: Ligne): string {
  if (!l.kdj) return ''
  const longOk = l.sma_au_dessus && l.au_dessus_ema && l.kdj[2] > l.kdj[1]
  const shortOk = !l.sma_au_dessus && !l.au_dessus_ema && l.kdj[2] < l.kdj[1]
  return longOk ? 'LONG' : shortOk ? 'SHORT' : ''
}

function valeurTri(l: Ligne, col: string): string | number | boolean {
  switch (col) {
    case 'asset': return l.asset.toLowerCase()
    case 'type': return l.type
    case 'adx_h1': return l.adx_h1
    case 'adx_d1': return Number.isFinite(l.adx_d1) ? l.adx_d1 : -1
    case 'force': return force(l)
    case 'direction': return l.direction
    case 'au_dessus': return l.au_dessus_ema
    case 'j': return l.kdj ? l.kdj[2] - l.kdj[1] : -999
    case 'pret': return pret(l) !== ''
    case 'verdict': return l.label
    default: return ''
  }
}

const lignesAffichees = computed(() => {
  const etat = (l: Ligne) =>
    filtreEtat.value === ''
      ? true
      : filtreEtat.value === 'franche' ? l.label === 'Tendance franche'
      : filtreEtat.value === 'moderee' ? l.label === 'Modérée'
      : l.label === 'Range'
  const liste = lignes.value.filter((l) => (!filtreType.value || l.type === filtreType.value) && etat(l))
  const col = triColonne.value
  return [...liste].sort((a, b) => {
    const va = valeurTri(a, col)
    const vb = valeurTri(b, col)
    const cmp = va < vb ? -1 : va > vb ? 1 : 0
    return triDir.value === 'asc' ? cmp : -cmp
  })
})

function classeAdx(v: number): string {
  if (!Number.isFinite(v)) return 'text-white'
  if (v >= 25) return 'text-emerald-400'
  if (v >= 20) return 'text-amber-300'
  return 'text-white'
}
function classeVerdict(label: string): string {
  if (label === 'Tendance franche') return 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
  if (label === 'Modérée') return 'bg-amber-500/10 text-amber-400 border-amber-500/30'
  if (label === 'Range') return 'bg-gray-500/10 text-white border-white/10'
  return 'bg-white/5 text-white border-white/10'
}

async function charger() {
  chargement.value = true
  try {
    const res = await http.get<Ligne[]>('/api/kdj/scanner')
    lignes.value = res.data
  } catch { lignes.value = [] }
  chargement.value = false
}

onMounted(() => {
  void charger()
  minuteur = setInterval(() => void charger(), 45_000)
})
onUnmounted(() => { if (minuteur) clearInterval(minuteur) })
</script>

<style scoped>
.filtre-btn {
  padding: 0.25rem 0.75rem;
  border-radius: 0.5rem;
  font-size: 0.75rem;
  color: #ffffff;
  background: rgba(255, 255, 255, 0.05);
  transition: all 0.15s ease;
}
.filtre-btn:hover { color: #fff; background: rgba(255, 255, 255, 0.1); }
.filtre-btn-actif {
  color: #fff;
  background: rgba(34, 211, 238, 0.25);
}
</style>
