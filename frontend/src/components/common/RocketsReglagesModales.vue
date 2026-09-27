<template>
  <!-- Modale de réglages Rockets (fusion 26/09 : registre + moteur empilés
       en une seule). -->
  <ModaleCadre v-if="ouverte" titre="⚙️ Paramètres — Rockets (registre + moteur)" @fermer="$emit('fermer')">
    <RegistreStrategieBloc id="rockets" libelle-risque="Risque par rocket" />
    <div class="border-t border-white/10 my-3" />
    <RocketsMoteurBloc />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import RocketsMoteurBloc from './RocketsMoteurBloc.vue'

export type ModaleRockets = 'parametres'

defineProps<{ ouverte: ModaleRockets | null }>()
defineEmits<{ (e: 'fermer'): void }>()
</script>
