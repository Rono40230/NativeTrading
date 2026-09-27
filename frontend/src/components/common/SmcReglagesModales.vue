<template>
  <!-- Modales de réglages SMC (fusion 26/09 : Paramètres = registre + niveaux
       de profits empilés — l'ancienne modale « niveaux » séparée a disparu).
       La modale Timeframe/Asset reste indépendante (bloc Assets). -->
  <ModaleCadre v-if="ouverte" :titre="TITRES[ouverte]" :large="ouverte === 'timeframes'" @fermer="$emit('fermer')">
    <template v-if="ouverte === 'parametres'">
      <RegistreStrategieBloc id="SMC" libelle-risque="Risque par trade" />
      <div class="border-t border-white/10 my-3" />
      <SmcNiveauxBloc />
    </template>
    <SmcCouplesBloc v-else />
  </ModaleCadre>
</template>

<script setup lang="ts">
import ModaleCadre from './ModaleCadre.vue'
import RegistreStrategieBloc from './RegistreStrategieBloc.vue'
import SmcNiveauxBloc from './SmcNiveauxBloc.vue'
import SmcCouplesBloc from './SmcCouplesBloc.vue'

export type ModaleSmc = 'parametres' | 'timeframes'

defineProps<{ ouverte: ModaleSmc | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const TITRES: Record<ModaleSmc, string> = {
  parametres: '⚙️ Paramètres — SMC (registre + niveaux de profits)',
  timeframes: '🕐 Choix des Timeframe / Asset — SMC',
}
</script>
