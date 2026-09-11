<template>
  <!-- Boutons d'accès direct d'une carte stratégie du dashboard (09/09,
       workflow) : caractéristiques (page définition) + réglages en modale.
       Ne rend rien si la stratégie n'a pas d'actions. Les modales vivent
       ici aussi — ModaleCadre stoppe la propagation pour ne jamais
       déclencher le clic carte. -->
  <div v-if="actions" class="flex items-center gap-1 flex-wrap">
    <button
      v-for="a in actions" :key="a.cle"
      class="text-[10px] font-semibold px-2 py-0.5 rounded-md border transition-colors whitespace-nowrap"
      :class="classeAction"
      :title="a.titre"
      @click.stop="surAction(a.cle)"
    >{{ a.label }}</button>

    <!-- Réglages en modale -->
    <SmcReglagesModales :ouverte="id === 'SMC' ? modaleSmc : null" @fermer="modaleSmc = null" />
    <StraddleReglagesModales :ouverte="id === 'straddle' ? modaleStraddle : null" @fermer="modaleStraddle = null" />
    <RocketsReglagesModales :ouverte="id === 'rockets' ? modaleRockets : null" @fermer="modaleRockets = null" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import SmcReglagesModales, { type ModaleSmc } from './SmcReglagesModales.vue'
import StraddleReglagesModales, { type ModaleStraddle } from './StraddleReglagesModales.vue'
import RocketsReglagesModales, { type ModaleRockets } from './RocketsReglagesModales.vue'

const props = defineProps<{ id: string }>()

const router = useRouter()
const modaleSmc = ref<ModaleSmc | null>(null)
const modaleStraddle = ref<ModaleStraddle | null>(null)
const modaleRockets = ref<ModaleRockets | null>(null)

const ROUTES_DEFINITION: Record<string, string> = {
  SMC: '/smc/definition',
  straddle: '/straddle/definition',
  rockets: '/rockets/definition',
}
const ROUTES_SCANNER: Record<string, string> = { rockets: '/rockets/scanner' }

interface ActionCarte { cle: string; label: string; titre: string }
const ACTIONS: Record<string, ActionCarte[]> = {
  SMC: [
    { cle: 'definition', label: '📐 Caractéristiques', titre: 'Les caractéristiques de la stratégie SMC' },
    { cle: 'parametres', label: '⚙️ Paramètres', titre: 'État, son Telegram, capital, risque' },
    { cle: 'niveaux', label: '💰 Niveaux de profits', titre: 'TP1/TP2/TP3, trailing, ventes partielles' },
    { cle: 'timeframes', label: '🕐 Choix des Timeframe/Asset', titre: 'Armement des couples générateurs de signaux' },
  ],
  straddle: [
    { cle: 'definition', label: '📐 Caractéristiques', titre: 'Les caractéristiques de la stratégie Straddle' },
    { cle: 'parametres', label: '⚙️ Paramètres', titre: 'État, son Telegram, capital, risque' },
    { cle: 'moteur', label: '🛠️ Paramètres moteur', titre: 'Minutage et risque (SL × ATR H1, trailing)' },
  ],
  rockets: [
    { cle: 'definition', label: '📐 Caractéristiques', titre: 'Les caractéristiques de la stratégie Rockets' },
    { cle: 'scanner', label: '🔭 Scanner', titre: 'Le scanner des candidats rockets' },
    { cle: 'parametres', label: '⚙️ Paramètres', titre: 'État, son Telegram, capital, risque' },
    { cle: 'moteur', label: '🛠️ Paramètres moteur', titre: 'Profil de risque, gestion et détection' },
  ],
}

const actions = computed(() => ACTIONS[props.id] ?? null)

/// Teinte des boutons = celle de la carte.
const CLASSES_ACTIONS: Record<string, string> = {
  SMC: 'bg-blue-500/15 hover:bg-blue-500/30 text-blue-100 border-blue-400/25',
  straddle: 'bg-amber-500/15 hover:bg-amber-500/30 text-amber-100 border-amber-400/25',
  rockets: 'bg-orange-500/15 hover:bg-orange-500/30 text-orange-100 border-orange-400/25',
}
const classeAction = computed(() => CLASSES_ACTIONS[props.id] ?? 'bg-white/10 hover:bg-white/20 text-white border-white/20')

function surAction(cle: string) {
  if (cle === 'definition') {
    const cible = ROUTES_DEFINITION[props.id]
    if (cible) router.push(cible)
    return
  }
  if (cle === 'scanner') {
    const cible = ROUTES_SCANNER[props.id]
    if (cible) router.push(cible)
    return
  }
  if (props.id === 'SMC') modaleSmc.value = cle as ModaleSmc
  else if (props.id === 'straddle') modaleStraddle.value = cle as ModaleStraddle
  else if (props.id === 'rockets') modaleRockets.value = cle as ModaleRockets
}
</script>
