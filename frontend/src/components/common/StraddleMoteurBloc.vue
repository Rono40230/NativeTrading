<template>
  <!-- Bloc moteur STRADDLE — extrait de l'ex-StraddleParamsCard (09/09,
       workflow dashboard) : vit dans la modale « Paramètres moteur » de la
       carte Straddle. Sauvegarde via le store → PUT /api/straddle/params
       (table straddle_params — la source du moteur v2). -->
  <div class="flex flex-col gap-4">
    <!-- Minutage -->
    <div>
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider mb-2">Minutage</h4>
      <div class="flex items-center justify-between gap-4">
        <span class="text-white text-xs">Placement des 2 jambes (secondes avant l'annonce)</span>
        <input v-model.number="store.straddleRaw['placement_sec']" type="number" :step="1" :min="1"
          class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
      </div>
    </div>

    <!-- Risque -->
    <div>
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider mb-2">Risque (R = SL × ATR H1)</h4>
      <div class="divide-y divide-white/5">
        <div class="flex items-center justify-between gap-4 py-2 first:pt-0">
          <span class="text-white text-xs">SL (1R) × ATR H1</span>
          <input v-model.number="store.straddleRaw['sl_mult']" type="number" :step="0.1" :min="0.1"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4 py-2 last:pb-0">
          <span class="text-white text-xs">Trailing (× R, dès TP2)</span>
          <input v-model.number="store.straddleRaw['trailing_r']" type="number" :step="0.1" :min="0.1"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
        </div>
      </div>
    </div>

    <p class="text-[11px] text-white leading-relaxed">
      R est mesuré sur l'ATR H1 (volatilité normale de l'actif) et non sur la
      compression M1 pré-annonce — un R microscopique faisait égorger les jambes
      par le spike initial (constat Gate 3 26/08). TP1 = 1R (SL resserré à E∓0,5R — tampon
      anti-whipsaw 27/08) et TP2 = 2R (SL à TP1 + trailing) canoniques. Time-stop 60 min.
      Ces réglages s'appliquent aux nouveaux signaux au prochain armement des
      moteurs (redémarrage de l'app).
    </p>

    <div class="flex items-center justify-between pt-1 border-t border-white/5">
      <span v-if="msg" class="text-xs mr-2" :class="msg.ok ? 'text-emerald-400' : 'text-red-400'">{{ msg.text }}</span>
      <span v-else class="text-xs mr-2 text-transparent">Sp</span>
      <button @click="enregistrer" :disabled="saving"
        class="px-4 py-2 w-full max-w-[140px] bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-all shadow-lg hover:shadow-blue-500/20 active:scale-95 disabled:opacity-50">
        {{ saving ? '...' : 'Enregistrer' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const store = useStrategyParamsStore()
const saving = ref(false)
const msg = ref<{ ok: boolean; text: string } | null>(null)

onMounted(() => { void store.charger() })

async function enregistrer() {
  saving.value = true
  msg.value = null
  try {
    await store.saveStraddle(store.straddleRaw)
    msg.value = { ok: true, text: 'Sauvegardé ✓' }
  } catch (e: unknown) {
    const err = e as { message?: string }
    msg.value = { ok: false, text: 'Échec — ' + (err.message ?? 'erreur inconnue') }
  } finally {
    saving.value = false
    setTimeout(() => msg.value = null, 4000)
  }
}
</script>
