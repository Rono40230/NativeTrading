<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" @click.self="$emit('close')">
      <div class="glass-card w-full max-w-md p-6 flex flex-col gap-4">
        <div class="flex items-center justify-between">
          <h2 class="text-white font-bold text-lg">Résultat du trade — {{ ticker }}</h2>
          <button class="text-gray-500 hover:text-white" @click="$emit('close')">✕</button>
        </div>

        <!-- Bouton Ignoré -->
        <button
          class="w-full py-2 rounded-lg border border-gray-600 text-gray-400 hover:bg-white/10 transition-all text-sm"
          @click="soumettre('ignore')"
        >
          🚫 Signal ignoré (non pris)
        </button>

        <div class="border-t border-white/10 my-1" />

        <!-- Formulaire trade pris -->
        <div class="flex flex-col gap-3">
          <div class="flex gap-3">
            <div class="flex-1 flex flex-col gap-1">
              <label class="text-xs text-gray-400">Prix d'entrée réel</label>
              <input v-model.number="entree" type="number" step="any" class="input-field" placeholder="ex: 84500" />
            </div>
            <div class="flex-1 flex flex-col gap-1">
              <label class="text-xs text-gray-400">Prix de sortie</label>
              <input v-model.number="sortie" type="number" step="any" class="input-field" placeholder="ex: 85200" />
            </div>
          </div>

          <div class="flex flex-col gap-1">
            <label class="text-xs text-gray-400">Résultat</label>
            <div class="flex gap-2 flex-wrap">
              <button v-for="v in verdicts" :key="v.val"
                class="px-3 py-1.5 rounded-lg text-sm font-bold border transition-all"
                :class="verdict === v.val ? v.classeActive : 'border-white/10 bg-white/5 text-gray-400 hover:bg-white/10'"
                @click="verdict = v.val"
              >{{ v.label }}</button>
            </div>
          </div>

          <div class="flex flex-col gap-1">
            <label class="text-xs text-gray-400">Notes (optionnel)</label>
            <input v-model="notes" type="text" class="input-field" placeholder="ex: mèche de rejet avant TP2" />
          </div>

          <button
            class="w-full py-2 rounded-lg bg-emerald-700 hover:bg-emerald-600 text-white font-bold transition-all disabled:opacity-40"
            :disabled="!verdict || !entree || !sortie || envoi"
            @click="soumettre(verdict!)"
          >
            {{ envoi ? 'Enregistrement…' : '✅ Enregistrer le résultat' }}
          </button>

          <p v-if="erreur" class="text-red-400 text-xs text-center">{{ erreur }}</p>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { apiService } from '@/services/api.service'

const props = defineProps<{ open: boolean; signalId: number; ticker: string }>()
const emit = defineEmits<{ close: []; saved: [] }>()

const entree = ref<number | null>(null)
const sortie = ref<number | null>(null)
const verdict = ref<'tp1' | 'tp2' | 'tp3' | 'sl' | null>(null)
const notes = ref('')
const envoi = ref(false)
const erreur = ref('')

const verdicts = [
  { val: 'tp1' as const, label: '🟡 TP1', classeActive: 'border-yellow-500 bg-yellow-900/50 text-yellow-300' },
  { val: 'tp2' as const, label: '🔵 TP2', classeActive: 'border-blue-500 bg-blue-900/50 text-blue-300' },
  { val: 'tp3' as const, label: '🟢 TP3', classeActive: 'border-emerald-500 bg-emerald-900/50 text-emerald-300' },
  { val: 'sl'  as const, label: '🔴 SL',  classeActive: 'border-red-500 bg-red-900/50 text-red-300' },
]

async function soumettre(v: 'tp1' | 'tp2' | 'tp3' | 'sl' | 'ignore') {
  envoi.value = true
  erreur.value = ''
  try {
    await apiService.postFeedbackTrader({
      signal_id: props.signalId,
      verdict: v,
      prix_entree_reel: entree.value ?? undefined,
      prix_sortie_reel: sortie.value ?? undefined,
      notes: notes.value || undefined,
    })
    emit('saved')
    emit('close')
  } catch {
    erreur.value = 'Erreur lors de l\'enregistrement'
  } finally {
    envoi.value = false
  }
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-[#0a0e27] backdrop-blur-md; }
.input-field { @apply w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-white text-sm placeholder-gray-600 focus:outline-none focus:border-blue-500; }
</style>
