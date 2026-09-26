<template>
  <!-- Commandes en tuiles-icônes d'une colonne-instrument (dessin
       propriétaire 25/09) : chaque bouton devient une icône cliquable qui
       évoque son thème, posée en flanc de jauge. bords="gauche"/"droite"
       rend la moitié correspondante de la liste (équilibrage automatique) ;
       l'étiquette complète vit dans le popover au survol, le détail dans
       son corps. Les modales ne vivent que dans l'instance gauche (pas de
       doublon). ModaleCadre stoppe la propagation pour ne jamais
       déclencher le clic carte. -->
  <div v-if="moitie" class="flex flex-col justify-center gap-2 shrink-0">
    <PopoverInfo v-for="a in moitie" :key="a.cle" :titre="a.label" :texte="a.titre">
      <button
        class="w-11 h-11 rounded-lg border text-[21px] leading-none flex items-center justify-center transition-all hover:scale-110 cursor-pointer"
        :class="classeAction"
        :aria-label="a.label"
        @click.stop="surAction(a.cle)"
      >{{ a.label.split(' ')[0] }}</button>
    </PopoverInfo>

    <!-- Réglages en modale (instance gauche uniquement) -->
    <template v-if="bords !== 'droite'">
      <SmcReglagesModales :ouverte="id === 'SMC' ? modaleSmc : null" @fermer="modaleSmc = null" />
      <StraddleReglagesModales :ouverte="id === 'straddle' ? modaleStraddle : null" @fermer="modaleStraddle = null" />
      <RocketsReglagesModales :ouverte="id === 'rockets' ? modaleRockets : null" @fermer="modaleRockets = null" />
      <KdjReglagesModales :ouverte="id === 'kdj_halftrend' ? modaleKdj : null" @fermer="modaleKdj = null" />
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import PopoverInfo from './PopoverInfo.vue'
import SmcReglagesModales, { type ModaleSmc } from './SmcReglagesModales.vue'
import StraddleReglagesModales, { type ModaleStraddle } from './StraddleReglagesModales.vue'
import RocketsReglagesModales, { type ModaleRockets } from './RocketsReglagesModales.vue'
import KdjReglagesModales, { type ModaleKdj } from './KdjReglagesModales.vue'

const props = defineProps<{ id: string; bords: 'gauche' | 'droite' }>()

const router = useRouter()
const modaleSmc = ref<ModaleSmc | null>(null)
const modaleStraddle = ref<ModaleStraddle | null>(null)
const modaleRockets = ref<ModaleRockets | null>(null)
const modaleKdj = ref<ModaleKdj | null>(null)

const ROUTES_DEFINITION: Record<string, string> = {
  SMC: '/smc/definition',
  straddle: '/straddle/definition',
  rockets: '/rockets/definition',
  kdj_halftrend: '/kdj/definition',
}
const ROUTES_SCANNER: Record<string, string> = { SMC: '/smc/scanner', rockets: '/rockets/scanner', kdj_halftrend: '/kdj/scanner' }

interface ActionCarte { cle: string; label: string; titre: string }
const ACTIONS: Record<string, ActionCarte[]> = {
  SMC: [
    { cle: 'definition', label: '📐 Caractéristiques', titre: 'Les caractéristiques de la stratégie SMC' },
    { cle: 'scanner', label: '🔭 Scanner', titre: 'Setups en formation, confirmation H1/H4, journal' },
    { cle: 'parametres', label: '⚙️ Paramètres', titre: 'État, son Telegram, capital, risque' },
    { cle: 'niveaux', label: '💰 Niveaux de profits', titre: 'TP1/TP2/TP3, trailing, ventes partielles' },
    { cle: 'timeframes', label: '🕐 Assets & TimeFrame', titre: 'Armement des couples générateurs de signaux' },
  ],
  straddle: [
    { cle: 'definition', label: '📐 Caractéristiques', titre: 'Les caractéristiques de la stratégie Straddle' },
    { cle: 'perimetre', label: '🎯 Assets & créneaux', titre: 'Périmètre de surveillance (moteurs M1, annonces) et créneaux armés' },
    { cle: 'parametres', label: '⚙️ Paramètres', titre: 'État, son Telegram, capital, risque' },
    { cle: 'moteur', label: '🛠️ Paramètres moteur', titre: 'Minutage et risque (SL × ATR H1, trailing)' },
  ],
  rockets: [
    { cle: 'definition', label: '📐 Caractéristiques', titre: 'Les caractéristiques de la stratégie Rockets' },
    { cle: 'scanner', label: '🔭 Scanner', titre: 'Le scanner des candidats rockets' },
    { cle: 'parametres', label: '⚙️ Paramètres', titre: 'État, son Telegram, capital, risque' },
    { cle: 'moteur', label: '🛠️ Paramètres moteur', titre: 'Profil de risque, gestion et détection' },
  ],
  kdj_halftrend: [
    { cle: 'definition', label: '📐 Caractéristiques', titre: 'Les caractéristiques de la stratégie KDJ/Halftrend' },
    { cle: 'scanner', label: '🔭 Scanner', titre: 'Tendances franches vs ranges (ADX)' },
    { cle: 'parametres', label: '⚙️ Paramètres', titre: 'État, son Telegram, capital, risque' },
    { cle: 'moteur', label: '🛠️ Paramètres moteur', titre: 'KDJ, HalfTrend, RatioRisk, filtre ADX' },
    { cle: 'assets', label: '🕐 Assets', titre: 'Actifs armés pour le moteur KDJ H1 (sans redémarrage)' },
  ],
}

/// Liste complète d'une stratégie : ses accès + le labo de simulation
/// (dernier de la pile — l'équilibrage le place en bas à droite).
const liste = computed(() => {
  const base = ACTIONS[props.id]
  if (!base) return null
  return [...base, {
    cle: 'simulation',
    label: '🧪 Simulation',
    titre: 'Laboratoire de simulation — tester des réglages sans jamais toucher aux chiffres officiels.',
  }]
})

/// La moitié demandée : ceil(n/2) à gauche, le reste à droite — cartes
/// équilibrées de part et d'autre de la jauge.
const moitie = computed(() => {
  const l = liste.value
  if (!l) return null
  const coupe = Math.ceil(l.length / 2)
  return props.bords === 'gauche' ? l.slice(0, coupe) : l.slice(coupe)
})

/// Teinte des tuiles = celle de la carte.
const CLASSES_ACTIONS: Record<string, string> = {
  SMC: 'bg-blue-500/15 hover:bg-blue-500/30 border-blue-400/25',
  straddle: 'bg-amber-500/15 hover:bg-amber-500/30 border-amber-400/25',
  rockets: 'bg-orange-500/15 hover:bg-orange-500/30 border-orange-400/25',
  kdj_halftrend: 'bg-cyan-500/15 hover:bg-cyan-500/30 border-cyan-400/25',
}
const classeAction = computed(() => CLASSES_ACTIONS[props.id] ?? 'bg-white/10 hover:bg-white/20 border-white/20')

function surAction(cle: string) {
  if (cle === 'simulation') {
    router.push(`/simulation?strategie=${props.id}`)
    return
  }
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
  else if (props.id === 'kdj_halftrend') modaleKdj.value = cle as ModaleKdj
}
</script>
