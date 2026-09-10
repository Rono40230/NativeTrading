<template>
  <!-- Les trois modales de réglages SMC, ouvertes depuis la carte SMC du
       dashboard (09/09, workflow) : Paramètres (registre), Niveaux de
       profits, Choix des Timeframe/Asset. Chaque bloc embarque sa propre
       sauvegarde — le cadre ne fait qu'habiller. -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" :large="ouverte === 'timeframes'" @fermer="$emit('fermer')">
    <RegistreStrategieBloc v-if="ouverte === 'parametres'" id="SMC" libelle-risque="Risque par trade" />
    <SmcNiveauxBloc v-else-if="ouverte === 'niveaux'" />
    <SmcCouplesBloc v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import SmcNiveauxBloc from './SmcNiveauxBloc.vue'
import SmcCouplesBloc from './SmcCouplesBloc.vue'

export type ModaleSmc = 'parametres' | 'niveaux' | 'timeframes'

defineProps<{ ouverte: ModaleSmc | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleSmc, string> = {
  parametres: '⚙️ Paramètres — registre SMC',
  niveaux: '💰 Niveaux de profits — SMC',
  timeframes: '🕐 Choix des Timeframe / Asset — SMC',
}
</script>
