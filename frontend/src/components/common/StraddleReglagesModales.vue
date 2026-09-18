<template>
  <!-- Modales de réglages Straddle, ouvertes depuis la carte du dashboard
       (09/09, workflow) : Paramètres (registre) et Paramètres moteur. -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" @fermer="$emit('fermer')">
    <RegistreStrategieBloc v-if="ouverte === 'parametres'" id="straddle" libelle-risque="Risque par passe" />
    <StraddlePerimetreContenu v-else-if="ouverte === 'perimetre'" />
    <StraddleMoteurBloc v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import StraddleMoteurBloc from './StraddleMoteurBloc.vue'
import StraddlePerimetreContenu from './StraddlePerimetreContenu.vue'

export type ModaleStraddle = 'parametres' | 'perimetre' | 'moteur'

defineProps<{ ouverte: ModaleStraddle | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleStraddle, string> = {
  parametres: '⚙️ Paramètres — registre Straddle',
  perimetre: '🎯 Choix des Assets & créneaux — Straddle',
  moteur: '🛠️ Paramètres moteur — Straddle',
}
</script>
