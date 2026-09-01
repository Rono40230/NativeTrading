<template>
  <div class="flex-1 min-h-0 flex flex-col gap-3">
    <!-- En-tête : identité + définition + lexique (retour = bouton Dashboard de la barre de titre) -->
    <div class="glass-card px-4 py-3 flex items-center gap-3 shrink-0">
      <span class="text-xl leading-none">{{ icone }}</span>
      <h1 class="text-xl font-bold text-white truncate">{{ titre }}</h1>
      <span
        class="text-[10px] font-semibold px-2 py-0.5 rounded-full border shrink-0"
        :class="badgeClasse"
      >{{ etat }}</span>
      <div class="ml-auto flex gap-2 shrink-0">
        <button class="btn-sm" @click="router.push(routeDefinition)">📖 Définition</button>
        <button class="btn-sm" @click="lexiqueOuvert = true">📚 Lexique</button>
      </div>
    </div>

    <!-- Setups et trades en cours sur la même ligne ; historique dessous -->
    <div class="flex-1 min-h-0 flex flex-col gap-3 overflow-y-auto pr-0.5">
      <div class="grid grid-cols-6 gap-3 min-h-0">
        <section v-if="$slots.setups" class="glass-card px-4 py-3 flex flex-col min-h-0 col-span-1">
          <h2 class="text-xs uppercase text-gray-500 font-semibold tracking-wider mb-2 shrink-0">
            ⏳ Setups en attente <span class="text-gray-600 normal-case">— ce qui peut arriver</span>
          </h2>
          <slot name="setups" />
        </section>

        <section class="glass-card px-4 py-3 flex flex-col min-h-0 col-span-5">
          <h2 class="text-xs uppercase text-gray-500 font-semibold tracking-wider mb-2 shrink-0">
            🟢 {{ titreEncours ?? 'Trades en cours' }}
          </h2>
          <slot name="encours" />
        </section>
      </div>

      <section class="glass-card px-4 py-3 flex flex-col min-h-0">
        <h2 class="text-xs uppercase text-gray-500 font-semibold tracking-wider mb-2 shrink-0">
          📜 Historique des trades <span class="text-gray-600 normal-case">— ce qui est passé</span>
        </h2>
        <slot name="historique" />
      </section>
    </div>

    <!-- Lexique en modale (design du lexique SMC, gabarit de référence) -->
    <div
      v-if="lexiqueOuvert"
      class="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-6"
      @click.self="lexiqueOuvert = false"
    >
      <div class="w-full max-w-5xl max-h-[85vh] overflow-y-auto p-6 rounded-xl border border-white/15 bg-slate-900 shadow-2xl">
        <div class="flex items-center justify-between mb-4 sticky top-0">
          <h2 class="text-lg font-bold text-white">📚 Lexique {{ titre }}</h2>
          <button class="btn-sm" @click="lexiqueOuvert = false">✕ Fermer</button>
        </div>
        <LexiquePanel :source="lexique" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import LexiquePanel from '@/components/common/LexiquePanel.vue'

const props = withDefaults(defineProps<{
  titre: string
  icone: string
  etat?: string
  routeDefinition: string
  lexique?: 'smc' | 'straddle' | 'rockets'
  /// Titre de la section en cours (ex. « 3 signaux en cours »).
  titreEncours?: string
}>(), { etat: 'Observation', lexique: 'smc' })

const router = useRouter()
const lexiqueOuvert = ref(false)

const badgeClasse = computed(() =>
  props.etat === 'Officielle'
    ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
    : 'bg-amber-500/10 text-amber-400 border-amber-500/30',
)
</script>

<style scoped>
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
</style>
