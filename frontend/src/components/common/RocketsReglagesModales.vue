<template>
  <!-- Modales de réglages Rockets, ouvertes depuis la carte du dashboard
       (09/09, workflow) : Paramètres (registre) et Paramètres moteur. -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" @fermer="$emit('fermer')">
    <RegistreStrategieBloc v-if="ouverte === 'parametres'" id="rockets" libelle-risque="Risque par rocket" />
    <RocketsMoteurBloc v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import RocketsMoteurBloc from './RocketsMoteurBloc.vue'

export type ModaleRockets = 'parametres' | 'moteur'

defineProps<{ ouverte: ModaleRockets | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleRockets, string> = {
  parametres: '⚙️ Paramètres — registre Rockets',
  moteur: '🛠️ Paramètres moteur — Rockets',
}
</script>
