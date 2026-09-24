<template>
  <!-- Panneau au survol — le design de l'info-bulle de la courbe capital
       (P4 24/09) : remplace les title natifs des badges et boutons des
       cartes stratégie. Fermé = zéro coût ; ouvert = fixed + clamp aux
       bords, retourné sous le déclencheur si près du haut. -->
  <span class="inline-flex" @mouseenter="survol" @mouseleave="fermer" @focus="survol" @blur="fermer">
    <slot />
    <teleport to="body">
      <div
        v-if="ouvert"
        class="fixed z-50 pointer-events-none bg-slate-900/95 border border-blue-400/30 rounded-lg px-2.5 py-1.5 shadow-xl max-w-[280px]"
        :style="style"
      >
        <p v-if="titre" class="text-[10px] font-bold text-white leading-snug break-words">{{ titre }}</p>
        <p v-for="(l, i) in lignes" :key="i" class="text-[9px] font-bold text-white/85 leading-snug whitespace-pre-wrap break-words">{{ l }}</p>
        <slot name="contenu" />
      </div>
    </teleport>
  </span>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

// titre : ligne de titre (gras). texte : corps multi-lignes (\n = une ligne).
const props = defineProps<{ titre?: string; texte?: string }>()

const ouvert = ref(false)
const ancre = ref({ x: 0, y: 0, largeur: 0 })

function survol(e: FocusEvent | MouseEvent) {
  const r = (e.currentTarget as Element).getBoundingClientRect()
  ancre.value = { x: r.left + r.width / 2, y: r.top, largeur: r.width }
  ouvert.value = true
}
function fermer() {
  ouvert.value = false
}

const lignes = computed(() => (props.texte ?? '').split('\n').filter(l => l.length > 0))

/// Ancrage fixed : centré au-dessus (retourné dessous si près du haut),
/// borné aux bords — même logique que l'info-bulle capital.
const style = computed(() => {
  const demi = 140
  const x = Math.min(Math.max(ancre.value.x, demi + 8), window.innerWidth - demi - 8)
  const auDessus = ancre.value.y > 160
  return {
    top: `${auDessus ? ancre.value.y - 8 : ancre.value.y + 18}px`,
    left: `${x}px`,
    transform: `translate(-50%, ${auDessus ? '-100%' : '0'})`,
  }
})
</script>
