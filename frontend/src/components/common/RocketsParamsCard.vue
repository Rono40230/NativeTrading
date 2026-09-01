<template>
  <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl flex flex-col overflow-hidden shadow-lg relative">
    <div class="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-blue-500/50 to-transparent"></div>
    <div class="p-5 border-b border-white/5 flex items-center justify-between pb-4">
      <h3 class="font-bold text-base flex items-center gap-2">
        <span class="w-8 h-8 rounded-full bg-blue-500/10 flex items-center justify-center text-blue-400">🚀</span>
        STRATÉGIE ROCKETS
      </h3>
    </div>
    <div class="p-5 flex-1 space-y-4">
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Profil de risque (choix propriétaire)</h4>
      <div class="flex gap-2">
        <button v-for="p in PROFILS" :key="p.valeur" @click="profil = p.valeur"
          class="flex-1 px-3 py-2 rounded-lg text-xs font-medium border transition-colors"
          :class="profil === p.valeur ? 'bg-blue-600/30 border-blue-400/50 text-white' : 'bg-white/5 border-white/10 text-white hover:bg-white/10'">
          {{ p.libelle }}<br /><span class="text-[10px] opacity-70">{{ p.pct }}</span>
        </button>
      </div>

      <div class="h-px w-full bg-white/5 my-2"></div>

      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Gestion & détection</h4>
      <div class="space-y-3">
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Plafond position (% du capital)</span>
          <input v-model.number="plafond" type="number" :step="0.5" :min="1" :max="25"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Trailing stop (% du prix, dès R1)</span>
          <input v-model.number="trailing" type="number" :step="0.5" :min="1" :max="30"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Volume au pivot (× MM50)</span>
          <input v-model.number="volumeMult" type="number" :step="0.1" :min="1" :max="3"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Conviction min. du ranker IA (0 = informatif)</span>
          <input v-model.number="convictionMin" type="number" :step="5" :min="0" :max="100"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Cassure décisive min. (% au-delà du pivot)</span>
          <input v-model.number="cassureMin" type="number" :step="0.5" :min="1" :max="10"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
      </div>

      <p class="text-[11px] text-white leading-relaxed">
        Vente fixe de 50 % à R1. News (1/10) et véto unlocks : enrichissement IA (étape 6).
      </p>
    </div>
    <div class="p-5 mt-auto bg-black/10 border-t border-white/5">
      <div class="flex items-center justify-between">
        <span v-if="msg" class="text-xs mr-2" :class="msg.ok ? 'text-emerald-400' : 'text-red-400'">{{ msg.text }}</span>
        <span v-else class="text-xs mr-2 text-transparent">Sp</span>
        <button @click="sauvegarder" :disabled="saving"
          class="px-4 py-2 w-full max-w-[140px] bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-all active:scale-95 disabled:opacity-50">
          {{ saving ? '...' : 'Enregistrer' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { http } from '@/services/http.client'

const PROFILS = [
  { valeur: 'PeuRisque', libelle: 'Peu Risqué', pct: '0,5 %' },
  { valeur: 'Neutre', libelle: 'Neutre', pct: '1 %' },
  { valeur: 'Risque', libelle: 'Risqué', pct: '2 %' },
]

const profil = ref('Neutre')
const plafond = ref(5)
const trailing = ref(5)
const volumeMult = ref(1.5)
const cassureMin = ref(3)
const convictionMin = ref(40)
const saving = ref(false)
const msg = ref<{ ok: boolean; text: string } | null>(null)

onMounted(async () => {
  try {
    const res = await http.get('/api/rockets/candidats') // vérifie la disponibilité
    void res
  } catch { /* silencieux */ }
})

async function sauvegarder() {
  saving.value = true; msg.value = null
  try {
    await http.put('/api/rockets/params', {
      profil: profil.value,
      plafond_position_pct: plafond.value,
      trailing_pct: trailing.value,
      volume_pivot_mult: volumeMult.value,
      cassure_min_pct: cassureMin.value,
      conviction_min: convictionMin.value,
    })
    msg.value = { ok: true, text: 'Sauvegardé ✓' }
  } catch (e: any) {
    msg.value = { ok: false, text: `Erreur : ${e.message}` }
  } finally {
    saving.value = false
    setTimeout(() => msg.value = null, 3000)
  }
}
</script>
