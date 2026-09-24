<template>
  <!-- Choix des assets armés pour le moteur KDJ H1 (24/09). Même logique
       que l'armement des couples SMC : la sélection vit en config
       (kdj_assets_armes), lue par le runtime à chaque tick (60 s) — aucun
       redémarrage nécessaire. Absent = TOUS armés ; liste vide = aucun. -->
  <div class="flex flex-col gap-3">
    <p class="text-white/70 text-xs leading-relaxed">
      Actifs armés pour le moteur KDJ H1. Le balayage du labo (7.G) montre que l'écart entre actifs
      dépasse celui des paramètres — forex en profonde perte sur le rejeu 24 mois, XRP/XAG en tête.
      <span class="text-white/50">Application au prochain cycle runtime (~60 s), sans redémarrage.</span>
    </p>

    <div class="flex items-center gap-2 text-[10px]">
      <button class="px-2 py-0.5 rounded border border-white/15 bg-white/5 hover:bg-white/10 text-white"
              @click="tout(true)">Tout</button>
      <button class="px-2 py-0.5 rounded border border-white/15 bg-white/5 hover:bg-white/10 text-white"
              @click="tout(false)">Aucun</button>
      <span class="text-white/50">{{ selection.length }} / {{ assetsDispo.length }} armés</span>
      <span v-if="configAbsente" class="text-white/40 italic">— aucune config enregistrée : tous armés (état natif)</span>
    </div>

    <div class="flex flex-wrap gap-1 max-h-[40vh] overflow-y-auto">
      <button v-for="a in assetsDispo" :key="a"
              class="text-[10px] px-1.5 py-0.5 rounded border font-mono transition-colors"
              :class="selection.includes(a) ? 'border-cyan-400/50 bg-cyan-500/20 text-white' : 'border-white/10 bg-white/[0.03] text-white/40 hover:border-white/25'"
              @click="basculer(a)">{{ a }}</button>
    </div>

    <p v-if="message" class="text-[11px]" :class="erreur ? 'text-red-400' : 'text-emerald-300'">{{ message }}</p>

    <div class="flex justify-end gap-2">
      <button class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-white/10 hover:bg-white/20 text-white" @click="$emit('fermer')">Fermer</button>
      <button class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-cyan-500/25 hover:bg-cyan-500/40 text-cyan-100 disabled:opacity-40"
              :disabled="enCours" @click="enregistrer">{{ enCours ? '⏳' : '✅ Enregistrer' }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { http } from '@/services/http.client'
import { useAssetsStore } from '@/stores/assets.store'

const emit = defineEmits<{ (e: 'fermer'): void }>()
const assetsStore = useAssetsStore()

const assetsDispo = ref<string[]>([])
const selection = ref<string[]>([])
const configAbsente = ref(true)
const enCours = ref(false)
const message = ref('')
const erreur = ref(false)

onMounted(async () => {
  if (!assetsStore.assets.length) await assetsStore.chargerAssets()
  assetsDispo.value = assetsStore.assets.map(a => a.id).sort()
  selection.value = [...assetsDispo.value]
  try {
    const res = await http.get('/api/config', { params: { cle: 'kdj_assets_armes' } })
    if (res.data?.valeur) {
      const liste: unknown = JSON.parse(res.data.valeur)
      if (Array.isArray(liste)) {
        selection.value = assetsDispo.value.filter(a => (liste as string[]).includes(a))
        configAbsente.value = false
      }
    }
  } catch {
    // absent = tous armés (état natif)
  }
})

function basculer(a: string) {
  selection.value = selection.value.includes(a)
    ? selection.value.filter(x => x !== a)
    : [...selection.value, a]
}

function tout(on: boolean) {
  selection.value = on ? [...assetsDispo.value] : []
}

async function enregistrer() {
  enCours.value = true
  message.value = ''
  try {
    await http.post('/api/config', { cle: 'kdj_assets_armes', valeur: JSON.stringify(selection.value) })
    configAbsente.value = false
    erreur.value = false
    message.value = `Enregistré : ${selection.value.length} actif(s) armé(s) — appliqué au prochain cycle runtime (~60 s).`
  } catch (e) {
    erreur.value = true
    message.value = `Échec : ${(e as Error).message}`
  } finally {
    enCours.value = false
  }
}
</script>
