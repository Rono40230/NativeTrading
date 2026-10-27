<template>
  <div class="absolute top-3 left-3 z-10 select-none" style="min-width: 340px;">
    <div class="bg-[#0a0e27]/90 border border-white/10 rounded-xl overflow-hidden shadow-2xl backdrop-blur-sm">
      <!-- Entête — une pastille par colonne, alignée sur les largeurs
           du corps (w-8 / flex-1 / w-4 / w-[74px] / w-4), owner 27/10. -->
      <div class="flex items-center px-3 py-1 border-b border-white/10">
        <span class="w-8 shrink-0 text-[9px] font-semibold text-white/70 uppercase">TF</span>
        <span class="flex-1 shrink-0 text-[9px] font-semibold text-white/70 uppercase" title="Tendance EMA vivantes">Tendance</span>
        <span class="w-[70px] shrink-0 text-center text-[9px] font-semibold text-white/70" title="État de structure (moteur v12 : séquence HH/HL vs LH/LL)">Etat structure</span>
        <span class="w-[94px] shrink-0 text-center text-[9px] font-semibold text-white/70" title="Dernier événement de structure (BOS/MSS/CHoCH/sweep)">Dernier événement</span>
        <span class="w-[50px] shrink-0 text-center text-[9px] font-semibold text-white/70" title="Premium / Discount (P au-dessus de l'équilibre, D en dessous)">Prem/Disc</span>
      </div>
      <!-- Chargement -->
      <div v-if="chargement" class="px-3 py-3 flex items-center gap-2">
        <span class="w-3 h-3 rounded-full border-2 border-blue-400 border-t-transparent animate-spin" />
        <span class="text-[10px] text-white">Calcul…</span>
      </div>
      <div v-else-if="erreur" class="px-3 py-2 text-[10px] text-red-400">
        ⚠ {{ erreur }}
      </div>
      <div v-else>
        <div
          v-for="ligne in lignes"
          :key="ligne.tf"
          class="flex items-center px-3 py-1 border-b border-white/5 last:border-0 transition-colors"
          :class="classeLigne(ligne)"
        >
          <!-- Colonne TF -->
          <span
            class="text-[11px] font-mono w-8 shrink-0"
            :class="ligne.tf === tfActifTable ? 'text-blue-300 font-bold' : 'text-white'"
          >{{ ligne.tf }}</span>
          <!-- Colonne direction — conteneur flex-1 : LA GRILLE de la ligne
               épouse celle de l'en-tête (justify-between répartissait
               l'espace dans les inter-colonnes et décalait tout, owner 27/10).
               Kasper (espion 15/10) : mot + emoji, texte blanc sur fond
               coloré par direction — pas de zone neutre, réaction temps
               réel sur la bougie en cours. -->
          <div class="flex-1 min-w-0">
            <div v-if="ligne.tendance === 'haussier'" class="flex items-center gap-1">
              <span class="text-[10px] font-semibold text-white">Haussier</span>
              <span class="text-[11px]">🚀</span>
            </div>
            <div v-else-if="ligne.tendance === 'baissier'" class="flex items-center gap-1">
              <span class="text-[10px] font-semibold text-white">Baissier</span>
              <span class="text-[11px]">🐻</span>
            </div>
            <span v-else class="text-[10px] text-white">—</span>
          </div>
          <!-- STRUCT P1 (owner 27/10) — colonnes structure, compactes :
               état + dernier événement + premium/discount. Les colonnes
               tendance ci-dessus restent INTACTES (validées 97,6 %). -->
          <span class="w-[70px] text-center text-[11px] shrink-0" :title="titreStructure(ligne.tf)">{{ emojiStructure(ligne.tf) }}</span>
          <span class="w-[94px] text-center font-mono text-[9px] text-white/80 shrink-0" :title="titreEvenement(ligne.tf)">{{ evenementCompact(ligne.tf) }}</span>
          <span class="w-[50px] text-center text-[9px] font-bold shrink-0" :class="pdClasse(ligne.tf)">{{ pdCourt(ligne.tf) }}</span>
        </div>
        <!-- Ligne W1 : structure seule (tendance retirée de la table Kasper
             par règle owner 15/10 — la structure hebdo est une info ICT). -->
        <div v-if="structureParTf['W1']" class="flex items-center px-3 py-1 border-b border-white/5 transition-colors hover:bg-white/5">
          <span class="text-[11px] font-mono w-8 shrink-0 text-white">W1</span>
          <span class="flex-1 min-w-0 text-[10px] text-white">—</span>
          <span class="w-[70px] text-center text-[11px] shrink-0" :title="titreStructure('W1')">{{ emojiStructure('W1') }}</span>
          <span class="w-[94px] text-center font-mono text-[9px] text-white/80 shrink-0" :title="titreEvenement('W1')">{{ evenementCompact('W1') }}</span>
          <span class="w-[50px] text-center text-[9px] font-bold shrink-0" :class="pdClasse('W1')">{{ pdCourt('W1') }}</span>
        </div>
        <!-- Verdict MTF (STRUCT P1, refonte owner 27/10) : décision lisible
             en 1re ligne (intention, jamais un ordre), le pourquoi factuel
             en 2e. Couleur = où l'entonnoir direction→zone→confirmation
             s'arrête. -->
        <div v-if="verdictData" class="px-3 py-1.5 border-t border-white/10 bg-white/[0.04] leading-tight">
          <div class="text-[10px] font-bold" :class="classeNiveau">{{ verdictData.action }}</div>
          <div class="mt-0.5 text-[9px] text-cyan-200/80">{{ verdictData.texte }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from 'vue'
import { apiService } from '@/services/api.service'
import { http } from '@/services/http.client'
import type { LigneTendanceKasper } from '@/services/api.types'

const props = defineProps<{
  asset: string
  timeframe: string
  periodeRapide: number
  periodeLente: number
  modeCalcul: 'bougie_cloturee' | 'bougie_en_cours'
}>()

const lignes = ref<LigneTendanceKasper[]>([])

// ── STRUCT P1 : structure de marché par TF (moteur v12, endpoint agrégé) ──
interface EvenementStructure { genre: string; dir: string; level: number; ts: number }
interface LigneStructure {
  tf: string; etat: string
  dernier_evenement: EvenementStructure | null
  dernier_pivot: { ptype: string; price: number; ts: number } | null
  in_premium: boolean; in_discount: boolean
  bsl: number | null; ssl: number | null
}
const structureParTf = ref<Record<string, LigneStructure>>({})
const verdictData = ref<{ action: string; niveau: string; texte: string } | null>(null)

async function chargerStructure() {
  try {
    const res = await http.get('/api/smc/structure-mtf', { params: { asset: props.asset }, timeout: 20000 })
    const carte: Record<string, LigneStructure> = {}
    for (const l of res.data?.lignes ?? []) carte[l.tf] = l
    structureParTf.value = carte
    const v = res.data?.verdict
    verdictData.value = v ? { action: v.action, niveau: v.niveau, texte: v.texte } : null
  } catch {
    // structure indisponible : la table tendance reste pleinement lisible
  }
}

/// Couleurs du verdict — l'œil apprend l'échelle fermée backend :
/// plein vert/rouge = setups actifs, ambre = je sais quoi attendre,
/// orange = transition, gris = rien à faire.
const CLASSES_NIVEAU: Record<string, string> = {
  actif_haussier: 'text-emerald-400',
  actif_baissier: 'text-red-400',
  attente_achat: 'text-amber-300',
  attente_vente: 'text-amber-300',
  transition: 'text-orange-400',
  neutre: 'text-white/60',
}
const classeNiveau = computed(() =>
  (verdictData.value && CLASSES_NIVEAU[verdictData.value.niveau]) || 'text-cyan-200')

/// Libellés tendance (« 15m », « 1H ») → clés structure (« M15 », « H1 »).
const CLE_STRUCTURE: Record<string, string> = {
  '1m': 'M1', '5m': 'M5', '15m': 'M15', '30m': 'M30',
  '1H': 'H1', '4H': 'H4', '1D': 'D1', W1: 'W1',
}
function ligneStructure(tf: string): LigneStructure | undefined {
  return structureParTf.value[CLE_STRUCTURE[tf] ?? tf]
}

function emojiStructure(tf: string): string {
  const l = ligneStructure(tf)
  if (!l) return '·'
  if (l.etat === 'haussiere') return '🟢'
  if (l.etat === 'baissiere') return '🔴'
  return '⚪'
}

/// Compact : « MSS↓ 09:00 » (heure Paris) ; âge relatif pour les TF hauts.
function evenementCompact(tf: string): string {
  const e = ligneStructure(tf)?.dernier_evenement
  if (!e) return '—'
  const fleche = e.dir === 'bull' ? '↑' : '↓'
  const d = new Date(e.ts * 1000)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${e.genre}${fleche} ${p(d.getDate())}/${p(d.getMonth() + 1)}`
}

function titreStructure(tf: string): string {
  const l = ligneStructure(tf)
  if (!l) return 'Structure : non calculée sur ce TF'
  const pivot = l.dernier_pivot ? ` · dernier pivot ${l.dernier_pivot.ptype} ${l.dernier_pivot.price.toFixed(2)}` : ''
  return `Structure ${l.etat}${pivot}`
}

function titreEvenement(tf: string): string {
  const l = structureParTf.value[tf]
  const e = l?.dernier_evenement
  if (!e) return 'Aucun événement de structure (BOS/MSS/CHoCH/sweep)'
  const cible = l?.bsl != null ? ` · BSL ${l.bsl.toFixed(2)}` : ''
  const sol = l?.ssl != null ? ` · SSL ${l.ssl.toFixed(2)}` : ''
  return `${e.genre} ${e.dir === 'bull' ? 'haussier' : 'baissier'} à ${e.level.toFixed(2)}${cible}${sol}`
}

function pdCourt(tf: string): string {
  const l = ligneStructure(tf)
  if (!l) return '·'
  if (l.in_discount) return 'D'
  if (l.in_premium) return 'P'
  return '·'
}

function pdClasse(tf: string): string {
  const l = ligneStructure(tf)
  if (!l) return 'text-white/30'
  if (l.in_discount) return 'text-emerald-400'
  if (l.in_premium) return 'text-amber-400'
  return 'text-white/30'
}

/// Fond de ligne par direction (retour owner 15/10 : les fonds verts/rouges
/// de Kasper) — le TF actif garde son repère bleu en plus.
function classeLigne(ligne: LigneTendanceKasper): string {
  const fond =
    ligne.tendance === 'haussier' ? 'bg-emerald-500/20'
    : ligne.tendance === 'baissier' ? 'bg-red-500/20'
    : 'hover:bg-white/5'
  const repere = ligne.tf === tfActifTable.value ? ' border-l-2 border-l-blue-400' : ''
  return fond + repere
}
const chargement = ref(false)
const erreur = ref<string | null>(null)

// Format chart (M1, H1…) → format API (1m, 1H…)
const tfActifTable = computed(() => {
  const map: Record<string, string> = {
    M1: '1m', M5: '5m', M15: '15m', M30: '30m',
    H1: '1H', H4: '4H', D1: '1D',
  }
  return map[props.timeframe] ?? ''
})

async function charger() {
  chargement.value = true
  erreur.value = null
  try {
    const data = await apiService.obtenirTendanceMultiTf(
      props.asset,
      props.periodeRapide,
      props.periodeLente,
      props.modeCalcul
    )
    lignes.value = data.lignes
  } catch (err: unknown) {
    erreur.value = err instanceof Error ? err.message : 'Erreur réseau'
  } finally {
    chargement.value = false
  }
}

onMounted(() => {
  charger()
  chargerStructure()
  // Miroir du panneau Kasper : il réévalue ses EMA vivantes en continu —
  // notre panneau se rafraîchit toutes les 10 s (espion 15/10). La
  // structure vit moins vite (cache serveur 60 s) : son poll suit.
  poll = setInterval(charger, 10_000)
  pollStructure = setInterval(chargerStructure, 60_000)
})
let poll: ReturnType<typeof setInterval> | null = null
let pollStructure: ReturnType<typeof setInterval> | null = null
onUnmounted(() => {
  if (poll !== null) clearInterval(poll)
  if (pollStructure !== null) clearInterval(pollStructure)
})

watch(
  () => [props.asset, props.timeframe, props.periodeRapide, props.periodeLente, props.modeCalcul],
  charger
)
</script>
