<template>
  <!-- COLONNE-INSTRUMENT (cockpit 24/09, dessin propriétaire) : une
       stratégie = un instrument autonome. Toute la hauteur à gauche : les
       commandes de l'ancienne carte étalées (aérées, justify-evenly),
       Simulation + Telegram en bas de marge ; à droite la jauge puis
       l'écran courbe. Les chiffres vivent dans la jauge (fenêtre $, ΣR,
       état, en cours) — l'écran ne raconte que l'histoire. Clic sur la
       carte = page de la stratégie (les boutons stoppent la propagation). -->
  <div
    class="flex gap-2 min-w-0 rounded-xl border p-2 cursor-pointer transition-all hover:brightness-110 hover:shadow-[0_0_18px_rgba(255,255,255,0.06)]"
    :class="teinte"
    :title="`Ouvrir la page ${nom}`"
    @click="router.push(route)"
  >

    <!-- Marge de commandes : pleine hauteur, boutons étalés -->
    <div class="flex flex-col gap-2 w-[132px] shrink-0">
      <ReglagesCarteBoutons :id="id" vertical />
      <div class="flex flex-col gap-1">
        <PopoverInfo texte="Laboratoire de simulation — tester des réglages sans jamais toucher aux chiffres officiels.">
          <button
            class="text-[9px] font-semibold px-1.5 py-0.5 rounded-md border border-teal-500/30 bg-teal-500/10 text-teal-300 hover:bg-teal-500/20 transition-colors text-left"
            @click.stop="router.push(`/simulation?strategie=${id}`)"
          >🧪 Simulation</button>
        </PopoverInfo>
        <PopoverInfo :texte="titreTelegram">
          <button
            class="text-[9px] font-semibold px-1.5 py-0.5 rounded-md border transition-colors text-left disabled:opacity-40"
            :class="notifications ? 'border-emerald-400/30 bg-emerald-500/10 text-emerald-300' : 'border-white/15 bg-white/5 text-white/60'"
            :disabled="bascule === id"
            @click.stop="basculerTelegram"
          >{{ notifications ? '🔔 Telegram ON' : '🔕 Telegram OFF' }}</button>
        </PopoverInfo>
      </div>
    </div>

    <!-- La colonne instrument : jauge puis écran -->
    <div class="flex-1 flex flex-col gap-1.5 min-w-0">
      <JaugeStrategie :id="id" :nom="nom" :icone="icone" class="min-w-0" />
      <EcranCourbe :id="id" />
    </div>

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

const etat = ref('—')
const notifications = ref(false)
const bascule = ref('')

const titreTelegram = computed(() => [
  `Telegram — messages d'imminence : ${notifications.value ? 'ACTIVÉS' : 'COUPÉS'} (clic pour ${notifications.value ? 'couper' : 'activer'}).`,
  "Condition complète d'envoi : réglage activé ET stratégie Officielle.",
  ...(etat.value !== 'Officielle' ? [`Ici état ${etat.value} → silencieux tant que la stratégie ne repasse pas Officielle.`] : []),
].join('\n'))

// PUT partiel du registre — l'envoi relit le drapeau à CHAQUE signal :
// effet immédiat, sans relance.
async function basculerTelegram() {
  bascule.value = props.id
  try {
    const res = await http.put<{ notifications: boolean }>(`/api/strategies/${props.id}`, {
      notifications: !notifications.value,
    })
    notifications.value = res.data.notifications
  } catch (e) {
    alerteStore.afficherErreur(`Telegram ${props.nom} : bascule échouée — ${(e as Error).message}`)
  }
  bascule.value = ''
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
