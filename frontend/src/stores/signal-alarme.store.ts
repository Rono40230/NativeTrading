/**
 * Store global pour la file d'alarmes signal.
 * Alimenté par le WebSocket global dans App.vue.
 * Consommé par SignalAlarmeModal.vue (affiché dans App.vue).
 */
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Signal } from '@/services/api.types'

export const useSignalAlarmeStore = defineStore('signal-alarme', () => {
  /** File d'attente des signaux non encore fermés par l'utilisateur */
  const file = ref<Signal[]>([])
  /** Index du signal actuellement affiché */
  const index = ref(0)

  const total = computed(() => file.value.length)
  const signalActuel = computed(() => file.value[index.value] ?? null)
  const visible = computed(() => file.value.length > 0)

  function ajouterSignal(signal: Signal) {
    file.value.push(signal)
    // Si c'était vide, on pointe sur 0
    if (file.value.length === 1) index.value = 0
  }

  function fermerActuel() {
    file.value.splice(index.value, 1)
    // Recaler l'index si on supprime le dernier
    if (index.value > 0 && index.value >= file.value.length) {
      index.value = file.value.length - 1
    }
  }

  function precedent() {
    if (index.value > 0) index.value--
  }

  function suivant() {
    if (index.value < file.value.length - 1) index.value++
  }

  return { file, index, total, signalActuel, visible, ajouterSignal, fermerActuel, precedent, suivant }
})
