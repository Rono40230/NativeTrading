<template>
  <!-- Modales de réglages Straddle (fusion 26/09 : Paramètres = registre +
       moteur empilés). La modale périmètre reste indépendante (bloc Assets). -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" @fermer="$emit('fermer')">
    <template v-if="ouverte === 'parametres'">
      <RegistreStrategieBloc id="straddle" libelle-risque="Risque par passe" />
      <div class="border-t border-white/10 my-3" />
      <StraddleMoteurBloc />
    <div class="border-t border-white/10 my-3" />
    <ReglagesAssetBloc strategie="straddle" />

    </template>
    <StraddlePerimetreContenu v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import StraddleMoteurBloc from './StraddleMoteurBloc.vue'
import ReglagesAssetBloc from './ReglagesAssetBloc.vue'
import StraddlePerimetreContenu from './StraddlePerimetreContenu.vue'

export type ModaleStraddle = 'parametres' | 'perimetre'

defineProps<{ ouverte: ModaleStraddle | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleStraddle, string> = {
  parametres: '⚙️ Paramètres — Straddle (registre + moteur)',
  perimetre: '🎯 Choix des Assets & créneaux — Straddle',
}
</script>
