<template>
  <!-- Bloc moteur ROCKETS — extrait de l'ex-RocketsParamsCard (09/09, workflow
       dashboard) : vit dans la modale « Paramètres moteur » de la carte
       Rockets. Charge les valeurs RÉELLES via GET /api/rockets/params
       (ajouté le 09/09 — l'ancienne carte affichait les défauts), sauve via
       PUT (table rockets_params — la source du scanner). -->
  <div class="flex flex-col gap-4">
    <!-- Profil de risque -->
    <div>
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider mb-2">Profil de risque (choix propriétaire)</h4>
      <div class="flex gap-2">
        <button v-for="p in PROFILS" :key="p.valeur" @click="profil = p.valeur"
          class="flex-1 px-3 py-2 rounded-lg text-xs font-medium border transition-colors"
          :class="profil === p.valeur ? 'bg-blue-600/30 border-blue-400/50 text-white' : 'bg-white/5 border-white/10 text-white hover:bg-white/10'">
          {{ p.libelle }}<br /><span class="text-[10px] opacity-70">{{ p.pct }}</span>
        </button>
      </div>
    </div>

    <!-- Gestion & détection -->
    <div>
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider mb-2">Gestion &amp; détection</h4>
      <div class="divide-y divide-white/5">
        <div class="flex items-center justify-between gap-4 py-2 first:pt-0">
          <span class="text-white text-xs">Plafond position (% du capital)</span>
          <input v-model.number="plafond" type="number" :step="0.5" :min="1" :max="25"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4 py-2">
          <span class="text-white text-xs">Trailing stop (% du prix, dès R1)</span>
          <input v-model.number="trailing" type="number" :step="0.5" :min="1" :max="30"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4 py-2">
          <span class="text-white text-xs">Volume au pivot (× MM50)</span>
          <input v-model.number="volumeMult" type="number" :step="0.1" :min="1" :max="3"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4 py-2">
          <span class="text-white text-xs">Conviction min. du ranker IA (0 = informatif)</span>
          <input v-model.number="convictionMin" type="number" :step="5" :min="0" :max="100"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4 py-2 last:pb-0">
          <span class="text-white text-xs">Cassure décisive min. (% au-delà du pivot)</span>
          <input v-model.number="cassureMin" type="number" :step="0.5" :min="1" :max="10"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
      </div>
    </div>

    <p class="text-[11px] text-white leading-relaxed">
      Vente fixe de 50 % à R1. News (1/10) et véto unlocks : enrichissement IA (étape 6).
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

interface ParamsApi {
  profil: string
  plafond_position_pct: number
  trailing_pct: number
  volume_pivot_mult: number
  cassure_min_pct: number
  conviction_min: number
}

onMounted(async () => {
  try {
    const res = await http.get<ParamsApi>('/api/rockets/params')
    profil.value = res.data.profil
    plafond.value = res.data.plafond_position_pct
    trailing.value = res.data.trailing_pct
    volumeMult.value = res.data.volume_pivot_mult
    cassureMin.value = res.data.cassure_min_pct
    convictionMin.value = res.data.conviction_min
  } catch { /* valeurs par défaut */ }
})

async function enregistrer() {
  saving.value = true
  msg.value = null
  try {
    await http.put('/api/rockets/params', {
      profil: profil.value,
      plafond_position_pct: plafond.value,
      trailing_pct: trailing.value,
      volume_pivot_mult: volumeMult.value,
      cassure_min_pct: cassureMin.value,
      conviction_min: convictionMin.value,
    })
    msg.value = { ok: true, text: 'Sauvegardé ✓ (actif au prochain scan)' }
  } catch (e: unknown) {
    const err = e as { message?: string }
    msg.value = { ok: false, text: 'Échec — ' + (err.message ?? 'erreur inconnue') }
  } finally {
    saving.value = false
    setTimeout(() => msg.value = null, 4000)
  }
}
</script>
