<template>
  <!-- Les 4 réglages de l'étalon (period, signal, amplitude, RatioRisk) +
       le filtre ADX optionnel (étude 7.E : sélecteur d'actifs). -->
  <div class="space-y-4">
    <div class="space-y-3">
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Indicateurs</h4>
      <div class="flex items-center justify-between gap-4">
        <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
              title="Fenêtre high/low du KDJ (input « period » du Pine).">KDJ · period</span>
        <input v-model.number="p.period" type="number" min="2" step="1"
          class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-cyan-500/50 appearance-none" />
      </div>
      <div class="flex items-center justify-between gap-4">
        <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
              title="Lissage K et D (input « signal »).">KDJ · signal</span>
        <input v-model.number="p.signal" type="number" min="2" step="1"
          class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-cyan-500/50 appearance-none" />
      </div>
      <div class="flex items-center justify-between gap-4">
        <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
              title="Fenêtre highma/lowma du HalfTrend (input « Amplitude »).">HalfTrend · amplitude</span>
        <input v-model.number="p.amplitude" type="number" min="1" step="1"
          class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-cyan-500/50 appearance-none" />
      </div>
    </div>

    <div class="h-px w-full bg-white/5 my-2"></div>

    <div class="space-y-3">
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Gestion</h4>
      <div class="flex items-center justify-between gap-4">
        <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
              title="TP = entrée ± RatioRisk × distance (entrée→EMA200) ; SL = EMA200 de la barre d'entrée.">RatioRisk (TP × R)</span>
        <input v-model.number="p.ratio_risk" type="number" min="0.1" max="10" step="0.1"
          class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-cyan-500/50 appearance-none" />
      </div>
      <div class="flex items-center justify-between gap-4">
        <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
              title="Filtre tendance : n'entrer que si ADX(14) ≥ seuil au signal. −1 = désactivé (fidélité étalon). Étude 7.E : améliore les métaux (XAU +0,12→0,31 %/trade, XAG +0,40→0,59), dégrade SP500/BTC — à utiliser avec le scanner, pas seul.">ADX(14) minimum (−1 = off)</span>
        <input v-model.number="p.adx_min" type="number" min="-1" max="60" step="1"
          class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-cyan-500/50 appearance-none" />
      </div>
    </div>

    <p class="text-[11px] text-white leading-relaxed">
      Moteur H1 sur tous les actifs collectés. Entrée à l'open suivant la
      clôture de signal, TP/SL par croisements (aucun ordre serveur) — miroir
      vérifié du Pine étalon (parité rejeu↔MQ5 : 6/6 trades, 0 divergence).
      Ces réglages s'appliquent au prochain démarrage de l'app.
    </p>

    <div class="flex items-center justify-between pt-2 border-t border-white/5">
      <span v-if="msg" class="text-xs" :class="msg.ok ? 'text-emerald-400' : 'text-red-400'">{{ msg.texte }}</span>
      <span v-else class="text-xs text-transparent">Sp</span>
      <button @click="enregistrer" :disabled="enCours"
        class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white text-sm font-medium rounded-lg transition-all active:scale-95 disabled:opacity-50">
        {{ enCours ? '...' : 'Enregistrer' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { http } from '@/services/http.client'

interface ParamsKdj { period: number; signal: number; amplitude: number; ratio_risk: number; adx_min: number }

const p = ref<ParamsKdj>({ period: 20, signal: 7, amplitude: 2, ratio_risk: 2.0, adx_min: -1 })
const enCours = ref(false)
const msg = ref<{ ok: boolean; texte: string } | null>(null)

onMounted(async () => {
  try {
    const res = await http.get<ParamsKdj>('/api/kdj/params')
    p.value = res.data
  } catch { /* défauts affichés */ }
})

async function enregistrer() {
  enCours.value = true
  msg.value = null
  try {
    await http.put('/api/kdj/params', p.value)
    msg.value = { ok: true, texte: 'Sauvegardé ✓ (effet au redémarrage)' }
  } catch (e: any) {
    msg.value = { ok: false, texte: e?.response?.data?.erreur ?? 'Erreur inconnue' }
  } finally {
    enCours.value = false
    setTimeout(() => (msg.value = null), 4000)
  }
}
</script>
