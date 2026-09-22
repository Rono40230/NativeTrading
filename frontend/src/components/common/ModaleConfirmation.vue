<template>
  <!-- Modale de confirmation designée (remplace window.confirm) —
       même design que ModaleCadre, avec boutons action/annulation. -->
  <div v-if="ouverte" class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-6"
    @click.self="$emit('annuler')">
    <div class="w-full max-w-md p-5 rounded-xl border shadow-2xl backdrop-blur-sm"
      :class="variant === 'danger' ? 'border-red-500/30 bg-slate-900' : 'border-white/15 bg-slate-900'">
      <!-- Titre avec icône -->
      <div class="flex items-start gap-3 mb-4">
        <span class="text-2xl shrink-0">{{ icone }}</span>
        <div class="flex-1 min-w-0">
          <h3 class="text-sm font-bold text-white leading-snug">{{ titre }}</h3>
          <p v-if="sousTitre" class="text-xs text-white/60 mt-1 leading-relaxed">{{ sousTitre }}</p>
        </div>
      </div>

      <!-- Corps : lignes de détail (diff, liste, etc.) -->
      <div v-if="lignes?.length" class="rounded-lg bg-white/5 border border-white/10 p-3 mb-4">
        <p v-for="(ligne, i) in lignes" :key="i" class="text-xs font-mono text-white/80 leading-relaxed">
          {{ ligne }}
        </p>
      </div>

      <!-- Message d'avertissement optionnel -->
      <p v-if="avertissement" class="text-[11px] text-amber-300/80 border-l-2 border-amber-400/40 pl-2 mb-4 leading-relaxed">
        {{ avertissement }}
      </p>

      <!-- Boutons -->
      <div class="flex gap-2 justify-end">
        <button
          class="px-4 py-2 rounded-lg text-xs font-semibold border border-white/10 bg-white/5 text-white hover:bg-white/10 transition-colors"
          @click="$emit('annuler')"
        >Annuler</button>
        <button
          class="px-4 py-2 rounded-lg text-xs font-semibold border transition-colors"
          :class="variant === 'danger'
            ? 'bg-red-600/30 text-red-300 border-red-500/40 hover:bg-red-600/40'
            : 'bg-emerald-600/30 text-emerald-300 border-emerald-500/40 hover:bg-emerald-600/40'"
          :disabled="enCours"
          @click="$emit('confirmer')"
        >{{ enCours ? '⏳…' : labelConfirmer }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  ouverte: boolean
  titre: string
  sousTitre?: string
  /** Lignes de détail (diff de réglages, liste d'éléments…) */
  lignes?: string[]
  /** Message d'avertissement en ambre (conséquences, irréversibilité…) */
  avertissement?: string
  /** Texte du bouton confirmer */
  labelConfirmer?: string
  /** danger = rouge (action destructrice), défaut = vert */
  variant?: 'default' | 'danger'
  /** Icône affichée à gauche du titre */
  icone?: string
  /** Désactive le bouton confirmer pendant un traitement */
  enCours?: boolean
}>()

defineEmits<{ 
  (e: 'confirmer'): void
  (e: 'annuler'): void 
}>()
</script>
