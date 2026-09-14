<template>
  <!-- Modales de réglages KDJ/Halftrend (7.E) : Paramètres (registre) et
       Paramètres moteur (les 4 réglages de l'étalon + filtre ADX). -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" @fermer="$emit('fermer')">
    <RegistreStrategieBloc v-if="ouverte === 'parametres'" id="kdj_halftrend" libelle-risque="Risque par trade" />
    <KdjMoteurBloc v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import KdjMoteurBloc from './KdjMoteurBloc.vue'

export type ModaleKdj = 'parametres' | 'moteur'

defineProps<{ ouverte: ModaleKdj | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleKdj, string> = {
  parametres: '⚙️ Paramètres — registre KDJ/Halftrend',
  moteur: '🛠️ Paramètres moteur — KDJ/Halftrend',
}
</script>
