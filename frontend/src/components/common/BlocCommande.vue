<template>
  <!-- BLOC DE COMMANDE (redesign cartes 26/09, phase 1) : rectangle gravé
       de la colonne droite d'une carte stratégie — titre uppercase, fenêtre
       digitale (contenu vivant par slot, rempli en phases 2-3), clic porté
       par la carte. Rétro-éclairage teinté de la stratégie au survol. -->
  <button
    type="button"
    class="bloc flex-1 min-h-0 flex flex-col rounded-lg border transition-all text-left cursor-pointer"
    :class="teinte"
    @click.stop="$emit('clic')"
  >
    <span class="bloc-titre shrink-0">{{ titre }} <span class="opacity-30">ⓘ</span></span>
    <span class="bloc-fenetre flex-1 min-h-0 flex flex-col justify-center px-1.5 py-0.5">
      <slot />
    </span>
  </button>
</template>

<script setup lang="ts">
defineProps<{ titre: string; teinte: string }>()
defineEmits<{ (e: 'clic'): void }>()
</script>

<style scoped>
.bloc {
  background: rgba(255, 255, 255, 0.03);
  border-color: rgba(255, 255, 255, 0.1);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
  padding: 4px 6px;
  gap: 2px;
}
.bloc-titre {
  font-size: 9px;
  font-weight: 800;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.55);
  font-family: ui-monospace, monospace;
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.bloc-fenetre {
  background: #020409;
  border-radius: 3px;
  overflow: hidden;
}
</style>
