<template>
  <!-- SMC : registre + niveaux de prise de profit, un seul bouton -->
  <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl flex flex-col overflow-hidden shadow-lg relative">
    <div class="absolute top-0 left-0 w-full h-1" :style="{ background: `linear-gradient(90deg, ${s.couleur}66, transparent)` }"></div>

    <!-- Header -->
    <div class="p-5 border-b border-white/5 flex items-center justify-between pb-4">
      <h3 class="font-bold text-base flex items-center gap-2">
        <span class="w-8 h-8 rounded-full flex items-center justify-center"
          :style="{ background: `${s.couleur}1A`, color: s.couleur }">{{ s.icone }}</span>
        STRATÉGIE SMC
      </h3>
      <span class="text-[10px] px-2 py-0.5 rounded-full font-semibold" :class="badgeEtat">{{ etat }}</span>
    </div>

    <!-- Content -->
    <div class="p-5 flex-1 space-y-4">
      <!-- Registre -->
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Registre</h4>
      <div class="space-y-3">
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
                title="Officielle : signaux réels + notifiés sur Telegram. En observation : signaux journalisés en base mais silencieux (pas de message). En construction : moteur en cours de développement, aucun signal généré.">État</span>
          <select v-model="etat" class="bg-black/30 border border-white/10 rounded-md px-2 py-1 text-xs text-white"
                  title="Officielle : signaux réels + Telegram. En observation : journalisé, silencieux. En construction : moteur non branché.">
            <option value="Officielle">Officielle</option>
            <option value="Observation">En observation</option>
            <option value="Construction">En construction</option>
          </select>
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Son Telegram</span>
          <button @click="notifications = !notifications"
            :class="notifications ? 'bg-emerald-500' : 'bg-gray-600'"
            class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors">
            <span :class="notifications ? 'translate-x-5' : 'translate-x-1'"
              class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform" />
          </button>
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Capital alloué ($)</span>
          <input v-model.number="capital" type="number" min="0" step="100"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Risque par trade</span>
          <select v-model.number="risquePct" class="bg-black/30 border border-white/10 rounded-md px-2 py-1.5 text-xs text-white">
            <option :value="1">1 %</option>
            <option :value="2">2 %</option>
            <option :value="3">3 %</option>
          </select>
        </div>
      </div>

      <div class="h-px w-full bg-white/5 my-2"></div>

      <!-- Paramètres moteur -->
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Niveaux de prise de profit</h4>
      <div class="space-y-3">
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">TP1 (× R)</span>
          <input v-model.number="tp1" type="number" :step="0.05" :min="0.2" :max="1.5"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
        </div>
      </div>

      <p class="text-[11px] text-white leading-relaxed">
        TP1 par défaut 0,6 (décision étape 4 — replay +239R), borné 0,2 – 1,5. TP2 = +2R et
        TP3 = liquidité restent fixes. S'applique aux nouveaux signaux au prochain armement
        des moteurs (redémarrage de l'app).
      </p>
    </div>

    <!-- Action unique : registre + paramètres moteur -->
    <div class="p-5 mt-auto bg-black/10 border-t border-white/5">
      <div class="flex items-center justify-between">
        <span v-if="msg" class="text-xs mr-2 transition-opacity" :class="msg.ok ? 'text-emerald-400' : 'text-red-400'">
          {{ msg.text }}
        </span>
        <span v-else class="text-xs mr-2 text-transparent">Sp</span>
        <button @click="enregistrer" :disabled="saving"
          class="px-4 py-2 w-full max-w-[140px] bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded-lg transition-all shadow-lg hover:shadow-blue-500/20 active:scale-95 disabled:opacity-50">
          {{ saving ? '...' : 'Enregistrer' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { http } from '@/services/http.client'

const props = defineProps<{ s: { id: string; nom: string; description: string; icone: string; couleur: string; etat: string; notifications: boolean; capital: number; risque_pct: number } }>()

const etat = ref(props.s.etat)
const notifications = ref(props.s.notifications)
const capital = ref(props.s.capital)
const risquePct = ref(props.s.risque_pct)
const tp1 = ref(0.6)
const saving = ref(false)
const msg = ref<{ ok: boolean; text: string } | null>(null)

const badgeEtat = computed(() => ({
  'Officielle': 'bg-emerald-500/20 text-emerald-300',
  'Observation': 'bg-yellow-500/20 text-yellow-300',
  'Construction': 'bg-gray-500/20 text-white',
}[etat.value] ?? 'bg-gray-500/20 text-white'))

onMounted(async () => {
  try {
    const res = await http.get<{ cle: string; valeur: string | null }>('/api/config', {
      params: { cle: 'smc_tp1_mult' },
    })
    if (res.data.valeur !== null) {
      const v = Number.parseFloat(res.data.valeur)
      if (!Number.isNaN(v)) tp1.value = v
    }
  } catch { /* valeur par défaut 0.6 */ }
})

/// Un seul bouton : sauve le registre PUIS le paramètre moteur, avec un
/// message honnête si l'un des deux échoue.
async function enregistrer() {
  saving.value = true; msg.value = null
  const erreurs: string[] = []
  try {
    await http.put(`/api/strategies/${props.s.id}`, {
      etat: etat.value,
      notifications: notifications.value,
      capital: capital.value,
      risque_pct: risquePct.value,
    })
  } catch (e: any) { erreurs.push(`registre : ${e.message}`) }

  if (Number.isNaN(tp1.value) || tp1.value < 0.2 || tp1.value > 1.5) {
    erreurs.push('TP1 doit être entre 0,2 et 1,5 (non sauvegardé)')
  } else {
    try {
      await http.post('/api/config', { cle: 'smc_tp1_mult', valeur: String(tp1.value) })
    } catch (e: any) { erreurs.push(`TP1 : ${e.message}`) }
  }

  msg.value = erreurs.length
    ? { ok: false, text: 'Échec — ' + erreurs.join(' · ') }
    : { ok: true, text: 'Sauvegardé ✓' }
  saving.value = false
  setTimeout(() => msg.value = null, 4000)
}
</script>
