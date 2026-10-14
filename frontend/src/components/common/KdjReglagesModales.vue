<template>
  <!-- Modales de réglages KDJ/Halftrend (fusion 26/09 : Paramètres = registre
       + moteur empilés). La modale choix des assets reste indépendante. -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" @fermer="$emit('fermer')">
    <template v-if="ouverte === 'parametres'">
      <RegistreStrategieBloc id="kdj_halftrend" libelle-risque="Risque par trade" />
      <div class="border-t border-white/10 my-3" />
      <KdjMoteurBloc />
    </template>
    <KdjAssetsBloc v-else @fermer="$emit('fermer')" />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import KdjMoteurBloc from './KdjMoteurBloc.vue'
import KdjAssetsBloc from './KdjAssetsBloc.vue'

export type ModaleKdj = 'parametres' | 'assets'

defineProps<{ ouverte: ModaleKdj | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleKdj, string> = {
  parametres: '⚙️ Paramètres — KDJ/Halftrend (registre + moteur)',
  assets: '🕐 Choix des assets — moteur KDJ H1',
}
</script>
