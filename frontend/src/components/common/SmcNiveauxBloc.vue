<template>
  <!-- Bloc NIVEAUX DE PRISE DE PROFIT SMC — extrait de l'ex-SmcParamsCard
       (09/09, workflow dashboard) : vit dans la modale « Niveaux de profits »
       de la carte SMC. Sauvegarde isolée vers la kv configuration (clés
       smc_* — les seules lues par le moteur via runtime_tick). -->
  <div class="divide-y divide-white/5">
    <div class="flex items-center justify-between gap-4 py-2 first:pt-0.5">
      <span class="text-white text-xs">TP1 (× R)</span>
      <input v-model.number="tp1" type="number" :step="0.05" :min="0.2" :max="1.5"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs">TP2 (× R)</span>
      <input v-model.number="tp2" type="number" :step="0.1" :min="1" :max="4"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs">TP3 — mode</span>
      <select v-model="tp3Mode" class="bg-black/30 border border-white/10 rounded-md px-2 py-1.5 text-xs text-white">
        <option value="lointaine">Liquidité lointaine</option>
        <option value="rfixe">R fixe</option>
      </select>
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
            title="Mode R fixe : cible directe. Mode liquidité lointaine : repli si aucune liquidité au-delà de TP2 (ou sous TP2).">R fixe / repli (× R)</span>
      <input v-model.number="tp3Rfixe" type="number" :step="0.5" :min="3" :max="10"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50" />
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs">Trailing stop après TP2</span>
      <button @click="trailingOn = !trailingOn"
        :class="trailingOn ? 'bg-emerald-500' : 'bg-gray-600'"
        class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors">
        <span :class="trailingOn ? 'translate-x-5' : 'translate-x-1'"
          class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform" />
      </button>
    </div>
    <div v-if="trailingOn" class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs">Distance du trailing (× R)</span>
      <input v-model.number="trailingR" type="number" :step="0.05" :min="0.1" :max="1"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50" />
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
            title="Ventes partielles : part du lot vendue à chaque palier. Le solde sort à la cible, au trailing ou à BE selon le verdict. Σ = 100 %.">Vente à TP1 (%)</span>
      <input v-model.number="fracTp1" type="number" :step="5" :min="0" :max="100"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs">Vente à TP2 (%)</span>
      <input v-model.number="fracTp2" type="number" :step="5" :min="0" :max="100"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
    </div>
    <div class="flex items-center justify-between gap-4 py-2 last:pb-0">
      <span class="text-white text-xs">Solde à TP3 (%)</span>
      <input v-model.number="fracTp3" type="number" :step="5" :min="0" :max="100"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
    </div>

    <div class="flex items-center justify-between pt-3">
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

const tp1 = ref(0.6)
const tp2 = ref(2.0)
const tp3Mode = ref<'lointaine' | 'rfixe'>('lointaine')
const tp3Rfixe = ref(3.0)
const trailingOn = ref(false)
const trailingR = ref(0.5)
const fracTp1 = ref(50)
const fracTp2 = ref(30)
const fracTp3 = ref(20)
const saving = ref(false)
const msg = ref<{ ok: boolean; text: string } | null>(null)

async function lire(cle: string): Promise<string | null> {
  const r = await http.get<{ cle: string; valeur: string | null }>('/api/config', { params: { cle } })
  return r.data.valeur
}

onMounted(async () => {
  try {
    for (const [cle, setter] of [
      ['smc_tp1_mult', (v: number) => (tp1.value = v)],
      ['smc_tp2_mult', (v: number) => (tp2.value = v)],
      ['smc_tp3_rfixe', (v: number) => (tp3Rfixe.value = v)],
      ['smc_tp3_trailing_r', (v: number) => (trailingR.value = v)],
      ['smc_frac_tp1', (v: number) => (fracTp1.value = Math.round(v * 100))],
      ['smc_frac_tp2', (v: number) => (fracTp2.value = Math.round(v * 100))],
      ['smc_frac_tp3', (v: number) => (fracTp3.value = Math.round(v * 100))],
    ] as const) {
      const brut = await lire(cle)
      if (brut !== null) {
        const v = Number.parseFloat(brut)
        if (!Number.isNaN(v)) setter(v)
      }
    }
    const mode = await lire('smc_tp3_mode')
    if (mode === 'rfixe' || mode === 'lointaine') tp3Mode.value = mode
    trailingOn.value = (await lire('smc_tp3_trailing')) === '1'
  } catch { /* valeurs par défaut */ }
})

async function enregistrer() {
  saving.value = true
  msg.value = null
  const erreurs: string[] = []
  const ecrire = async (cle: string, valeur: string, libelle: string) => {
    try {
      await http.post('/api/config', { cle, valeur })
    } catch (e: unknown) {
      const err = e as { message?: string }
      erreurs.push(`${libelle} : ${err.message ?? 'erreur'}`)
    }
  }

  if (Number.isNaN(tp1.value) || tp1.value < 0.2 || tp1.value > 1.5) {
    erreurs.push('TP1 doit être entre 0,2 et 1,5 (non sauvegardé)')
  } else {
    await ecrire('smc_tp1_mult', String(tp1.value), 'TP1')
  }

  if (Number.isNaN(tp2.value) || tp2.value < 1.0 || tp2.value > 4.0) {
    erreurs.push('TP2 doit être entre 1,0 et 4,0 (non sauvegardé)')
  } else if (!Number.isNaN(tp1.value) && tp2.value <= tp1.value) {
    erreurs.push('TP2 doit être supérieur à TP1 (non sauvegardé)')
  } else {
    await ecrire('smc_tp2_mult', String(tp2.value), 'TP2')
  }

  if (Number.isNaN(tp3Rfixe.value) || tp3Rfixe.value < 3.0 || tp3Rfixe.value > 10.0) {
    erreurs.push('R fixe TP3 doit être entre 3 et 10 (non sauvegardé)')
  } else if (!Number.isNaN(tp2.value) && tp3Rfixe.value <= tp2.value) {
    erreurs.push('R fixe TP3 doit être supérieur à TP2 (non sauvegardé)')
  } else {
    await ecrire('smc_tp3_rfixe', String(tp3Rfixe.value), 'TP3')
    await ecrire('smc_tp3_mode', tp3Mode.value, 'TP3 mode')
  }

  if (trailingOn.value && (Number.isNaN(trailingR.value) || trailingR.value < 0.1 || trailingR.value > 1.0)) {
    erreurs.push('Distance du trailing entre 0,1 et 1R (non sauvegardé)')
  } else {
    await ecrire('smc_tp3_trailing', trailingOn.value ? '1' : '0', 'trailing')
    if (trailingOn.value) {
      await ecrire('smc_tp3_trailing_r', String(trailingR.value), 'trailing R')
    }
  }

  const somme = fracTp1.value + fracTp2.value + fracTp3.value
  if (somme !== 100) {
    erreurs.push(`Fractions : Σ = ${somme} % — doit faire 100 % (non sauvegardé)`)
  } else {
    await ecrire('smc_frac_tp1', String(fracTp1.value / 100), 'fractions')
    await ecrire('smc_frac_tp2', String(fracTp2.value / 100), 'fractions')
    await ecrire('smc_frac_tp3', String(fracTp3.value / 100), 'fractions')
  }

  msg.value = erreurs.length
    ? { ok: false, text: 'Échec — ' + erreurs.join(' · ') }
    : { ok: true, text: 'Sauvegardé ✓ (effet ≤ 60 s)' }
  saving.value = false
  setTimeout(() => msg.value = null, 4000)
}
</script>
