<template>
  <!-- Périmètre de simulation (virtuel — ne touche pas à l'armement) :
       chips paires + timeframes, extraites de SimulationSmcPanel (limite
       600 lignes). Vide = toutes les paires armées / tous les TF. -->
  <div class="flex flex-col gap-1.5">
    <p class="text-[10px] font-semibold uppercase tracking-wider text-white/60">Paires simulées <span class="font-normal normal-case text-white/40">— vide = toutes les armées</span></p>
    <div class="flex flex-wrap gap-1">
      <button v-for="a in assetsDispo" :key="'pa' + a"
              class="text-[10px] px-1.5 py-0.5 rounded border font-mono transition-colors"
              :class="assets.includes(a) ? 'border-teal-400/50 bg-teal-500/20 text-white' : 'border-white/10 bg-white/[0.03] text-white/50 hover:border-white/25'"
              @click="basculer('assets', a)">{{ a }}</button>
    </div>
    <p class="text-[10px] font-semibold uppercase tracking-wider text-white/60 mt-1">Timeframes simulés</p>
    <div class="flex flex-wrap gap-1">
      <button v-for="tf in TFS" :key="'pt' + tf"
              class="text-[10px] px-1.5 py-0.5 rounded border font-mono transition-colors"
              :class="tfs.includes(tf) ? 'border-teal-400/50 bg-teal-500/20 text-white' : 'border-white/10 bg-white/[0.03] text-white/50 hover:border-white/25'"
              @click="basculer('tfs', tf)">{{ tf }}</button>
    </div>
  </div>
</template>

<script setup lang="ts">
const props = defineProps<{ assetsDispo: string[] }>()

const assets = defineModel<string[]>('assets', { required: true })
const tfs = defineModel<string[]>('tfs', { required: true })

/// TF générateurs SMC (périmètre figé du labo SMC).
const TFS = ['M1', 'M5', 'M15', 'M30']

function basculer(cible: 'assets' | 'tfs', v: string) {
  if (cible === 'assets') {
    assets.value = assets.value.includes(v) ? assets.value.filter(x => x !== v) : [...assets.value, v]
  } else {
    tfs.value = tfs.value.includes(v) ? tfs.value.filter(x => x !== v) : [...tfs.value, v]
  }
}
</script>
