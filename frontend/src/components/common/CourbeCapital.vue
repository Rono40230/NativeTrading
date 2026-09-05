<template>
  <!-- Courbe bicolore au capital de départ : verte au-dessus de la ligne
       pointillée, rouge en dessous — le même tracé deux fois, chaque copie
       clippée par un rectangle calé sur le départ (la découpe suit
       l'étirement du graphique, la frontière tombe pile sur la ligne). -->
  <svg :viewBox="`0 0 ${LARGEUR} ${HAUTEUR}`" preserveAspectRatio="none" class="w-full h-full">
    <defs>
      <clipPath :id="`cap-haut-${idBloc}`"><rect x="0" y="0" :width="LARGEUR" :height="yDepart" /></clipPath>
      <clipPath :id="`cap-bas-${idBloc}`"><rect x="0" :y="yDepart" :width="LARGEUR" :height="HAUTEUR - yDepart" /></clipPath>
    </defs>
    <line x1="0" :x2="LARGEUR" :y1="yDepart" :y2="yDepart" stroke="rgba(255,255,255,0.4)" stroke-width="1" vector-effect="non-scaling-stroke" stroke-dasharray="4 3" />
    <polyline :points="points" fill="none" stroke="#34d399" stroke-width="1.5" vector-effect="non-scaling-stroke" stroke-linejoin="round" stroke-linecap="round" :clip-path="`url(#cap-haut-${idBloc})`" />
    <polyline :points="points" fill="none" stroke="#f87171" stroke-width="1.5" vector-effect="non-scaling-stroke" stroke-linejoin="round" stroke-linecap="round" :clip-path="`url(#cap-bas-${idBloc})`" />
  </svg>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { LARGEUR, HAUTEUR, pointsCapital, yDepartCapital, type CapitalCourbe } from '@/composables/useCourbeCapital'

const props = defineProps<{ capital: CapitalCourbe; idBloc: string }>()

const points = computed(() => pointsCapital(props.capital))
const yDepart = computed(() => yDepartCapital(props.capital))
</script>
