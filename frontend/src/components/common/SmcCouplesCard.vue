<template>
  <!-- Timeframes par asset (SMC) — armement de la GÉNÉRATION par couple.
       H1 n'est plus générateur (étude 24 mois : 0,022 R/trade, WR 23 %) mais
       reste collecté pour l'amorce MTF. Straddle indépendant (rail M1). -->
  <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl flex flex-col overflow-hidden shadow-lg relative">
    <div class="absolute top-0 left-0 w-full h-1" style="background: linear-gradient(90deg, #2196F366, transparent)"></div>

    <div class="p-5 border-b border-white/5 flex items-center justify-between pb-4">
      <h3 class="font-bold text-base flex items-center gap-2">
        <span class="w-8 h-8 rounded-full flex items-center justify-center bg-blue-500/10 text-blue-400">📐</span>
        TIMEFRAMES PAR ASSET
      </h3>
      <span class="text-[10px] text-white" title="Les métriques (carte, capital, analyses) suivent le périmètre armé — le re-jeu est relancé à l'enregistrement.">SMC · génération de signaux</span>
    </div>

    <div class="p-5 flex-1 space-y-4">
      <!-- Grille assets × TF -->
      <div v-if="assets.length" class="overflow-x-auto">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-white">
              <th class="text-left font-semibold pb-2 pr-3">Asset</th>
              <th v-for="t in tfs" :key="t" class="pb-2 px-1.5 text-center font-semibold cursor-help" :title="titreTf(t)">{{ t }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="a in assets" :key="a" class="border-t border-white/5">
              <td class="py-1.5 pr-3 font-mono font-semibold text-white">{{ a }}</td>
              <td v-for="t in tfs" :key="a + t" class="py-1.5 px-1.5 text-center">
                <button
                  class="px-2 py-0.5 rounded-full font-mono text-[10px] font-bold transition-colors"
                  :class="estArme(a, t)
                    ? 'bg-blue-500/60 text-white hover:bg-blue-500/40'
                    : 'bg-white/10 text-white/40 hover:bg-white/20'"
                  :title="estArme(a, t)
                    ? `${a} ${t} génère des signaux SMC — cliquer pour désarmer`
                    : `${a} ${t} désarmé — cliquer pour armer`"
                  @click="basculer(a, t)"
                >{{ estArme(a, t) ? '●' : '○' }}</button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <p v-else class="text-xs text-white py-4 text-center">Périmètre indisponible — réessayez dans un instant.</p>
    </div>

    <!-- Action : même design que la carte STRATÉGIE SMC -->
    <div class="p-5 mt-auto bg-black/10 border-t border-white/5">
      <div class="flex items-center justify-between">
        <span v-if="message" class="text-xs mr-2 transition-opacity" :class="message.ok ? 'text-emerald-400' : 'text-red-400'">
          {{ message.texte }}
        </span>
        <span v-else class="text-xs mr-2 text-transparent">Sp</span>
        <button @click="enregistrer" :disabled="enCours"
          class="px-4 py-2 w-full max-w-[140px] bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-all shadow-lg hover:shadow-blue-500/20 active:scale-95 disabled:opacity-50">
          {{ enCours ? '...' : 'Enregistrer' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { http } from '@/services/http.client'

const tfs = ref<string[]>([])
const assets = ref<string[]>([])
/** Armement effectif serveur (référence) et copie locale (édition). */
const armesServeur = ref<Record<string, string[]>>({})
const armesLocal = ref<Record<string, string[]>>({})
const enCours = ref(false)
const message = ref<{ ok: boolean; texte: string } | null>(null)

/// Chiffres du comparatif 24 mois (04/09) — R pondéré par trade et WR par TF.
const ETUDE: Record<string, { r: string; wr: string }> = {
  M1: { r: '+0,022 R/trade', wr: '63 %' },
  M5: { r: '+0,020 R/trade', wr: '53 %' },
  M15: { r: '+0,051 R/trade (meilleur TF par mise)', wr: '41 %' },
  M30: { r: '+0,045 R/trade', wr: '34 %' },
}

function titreTf(tf: string): string {
  const e = ETUDE[tf]
  return e ? `${tf} — étude 24 mois : ${e.r} · WR ${e.wr}` : tf
}

function estArme(asset: string, tf: string): boolean {
  return (armesLocal.value[asset] ?? []).includes(tf)
}

function basculer(asset: string, tf: string) {
  const actuel = new Set(armesLocal.value[asset] ?? [])
  if (actuel.has(tf)) actuel.delete(tf)
  else actuel.add(tf)
  armesLocal.value = { ...armesLocal.value, [asset]: [...actuel] }
}

const modifie = computed(() => JSON.stringify(tri(armesLocal.value)) !== JSON.stringify(tri(armesServeur.value)))

function tri(m: Record<string, string[]>): Record<string, string[]> {
  const out: Record<string, string[]> = {}
  for (const k of Object.keys(m).sort()) out[k] = [...m[k]].sort()
  return out
}

async function charger() {
  try {
    const res = await http.get<{ tfs: string[]; assets: string[]; armes: Record<string, string[]> }>('/api/smc/couples')
    tfs.value = res.data.tfs
    assets.value = res.data.assets
    armesServeur.value = res.data.armes
    armesLocal.value = JSON.parse(JSON.stringify(res.data.armes))
  } catch {
    assets.value = []
  }
}

async function enregistrer() {
  if (!modifie.value) {
    message.value = { ok: false, texte: 'Aucun changement' }
    setTimeout(() => message.value = null, 4000)
    return
  }
  enCours.value = true
  message.value = null
  try {
    await http.put('/api/smc/couples', armesLocal.value)
    armesServeur.value = JSON.parse(JSON.stringify(armesLocal.value))
    message.value = { ok: true, texte: 'Sauvegardé ✓ (effet ≤ 60 s)' }
  } catch (err: any) {
    message.value = { ok: false, texte: err?.response?.data?.erreur ?? err?.response?.data?.error ?? 'Erreur inconnue' }
  } finally {
    enCours.value = false
  }
}

onMounted(charger)
</script>
