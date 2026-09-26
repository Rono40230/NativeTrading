<template>
  <!-- COLONNE-INSTRUMENT (cockpit 24/09, dessin propriétaire 25/09) : une
       stratégie = un instrument autonome. La jauge CENTRÉE, ses tuiles
       icônes de commande équilibrées de part et d'autre, l'étiquette sous
       la jauge (dans la jauge), l'écran courbe dessous sur toute la
       largeur. Pastille Telegram sur le coin de la carte (vert = activé,
       rouge = coupé). Clic sur la carte = page de la stratégie (les tuiles
       et la pastille stoppent la propagation). -->
  <div
    class="relative flex flex-col gap-1.5 min-w-0 rounded-xl border p-2 cursor-pointer transition-all hover:brightness-110 hover:shadow-[0_0_18px_rgba(255,255,255,0.06)]"
    :class="teinte"
    :title="`Ouvrir la page ${nom}`"
    @click="router.push(route)"
  >

    <!-- Pastille Telegram : commande du canal, posée sur le coin de la
         carte (décision owner 25/09). -->
    <PopoverInfo titre="Telegram — messages d'imminence" :texte="titreTelegram">
      <button
        class="absolute -top-2 -right-2 z-10 w-[28px] h-[28px] rounded-full flex items-center justify-center border-2 shadow-lg transition-all hover:scale-110 disabled:opacity-50"
        :class="notifications
          ? 'bg-emerald-500 border-emerald-200/50 shadow-emerald-500/40'
          : 'bg-red-500 border-red-200/50 shadow-red-500/40'"
        :disabled="bascule"
        :aria-label="notifications ? 'Telegram activé — cliquer pour couper' : 'Telegram coupé — cliquer pour activer'"
        @click.stop="basculerTelegram"
      >
        <svg viewBox="0 0 24 24" class="w-[16px] h-[16px]" aria-hidden="true">
          <path fill="#fff" d="M9.04 15.51l-.38 5.36c.54 0 .78-.23 1.06-.5l2.55-2.44 5.28 3.87c.97.53 1.66.25 1.92-.9L23.9 4.6c.31-1.42-.5-1.98-1.45-1.63L2.7 10.3c-1.39.54-1.37 1.32-.24 1.67l5.05 1.57L19.5 6.2c.55-.36 1.05-.16.64.2z" />
        </svg>
      </button>
    </PopoverInfo>

    <!-- Rang instrument : tuiles équilibrées | jauge | tuiles -->
    <div class="flex items-center justify-center gap-2 min-w-0">
      <ReglagesCarteBoutons :id="id" bords="gauche" />
      <JaugeStrategie :id="id" :nom="nom" :icone="icone" class="min-w-0" />
      <ReglagesCarteBoutons :id="id" bords="droite" />
    </div>

    <!-- L'écran : l'histoire en $, toute la largeur -->
    <EcranCourbe :id="id" />

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { http } from '@/services/http.client'
import PopoverInfo from './PopoverInfo.vue'
import ReglagesCarteBoutons from './ReglagesCarteBoutons.vue'
import JaugeStrategie from './JaugeStrategie.vue'
import EcranCourbe from './EcranCourbe.vue'
import { useAlerteStore } from '@/stores/alerte.store'

const props = defineProps<{
  id: string
  nom: string
  icone: string
  route: string
  teinte: string
}>()

const router = useRouter()
const alerteStore = useAlerteStore()

// ── Pastille Telegram (drapeau du registre, PUT partiel) ───────────────────
const etat = ref('—')
const notifications = ref(false)
const bascule = ref(false)

/// Info-bulle de la pastille : la règle d'envoi complète — le drapeau ET
/// l'état (Observation = silencieux, décision 15/09 ; le clic pré-règle
/// le drapeau).
const titreTelegram = computed(() => [
  `État : ${notifications.value ? 'ACTIVÉS' : 'COUPÉS'} (clic pour ${notifications.value ? 'couper' : 'activer'}).`,
  "Condition complète d'envoi : réglage activé ET stratégie Officielle.",
  ...(etat.value !== 'Officielle' ? [`Ici état ${etat.value} → silencieux tant que la stratégie ne repasse pas Officielle.`] : []),
].join('\n'))

// L'envoi relit le drapeau à CHAQUE signal : effet immédiat, sans relance.
async function basculerTelegram() {
  bascule.value = true
  try {
    const res = await http.put<{ notifications: boolean }>(`/api/strategies/${props.id}`, {
      notifications: !notifications.value,
    })
    notifications.value = res.data.notifications
  } catch (e) {
    alerteStore.afficherErreur(`Telegram ${props.nom} : bascule échouée — ${(e as Error).message}`)
  }
  bascule.value = false
}

async function chargerRegistre() {
  try {
    const r = await http.get('/api/strategies')
    const s = (r.data as { id: string; etat: string; notifications: boolean }[]).find(x => x.id === props.id)
    if (s) { etat.value = s.etat; notifications.value = s.notifications }
  } catch { /* silencieux */ }
}
onMounted(() => { void chargerRegistre() })
</script>
