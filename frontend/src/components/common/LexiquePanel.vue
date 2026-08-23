<template>
  <div class="space-y-4">
    <!-- Recherche + filtres catégorie -->
    <div class="flex flex-col gap-2">
      <input
        v-model="recherche"
        type="text"
        placeholder="Rechercher un terme ou abréviation…"
        class="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white placeholder:text-gray-500 focus:outline-none focus:border-white/30"
      />
      <div class="flex gap-1.5 flex-wrap">
        <button
          v-for="key in toutes"
          :key="key"
          class="px-2.5 py-1 rounded-full text-xs font-medium transition-colors"
          :class="catActive === key ? 'bg-white/20 text-white' : 'bg-white/5 text-gray-400 hover:bg-white/10'"
          @click="catActive = key"
        >
          {{ key === 'tous' ? `Tous (${TERMES.length})` : CAT_LABELS[key].label }}
        </button>
      </div>
    </div>

    <!-- Grille de cartes -->
    <div v-if="filtres.length > 0" class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
      <div
        v-for="t in filtres"
        :key="t.abrev"
        class="rounded-xl border border-white/10 p-3.5 flex flex-col gap-1.5 cursor-pointer transition-colors"
        :class="selectionne?.abrev === t.abrev ? 'border-white/25 bg-white/8' : 'bg-white/5 hover:border-white/18 hover:bg-white/7'"
        @click="selectionne = selectionne?.abrev === t.abrev ? null : t"
      >
        <div class="flex items-start justify-between gap-2">
          <code class="font-bold text-white text-sm leading-tight">{{ t.abrev }}</code>
          <span class="text-[10px] px-2 py-0.5 rounded-full font-medium shrink-0 leading-5" :class="CAT_LABELS[t.cat].couleur">
            {{ CAT_LABELS[t.cat].label }}
          </span>
        </div>
        <p class="text-xs text-gray-400 font-medium leading-snug">{{ t.nom }}</p>

        <div v-if="selectionne?.abrev === t.abrev" class="expand-body mt-1 flex flex-col gap-3">
          <p class="text-xs text-gray-300 leading-relaxed">{{ t.def }}</p>
          <div
            v-if="t.svg"
            class="rounded-lg bg-black/30 border border-white/8 p-3 flex items-center justify-center"
            v-html="t.svg"
          />
          <p v-if="t.svg" class="text-[10px] text-gray-500 text-center -mt-1">Schéma illustratif</p>
        </div>

        <div class="flex justify-end mt-auto pt-1">
          <span class="text-[10px] select-none" :class="t.svg ? 'text-blue-500/70' : 'text-gray-600'">
            <template v-if="selectionne?.abrev === t.abrev">▲ Réduire</template>
            <template v-else-if="t.svg">▼ Détail avec graphique</template>
            <template v-else>▼ Détail</template>
          </span>
        </div>
      </div>
    </div>

    <div v-else class="text-center text-gray-500 py-10 text-sm">
      Aucun terme trouvé pour « {{ recherche }} »
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { TERMES, CAT_LABELS, type Categorie, type TermeSMC } from '@/data/lexique'

const recherche = ref('')
const catActive = ref<'tous' | Categorie>('tous')
const selectionne = ref<TermeSMC | null>(null)

const toutes = computed(() => ['tous', ...Object.keys(CAT_LABELS)] as ('tous' | Categorie)[])

const filtres = computed(() => {
  const q = recherche.value.toLowerCase().trim()
  return TERMES.filter(t => {
    const matchCat = catActive.value === 'tous' || t.cat === catActive.value
    const matchQ = !q
      || t.abrev.toLowerCase().includes(q)
      || t.nom.toLowerCase().includes(q)
      || t.def.toLowerCase().includes(q)
    return matchCat && matchQ
  })
})
</script>

<style scoped>
.expand-body {
  animation: expandIn 0.15s ease;
}
@keyframes expandIn {
  from { opacity: 0; transform: translateY(-4px); }
  to   { opacity: 1; transform: translateY(0); }
}
</style>
