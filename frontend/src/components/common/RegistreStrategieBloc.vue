<template>
  <!-- Bloc REGISTRE — commun aux trois stratégies (workflow dashboard 09/09) :
       vit dans la modale « Paramètres » de la carte du dashboard. Sauvegarde
       isolée via PUT /api/strategies/{id}. -->
  <div class="divide-y divide-white/5">
    <div class="flex items-center justify-between gap-4 py-2 first:pt-0.5">
      <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
            title="Officielle : signaux réels + notifiés sur Telegram. En observation : signaux journalisés en base mais silencieux (pas de message). En construction : moteur en cours de développement, aucun signal généré.">État</span>
      <select v-model="etat" class="bg-black/30 border border-white/10 rounded-md px-2 py-1 text-xs text-white"
              title="Officielle : signaux réels + Telegram. En observation : journalisé, silencieux. En construction : moteur non branché.">
        <option value="Officielle">Officielle</option>
        <option value="Observation">En observation</option>
        <option value="Construction">En construction</option>
      </select>
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs">Son Telegram</span>
      <button @click="notifications = !notifications"
        :class="notifications ? 'bg-emerald-500' : 'bg-gray-600'"
        class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors">
        <span :class="notifications ? 'translate-x-5' : 'translate-x-1'"
          class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform" />
      </button>
    </div>
    <div class="flex items-center justify-between gap-4 py-2">
      <span class="text-white text-xs">Capital alloué ($)</span>
      <input v-model.number="capital" type="number" min="0" step="100"
        class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
    </div>
    <div class="flex items-center justify-between gap-4 py-2 last:pb-0">
      <span class="text-white text-xs">{{ libelleRisque }}</span>
      <select v-model.number="risquePct" class="bg-black/30 border border-white/10 rounded-md px-2 py-1.5 text-xs text-white">
        <option :value="1">1 %</option>
        <option :value="2">2 %</option>
        <option :value="3">3 %</option>
      </select>
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

const props = defineProps<{
  /** Identifiant stratégie côté API ('SMC' | 'straddle' | 'rockets'). */
  id: string
  /** Libellé de la ligne de risque (« Risque par trade/passe/rocket »). */
  libelleRisque: string
}>()

interface StrategieApi {
  id: string; etat: string; notifications: boolean; capital: number; risque_pct: number
}

const etat = ref('Observation')
const notifications = ref(false)
const capital = ref(2000)
const risquePct = ref(1)
const saving = ref(false)
const msg = ref<{ ok: boolean; text: string } | null>(null)

onMounted(async () => {
  try {
    const res = await http.get<StrategieApi[]>('/api/strategies')
    const s = res.data.find(x => x.id === props.id)
    if (s) {
      etat.value = s.etat
      notifications.value = s.notifications
      capital.value = s.capital
      risquePct.value = s.risque_pct
    }
  } catch { /* valeurs par défaut */ }
})

async function enregistrer() {
  saving.value = true
  msg.value = null
  try {
    await http.put(`/api/strategies/${props.id}`, {
      etat: etat.value,
      notifications: notifications.value,
      capital: capital.value,
      risque_pct: risquePct.value,
    })
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
