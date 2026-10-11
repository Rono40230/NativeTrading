<template>
  <!-- 7 · Bibliothèque des essais (extraite de SimulationSmcPanel — limite
       600 lignes). Les essais vivent côté parent ; ce bloc est la vue :
       clic = recharger les paramètres, 🗑 = supprimer. -->
  <section class="glass-card p-4 flex flex-col gap-3">
    <div class="flex items-center gap-2">
      <h2 class="text-sm font-bold text-white uppercase tracking-wider">7 · Bibliothèque des essais</h2>
      <span class="ml-auto text-[10px] text-white/60">clic = recharge les paramètres · 🗑 = supprime</span>
    </div>
    <div v-if="essais.length" class="flex flex-col gap-1">
      <div v-for="e in essais" :key="e.id"
           class="flex items-center gap-3 text-xs rounded-lg px-3 py-2 bg-white/5 border border-white/10 hover:border-teal-400/40 cursor-pointer"
           @click="$emit('recharger', e.params)">
        <span class="text-white/60 font-mono shrink-0">{{ dateCourte(e.cree_le) }}</span>
        <span class="text-white truncate"><slot name="ligne" :essai="e">{{ e.id }}</slot></span>
        <span class="ml-auto font-mono shrink-0" :class="(e.resultat?.capital_actuel ?? 0) >= (e.resultat?.capital_depart ?? 1) ? 'text-emerald-400' : 'text-red-400'">
          {{ fmtDollars(e.resultat?.capital_actuel ?? 0) }} · {{ fmtR2(e.resultat?.r_total_pondere ?? 0) }}
        </span>
        <button class="text-white/50 hover:text-red-400 shrink-0" title="Supprimer l'essai"
                @click.stop="$emit('supprimer', e.id)">🗑</button>
      </div>
    </div>
    <p v-else class="text-xs text-white/60 py-2 text-center">Aucun essai encore — lance une simulation, elle sera conservée ici.</p>
  </section>
</template>

<script setup lang="ts">
export interface EssaiAffichable {
  id: string
  cree_le: number
  params: Record<string, unknown>
  resultat?: { capital_depart?: number; capital_actuel?: number; r_total_pondere?: number } | null
}

defineProps<{ essais: EssaiAffichable[] }>()
defineEmits<{
  (e: 'recharger', params: Record<string, unknown>): void
  (e: 'supprimer', id: string): void
}>()

function fmtDollars(v: number): string {
  return `${Math.round(v).toLocaleString('fr-FR')} $`
}
function fmtR2(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(2)} R`
}
function dateCourte(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString('fr-FR', { day: '2-digit', month: '2-digit' })
}
</script>
