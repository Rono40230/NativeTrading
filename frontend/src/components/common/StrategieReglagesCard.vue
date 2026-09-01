<template>
  <div class="border border-white/10 bg-white/5 backdrop-blur-md rounded-xl overflow-hidden shadow-lg relative">
    <div class="absolute top-0 left-0 w-full h-1" :style="{ background: `linear-gradient(90deg, ${s.couleur}66, transparent)` }" />
    <div class="p-4 border-b border-white/5 flex items-center justify-between">
      <h3 class="font-bold text-sm flex items-center gap-2">
        <span>{{ s.icone }}</span> {{ s.nom }}
      </h3>
      <span class="text-[10px] px-2 py-0.5 rounded-full font-semibold" :class="badgeEtat">{{ s.etat }}</span>
    </div>

    <div class="p-4 space-y-3">
      <div class="flex items-center justify-between gap-3">
        <span class="text-white text-xs cursor-help border-b border-dotted border-gray-600"
              title="Officielle : signaux réels + notifiés sur Telegram. En observation : signaux journalisés en base mais silencieux (pas de message). En construction : moteur en cours de développement, aucun signal généré.">État</span>
        <select v-model="etat" class="bg-black/30 border border-white/10 rounded-md px-2 py-1 text-xs text-white"
                title="Officielle : signaux réels + Telegram. En observation : journalisé, silencieux. En construction : moteur non branché.">
          <option value="Officielle">Officielle</option>
          <option value="Observation">En observation</option>
          <option value="Construction">En construction</option>
        </select>
      </div>

      <div class="flex items-center justify-between gap-3">
        <span class="text-white text-xs">Son Telegram</span>
        <button @click="notifications = !notifications"
          :class="notifications ? 'bg-emerald-500' : 'bg-gray-600'"
          class="relative inline-flex h-5 w-9 items-center rounded-full transition-colors">
          <span :class="notifications ? 'translate-x-5' : 'translate-x-1'"
            class="inline-block h-3 w-3 transform rounded-full bg-white transition-transform" />
        </button>
      </div>

      <div class="flex items-center justify-between gap-3">
        <span class="text-white text-xs">Capital alloué ($)</span>
        <input v-model.number="capital" type="number" min="0" step="100"
          class="w-24 bg-black/30 border border-white/10 rounded-md px-2 py-1 text-right text-xs text-white" />
      </div>

      <div class="flex items-center justify-between gap-3">
        <span class="text-white text-xs">Risque par trade</span>
        <select v-model.number="risquePct" class="bg-black/30 border border-white/10 rounded-md px-2 py-1 text-xs text-white">
          <option :value="1">1 %</option>
          <option :value="2">2 %</option>
          <option :value="3">3 %</option>
        </select>
      </div>

      <div class="pt-1 flex items-center justify-between">
        <span v-if="message" :class="message.ok ? 'text-emerald-400' : 'text-red-400'" class="text-[11px]">{{ message.texte }}</span>
        <span v-else class="text-[11px] text-white">{{ s.description }}</span>
        <button @click="sauvegarder" :disabled="enCours"
          class="px-3 py-1 rounded-md text-xs font-medium bg-cyan-500/20 border border-cyan-500/40 text-cyan-200 hover:bg-cyan-500/30 disabled:opacity-50">
          {{ enCours ? '…' : '💾' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { http } from '@/services/http.client'

const props = defineProps<{ s: { id: string; nom: string; description: string; icone: string; couleur: string; etat: string; notifications: boolean; capital: number; risque_pct: number } }>()

const etat = ref(props.s.etat)
const notifications = ref(props.s.notifications)
const capital = ref(props.s.capital)
const risquePct = ref(props.s.risque_pct)
const enCours = ref(false)
const message = ref<{ ok: boolean; texte: string } | null>(null)

const badgeEtat = computed(() => ({
  'Officielle': 'bg-emerald-500/20 text-emerald-300',
  'Observation': 'bg-yellow-500/20 text-yellow-300',
  'Construction': 'bg-gray-500/20 text-white',
}[etat.value] ?? 'bg-gray-500/20 text-white'))

async function sauvegarder() {
  enCours.value = true
  message.value = null
  try {
    await http.put(`/api/strategies/${props.s.id}`, {
      etat: etat.value,
      notifications: notifications.value,
      capital: capital.value,
      risque_pct: risquePct.value,
    })
    message.value = { ok: true, texte: '✅ Sauvegardé' }
  } catch (e: unknown) {
    message.value = { ok: false, texte: e instanceof Error ? e.message : 'Erreur' }
  } finally {
    enCours.value = false
  }
}
</script>
