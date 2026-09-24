<template>
  <div class="glass-card px-4 py-2 flex flex-col gap-1.5">
    <!-- En-tête cliquable repliable (même motif que Créneaux de volatilité) —
         remplace la page /heatmap et le bouton ⚡ (décision 22/09). -->
    <div class="flex items-center justify-between shrink-0 gap-2 cursor-pointer select-none" @click="basculer()">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">
        <span class="inline-block transition-transform" :class="ouvert ? 'rotate-90' : ''">▸</span>
        🌡️ Radar ATR temps réel
        <span v-if="!ouvert && classement.length" class="text-white font-normal normal-case tracking-normal">
          · top {{ classement[0].asset }} {{ classement[0].tf }}
        </span>
      </p>
      <span class="text-[9px] text-white shrink-0">MAJ 60 s</span>
    </div>

    <template v-if="ouvert">
      <!-- Légende des teintes — la mesure est corrigée de la saisonnalité
           jour × heure (6.6, 24/09) : 100 % = conforme à l'habitude de CET
           instant, pas à une moyenne a-saisonnière. H4 et + : l'heure n'y a
           pas de sens (fenêtres ≥ 24 h) → ratio brut vs 60 bougies. -->
      <div class="flex items-center gap-2 flex-wrap text-[9px] text-white">
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#10b981" /> calme &lt;80</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#f59e0b" /> modéré</span>
        <span class="flex items-center gap-1"><span class="w-3 h-2 rounded-[2px]" style="background:#ef4444" /> élevé &gt;120</span>
        <span class="text-white/50">vs habitude de cet instant (jour × heure, 24 mois)</span>
      </div>

      <!-- Anomalies saisonnières : volatilité anormale POUR CET INSTANT —
           remplace l'ancienne confluence « ratio brut chaud × slot
           habituellement chaud » qui tirait surtout sur les ouvertures de
           session (constat propriétaire 23/09). -->
      <div v-if="confluences.length" class="rounded-lg border border-orange-500/40 bg-orange-500/10 px-2 py-1.5 flex flex-wrap gap-1.5 items-center">
        <span class="text-[9px] text-orange-200/80 font-semibold uppercase tracking-wider">Anomalies</span>
        <span
          v-for="c in confluences" :key="c.asset + c.tf"
          class="flex items-center gap-1 bg-orange-500/15 border border-orange-500/30 rounded px-1.5 py-0.5 text-[9px]"
          :title="`${c.asset} ${c.tf} : ${c.atr.toFixed(0)} % de l'habitude de cet instant (brut ${c.brut.toFixed(0)} %)`"
        >
          <span class="font-bold text-white">{{ c.asset }}</span>
          <span class="text-white bg-white/10 px-1 rounded font-mono">{{ c.tf }}</span>
          <span class="text-orange-300 font-mono">{{ c.atr.toFixed(0) }}%</span>
        </span>
      </div>

      <div v-if="chargement" class="text-center text-white text-xs py-3">Calcul…</div>
      <div v-else-if="!classement.length" class="text-center text-white text-xs py-3">Aucune donnée</div>

      <!-- Classement : les cellules les plus volatiles d'abord (le tableau
           complet asset×TF ne tenait pas dans la colonne latérale). -->
      <div v-else class="flex flex-col gap-1 max-h-[38vh] overflow-y-auto pr-0.5">
        <div
          v-for="i in classement.slice(0, 12)" :key="i.cle"
          class="rounded-lg px-2 py-1 flex items-center gap-1.5 border"
          :style="ligneStyle(i.atr)"
          :title="titreLigne(i)"
        >
          <span class="text-[11px] font-bold text-white">{{ i.asset }}</span>
          <span class="text-[9px] text-white bg-white/10 px-1 rounded font-mono">{{ i.tf }}</span>
          <span class="ml-auto text-[10px] font-mono text-white">{{ i.atr.toFixed(0) }} %</span>
          <span class="text-[9px] text-white/80">{{ libelle(i.atr) }}</span>
        </div>
        <p class="text-[9px] text-white/40 pt-0.5">H4/D1/W1 : % de la moyenne des 60 dernières bougies (l'heure y est sans objet).</p>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { apiService } from '@/services/api.service'
import type { Candle, ReponsePatternsVolatilite } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'

/// Radar ATR temps réel en bloc repliable du dashboard (ex-page /heatmap,
/// décision 22/09) : volatilité actuelle vs HABITUDE DE L'INSTANT.
/// 6.6 (24/09) : le ratio ATR(6)/ATR(60) brut mesurait en partie l'heure de
/// la session (ouverture NY gonflée par construction, Asie écrasée) — il est
/// divisé par le facteur saisonnier jour × heure de l'asset (patterns 24
/// mois, M1, même source que les Créneaux). 100 % = conforme à l'habitude
/// de CET instant. H4 et + : l'heure n'a pas de sens (fenêtres ≥ 24 h) →
/// ratio brut. Chargé UNIQUEMENT ouvert, rafraîchi toutes les 60 s.
const TIMEFRAMES = ['M1', 'M5', 'M15', 'M30', 'H1', 'H4', 'D1', 'W1']
const INTRADAY = new Set(['M1', 'M5', 'M15', 'M30', 'H1'])

interface LigneRadar { cle: string; asset: string; tf: string; atr: number; brut: number }

const assetsStore = useAssetsStore()
const ouvert = ref(false)
const chargement = ref(false)
const donnees = ref<Record<string, LigneRadar>>({})
const assets = computed(() => assetsStore.assets.map(a => a.id))

/// Baselines saisonnières par asset : ATR moyen par cellule jour × heure
/// (UTC — sémantique des patterns), moyenne globale pondérée, repli
/// heure-semaine puis moyenne si la cellule est absente.
interface BaseAsset { moyenne: number; parJourHeure: Map<string, number>; parHeure: Map<number, number> }
const baselines = ref<Map<string, BaseAsset>>(new Map())

function construireBaselines(reponses: ReponsePatternsVolatilite[]) {
  const m = new Map<string, BaseAsset>()
  for (const r of reponses) {
    let sp = 0, sn = 0
    const parJourHeure = new Map<string, number>()
    const accHeure = new Map<number, { somme: number; n: number }>()
    for (const p of r.patterns) {
      if (p.nb_points <= 0 || p.atr_moyen <= 0) continue
      sp += p.atr_moyen * p.nb_points
      sn += p.nb_points
      parJourHeure.set(`${p.jour_semaine}_${p.heure}`, p.atr_moyen)
      const e = accHeure.get(p.heure) ?? { somme: 0, n: 0 }
      e.somme += p.atr_moyen * p.nb_points
      e.n += p.nb_points
      accHeure.set(p.heure, e)
    }
    if (sn > 0) {
      const parHeure = new Map<number, number>()
      for (const [h, e] of accHeure) parHeure.set(h, e.somme / e.n)
      m.set(r.asset, { moyenne: sp / sn, parJourHeure, parHeure })
    }
  }
  baselines.value = m
}

/// Facteur saisonnier de l'instant : 1,15 = cet asset est habituellement
/// 15 % plus volatile à ce jour × heure que sa moyenne globale. Absent de
/// la base → 1 (aucune correction, ratio brut).
function facteurSaisonnier(asset: string): number {
  const b = baselines.value.get(asset)
  if (!b || b.moyenne <= 0) return 1
  const now = new Date()
  const base = b.parJourHeure.get(`${now.getUTCDay()}_${now.getUTCHours()}`)
    ?? b.parHeure.get(now.getUTCHours())
    ?? b.moyenne
  return base / b.moyenne
}

const classement = computed(() =>
  Object.values(donnees.value).filter(i => i.atr > 0).sort((a, b) => b.atr - a.atr)
)

/// Anomalies saisonnières : intraday seulement, ≥ 120 % de l'habitude de
/// l'instant — calcul pur, plus aucun aller-retour de patterns par asset.
const confluences = computed(() =>
  classement.value.filter(i => i.atr >= 120 && INTRADAY.has(i.tf)).slice(0, 6)
)

function basculer() {
  ouvert.value = !ouvert.value
}

function libelle(ratio: number): string {
  if (ratio < 80) return 'calme'
  if (ratio < 120) return 'modéré'
  return 'élevé'
}

function ligneStyle(ratio: number) {
  const couleur = ratio < 80 ? '#10b981' : ratio < 120 ? '#f59e0b' : '#ef4444'
  return {
    background: `${couleur}20`,
    borderColor: ratio >= 120 ? '#ef4444' : `${couleur}55`,
  }
}

function titreLigne(i: LigneRadar): string {
  return INTRADAY.has(i.tf)
    ? `ATR ${i.asset} ${i.tf} : ${i.atr.toFixed(0)} % de l'habitude de cet instant (brut ${i.brut.toFixed(0)} %)`
    : `ATR ${i.asset} ${i.tf} : ${i.atr.toFixed(0)} % de la moyenne des 60 dernières bougies`
}

function calcAtr(candles: Candle[], periode = 14): number {
  if (candles.length < 2) return 0
  const trs = candles.slice(1).map((c, i) => {
    const prev = candles[i].close
    return Math.max(c.high - c.low, Math.abs(c.high - prev), Math.abs(c.low - prev))
  })
  const fenetre = trs.slice(-Math.min(periode, trs.length))
  return fenetre.reduce((s, v) => s + v, 0) / fenetre.length
}

/// Ratio brut (ATR 6 dernières / ATR 60 dernières, ×100).
function calcAtrRatio(candles: Candle[]): number {
  if (candles.length < 30) return 0
  const atrActuel = calcAtr(candles.slice(-7), 6)
  const atrMoyen = calcAtr(candles, Math.min(candles.length - 1, 60))
  return atrMoyen > 0 ? (atrActuel / atrMoyen) * 100 : 100
}

async function actualiser() {
  chargement.value = true
  // Baselines saisonnières (cache serveur 1 h — une requête par cycle).
  try {
    construireBaselines(await apiService.obtenirPatternsJourTousActifs())
  } catch {
    baselines.value = new Map()
  }
  const paires = assets.value.flatMap(a => TIMEFRAMES.map(tf => ({ a, tf })))
  const resultats = await Promise.allSettled(
    paires.map(({ a, tf }) => apiService.getCandles(a, tf, 80).then(c => ({ a, tf, c })))
  )
  for (const r of resultats) {
    if (r.status === 'fulfilled') {
      const { a, tf, c } = r.value
      const brut = calcAtrRatio(c)
      if (brut <= 0) continue
      const facteur = facteurSaisonnier(a)
      donnees.value[`${a}_${tf}`] = {
        cle: `${a}_${tf}`, asset: a, tf,
        atr: INTRADAY.has(tf) ? brut / facteur : brut,
        brut,
      }
    }
  }
  chargement.value = false
}

/// Le cycle 60 s ne vit que pendant l'ouverture — replié, le bloc ne coûte rien.
let intervalId: ReturnType<typeof setInterval> | null = null
watch(ouvert, async (ouvertMaintenant) => {
  if (ouvertMaintenant) {
    await actualiser()
    intervalId = setInterval(actualiser, 60_000)
  } else if (intervalId) {
    clearInterval(intervalId)
    intervalId = null
  }
})
onMounted(() => { if (!assetsStore.assets.length) void assetsStore.chargerAssets() })
onUnmounted(() => { if (intervalId) clearInterval(intervalId) })
</script>
