<template>
  <!-- Bloc TIMEFRAMES PAR ASSET (SMC) — armement de la GÉNÉRATION par couple.
       Refonte de l'ex-SmcCouplesCard (09/09, workflow dashboard) : vit dans
       la modale « Choix des Timeframe/Asset » de la carte SMC — le cadre est
       fourni par la modale, ici le tableau + l'action.
       H1 n'est plus générateur (étude 24 mois : 0,022 R/trade, WR 23 %) mais
       reste collecté pour l'amorce MTF. Straddle indépendant (rail M1). -->
  <div class="flex flex-col gap-3">
    <p class="text-[10px] text-white" title="Les métriques (carte, capital, analyses) suivent le périmètre armé — le re-jeu est relancé à l'enregistrement.">
      SMC · génération de signaux — les métriques suivent le périmètre armé
    </p>

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

    <div class="flex items-center justify-between pt-1 border-t border-white/5">
      <span v-if="message" class="text-xs mr-2" :class="message.ok ? 'text-emerald-400' : 'text-red-400'">
        {{ message.texte }}
      </span>
      <span v-else class="text-xs mr-2 text-transparent">Sp</span>
      <button @click="enregistrer" :disabled="enCours"
        class="px-4 py-2 w-full max-w-[140px] bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-all shadow-lg hover:shadow-blue-500/20 active:scale-95 disabled:opacity-50">
        {{ enCours ? '...' : 'Enregistrer' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
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
  const modifie = JSON.stringify(tri(armesLocal.value)) !== JSON.stringify(tri(armesServeur.value))
  if (!modifie) {
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
  } catch (err: unknown) {
    const e = err as { response?: { data?: { erreur?: string; error?: string } } }
    message.value = { ok: false, texte: e.response?.data?.erreur ?? e.response?.data?.error ?? 'Erreur inconnue' }
  } finally {
    enCours.value = false
  }
}

onMounted(charger)
</script>
