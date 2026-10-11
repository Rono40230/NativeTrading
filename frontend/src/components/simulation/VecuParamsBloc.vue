<template>
  <!-- 📋 En production actuellement (owner 11/10) : les paramètres RÉELS du
       vécu, lecture seule, sous les KPI. Les données viennent du parent
       (déjà chargées pour la grille virtuelle) — ce bloc est pur affichage. -->
  <div class="rounded-lg border border-white/10 bg-black/20 px-3 py-2.5 flex flex-col gap-1.5">
    <div class="flex items-center gap-2">
      <p class="text-[10px] font-bold uppercase tracking-wider text-white/70">📋 En production actuellement</p>
      <button class="ml-auto text-[9px] px-2 py-0.5 rounded bg-white/10 hover:bg-teal-500/20 hover:text-teal-300 text-white/70 transition-colors"
              title="Copie ces paramètres dans la grille virtuelle (colonne centrale) pour en partir"
              @click="$emit('copier')">⧉ Copier dans le virtuel</button>
    </div>
    <div class="flex flex-wrap gap-1.5">
      <span v-for="l in lignes" :key="l.label"
            class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-white/5 border border-white/10 text-white/85"
            :title="l.title ?? `${l.label} : ${l.valeur}`">{{ l.label }} <b class="text-white">{{ l.valeur }}</b></span>
    </div>
    <p v-if="surcharges?.length" class="text-[9px] text-amber-300/90">
      Surcharges actives : {{ surcharges.join(' · ') }}
    </p>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  lignes: { label: string; valeur: string; title?: string }[]
  surcharges?: string[]
}>()
defineEmits<{ (e: 'copier'): void }>()
</script>
