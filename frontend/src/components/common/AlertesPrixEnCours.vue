<template>
  <div class="glass-card flex flex-col p-2 min-h-0">
    <div class="shrink-0 mb-1 border-b border-white/10 pb-1 flex items-center justify-between">
      <p class="text-xs uppercase font-bold text-white whitespace-nowrap truncate">🔔 Alertes prix en cours</p>
      <span v-if="alertes.length" class="text-amber-400 text-[10px] font-semibold">{{ alertes.length }}</span>
    </div>

    <div v-if="!alertes.length" class="text-[10px] text-gray-600 leading-snug">
      Aucune alerte active — posez-les sur les graphiques (clic droit sur le prix)
    </div>

    <div v-else class="flex flex-col gap-1 flex-1 min-h-0 overflow-y-auto pr-0.5">
      <div
        v-for="a in alertes"
        :key="a.id"
        class="flex items-center gap-1 bg-white/5 rounded px-1.5 py-1 shrink-0"
        :title="a.note ? a.note : titreAlerte(a)"
      >
        <span class="text-[11px]">{{ a.sens === 'en_dessous' ? '🔻' : '🔺' }}</span>
        <span class="text-[11px] font-semibold text-white truncate">{{ a.asset }}</span>
        <span class="text-[11px] font-mono text-amber-300 whitespace-nowrap">{{ formaterPrix(a.prix) }}</span>
        <button
          class="ml-auto text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-blue-600/60 text-gray-300 hover:text-white transition-colors whitespace-nowrap"
          title="Ouvrir le graphique de cet asset"
          @click.stop="voirAlerte(a)"
        >👁</button>
        <button
          class="text-[9px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-red-600/60 text-gray-300 hover:text-white transition-colors whitespace-nowrap"
          title="Supprimer l'alerte (bloc + graphique)"
          @click.stop="supprimerAlerte(a)"
        >✕</button>
      </div>
    </div>

    <!-- Déclenchées récemment : rappel gris, les 3 dernières -->
    <div v-if="declenchees.length" class="shrink-0 mt-1 pt-1 border-t border-white/10">
      <p class="text-[8px] uppercase text-gray-600 mb-0.5">Déclenchées (24 h)</p>
      <div
        v-for="a in declenchees"
        :key="a.id"
        class="flex items-center gap-1.5 px-1.5 py-0.5"
        :title="titreAlerte(a)"
      >
        <span class="text-[10px] text-gray-500">{{ a.sens === 'en_dessous' ? '🔻' : '🔺' }}</span>
        <span class="text-[10px] text-gray-500 truncate">{{ a.asset }}</span>
        <span class="ml-auto text-[10px] text-gray-600 font-mono">{{ formaterPrix(a.prix) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useSettingsStore } from '@/stores/settings.store'
import { alertesApi } from '@/services/api.alertes'
import { ciblerPremierSlot } from '@/utils/graphiques'

interface AlertePrix {
  id: number
  asset: string
  prix: number
  sens: string
  note: string | null
  /** L'API sérialise en booléen (true/false), la table en 1/0 — les deux acceptés. */
  active: boolean | number
  cree_le: number
  declenchee_le: number | null
}

const router = useRouter()
const settingsStore = useSettingsStore()
const alertesToutes = ref<AlertePrix[]>([])

/** Voir : cible l'asset de l'alerte dans le premier slot de la grille
 *  sauvegardée puis ouvre la page Graphiques (utilitaire partagé). */
function voirAlerte(a: AlertePrix) {
  settingsStore.assetActif = a.asset
  ciblerPremierSlot(a.asset)
  router.push('/smc/graphiques')
}

/** Supprimer : l'API efface l'alerte ; les graphiques retirent leur ligne
 *  à leur prochain poll (10 s) — le bloc se rafraîchit immédiatement. */
async function supprimerAlerte(a: AlertePrix) {
  try {
    await alertesApi.supprimer(a.id)
    await charger()
  } catch { /* suppression impossible : le prochain poll réaffichera l'état réel */ }
}

const alertes = computed(() => alertesToutes.value.filter(a => !!a.active))
const declenchees = computed(() =>
  alertesToutes.value
    .filter(a => !a.active && a.declenchee_le && Date.now() / 1000 - a.declenchee_le < 86400)
    .sort((a, b) => (b.declenchee_le ?? 0) - (a.declenchee_le ?? 0))
    .slice(0, 3),
)

function formaterPrix(p: number): string {
  if (p >= 1000) return new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 }).format(p)
  if (p >= 1) return p.toFixed(2)
  return p.toFixed(4)
}

function titreAlerte(a: AlertePrix): string {
  return `${a.asset} — ${a.sens === 'en_dessous' ? 'descente sous' : 'montée au-dessus de'} ${formaterPrix(a.prix)}${a.note ? ` · ${a.note}` : ''}`
}

async function charger() {
  try {
    alertesToutes.value = await alertesApi.lister()
  } catch {
    alertesToutes.value = []
  }
}

let poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void charger()
  poll = setInterval(charger, 30_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>
