<template>
  <!-- STRADDLE : registre + minutage/risque, un seul bouton -->
  <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl flex flex-col overflow-hidden shadow-lg relative">
    <div class="absolute top-0 left-0 w-full h-1" :style="{ background: `linear-gradient(90deg, ${s.couleur}66, transparent)` }"></div>

    <!-- Header -->
    <div class="p-5 border-b border-white/5 flex items-center justify-between pb-4">
      <h3 class="font-bold text-base flex items-center gap-2">
        <span class="w-8 h-8 rounded-full flex items-center justify-center"
          :style="{ background: `${s.couleur}1A`, color: s.couleur }">{{ s.icone }}</span>
        STRATÉGIE STRADDLE
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
          <span class="text-white text-xs">Risque par passe</span>
          <select v-model.number="risquePct" class="bg-black/30 border border-white/10 rounded-md px-2 py-1.5 text-xs text-white">
            <option :value="1">1 %</option>
            <option :value="2">2 %</option>
            <option :value="3">3 %</option>
          </select>
        </div>
      </div>

      <div class="h-px w-full bg-white/5 my-2"></div>

      <!-- Minutage -->
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Minutage</h4>
      <div class="space-y-3">
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Placement des 2 jambes (secondes avant l'annonce)</span>
          <input v-model.number="store.straddleRaw['placement_sec']" type="number" :step="1" :min="1"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
        </div>
      </div>

      <!-- Risque -->
      <h4 class="text-xs uppercase text-white font-semibold tracking-wider">Risque (R = SL × ATR H1)</h4>
      <div class="space-y-3">
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">SL (1R) × ATR H1</span>
          <input v-model.number="store.straddleRaw['sl_mult']" type="number" :step="0.1" :min="0.1"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
        </div>
        <div class="flex items-center justify-between gap-4">
          <span class="text-white text-xs">Trailing (× R, dès TP2)</span>
          <input v-model.number="store.straddleRaw['trailing_r']" type="number" :step="0.1" :min="0.1"
            class="w-20 bg-black/20 border border-white/10 rounded-md px-3 py-1.5 text-right text-white focus:outline-none focus:ring-1 focus:ring-blue-500/50 transition-all appearance-none" />
        </div>
      </div>

      <p class="text-[11px] text-white leading-relaxed">
        R est mesuré sur l'ATR H1 (volatilité normale de l'actif) et non sur la
        compression M1 pré-annonce — un R microscopique faisait égorger les jambes
        par le spike initial (constat Gate 3 26/08). TP1 = 1R (SL resserré à E∓0,5R — tampon
        anti-whipsaw 27/08) et TP2 = 2R (SL à TP1 + trailing) canoniques. Time-stop 60 min.
        Ces trois réglages s'appliquent aux nouveaux signaux au prochain armement des
        moteurs (redémarrage de l'app).
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
import { ref, computed } from 'vue'
import { http } from '@/services/http.client'
import { useStrategyParamsStore } from '@/stores/strategyParams.store'

const props = defineProps<{ s: { id: string; nom: string; description: string; icone: string; couleur: string; etat: string; notifications: boolean; capital: number; risque_pct: number } }>()

const store = useStrategyParamsStore()

const etat = ref(props.s.etat)
const notifications = ref(props.s.notifications)
const capital = ref(props.s.capital)
const risquePct = ref(props.s.risque_pct)
const saving = ref(false)
const msg = ref<{ ok: boolean; text: string } | null>(null)

const badgeEtat = computed(() => ({
  'Officielle': 'bg-emerald-500/20 text-emerald-300',
  'Observation': 'bg-yellow-500/20 text-yellow-300',
  'Construction': 'bg-gray-500/20 text-white',
}[etat.value] ?? 'bg-gray-500/20 text-white'))

/// Un seul bouton : sauve le registre PUIS les paramètres moteur, avec un
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

  try {
    await store.saveStraddle(store.straddleRaw)
  } catch (e: any) { erreurs.push(`paramètres moteur : ${e.message}`) }

  msg.value = erreurs.length
    ? { ok: false, text: 'Échec — ' + erreurs.join(' · ') }
    : { ok: true, text: 'Sauvegardé ✓' }
  saving.value = false
  setTimeout(() => msg.value = null, 4000)
}
</script>
