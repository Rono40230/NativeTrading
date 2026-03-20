import { defineStore } from 'pinia'
import { ref } from 'vue'

export type TypeAlerte = 'success' | 'error' | 'warning' | 'info'

export interface Alerte {
  id: number
  type: TypeAlerte
  message: string
}

let compteur = 0

export const useAlerteStore = defineStore('alertes', () => {
  const alertes = ref<Alerte[]>([])

  function afficher(message: string, type: TypeAlerte = 'info') {
    const id = ++compteur
    alertes.value.push({ id, type, message })
    setTimeout(() => supprimer(id), 30_000)
  }

  function afficherSucces(message: string) {
    afficher(message, 'success')
  }

  function afficherErreur(message: string) {
    afficher(message, 'error')
  }

  function afficherAvertissement(message: string) {
    afficher(message, 'warning')
  }

  function supprimer(id: number) {
    alertes.value = alertes.value.filter((a) => a.id !== id)
  }

  return { alertes, afficher, afficherSucces, afficherErreur, afficherAvertissement, supprimer }
})
