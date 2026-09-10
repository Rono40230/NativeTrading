<template>
  <!-- Modales de réglages Straddle, ouvertes depuis la carte du dashboard
       (09/09, workflow) : Paramètres (registre) et Paramètres moteur. -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" @fermer="$emit('fermer')">
    <RegistreStrategieBloc v-if="ouverte === 'parametres'" id="straddle" libelle-risque="Risque par passe" />
    <StraddleMoteurBloc v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import StraddleMoteurBloc from './StraddleMoteurBloc.vue'

export type ModaleStraddle = 'parametres' | 'moteur'

defineProps<{ ouverte: ModaleStraddle | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleStraddle, string> = {
  parametres: '⚙️ Paramètres — registre Straddle',
  moteur: '🛠️ Paramètres moteur — Straddle',
}
</script>
