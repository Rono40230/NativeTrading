<template>
  <!-- Modales de réglages KDJ/Halftrend (7.E + choix des assets 24/09) :
       Paramètres (registre), Paramètres moteur (étalon + filtre ADX) et
       Choix des assets (armement du moteur H1). -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" @fermer="$emit('fermer')">
    <RegistreStrategieBloc v-if="ouverte === 'parametres'" id="kdj_halftrend" libelle-risque="Risque par trade" />
    <KdjAssetsBloc v-else-if="ouverte === 'assets'" @fermer="$emit('fermer')" />
    <KdjMoteurBloc v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import KdjMoteurBloc from './KdjMoteurBloc.vue'
import KdjAssetsBloc from './KdjAssetsBloc.vue'

export type ModaleKdj = 'parametres' | 'moteur' | 'assets'

defineProps<{ ouverte: ModaleKdj | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleKdj, string> = {
  parametres: '⚙️ Paramètres — registre KDJ/Halftrend',
  moteur: '🛠️ Paramètres moteur — KDJ/Halftrend',
  assets: '🕐 Choix des assets — moteur KDJ H1',
}
</script>
