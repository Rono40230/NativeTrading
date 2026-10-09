<template>
  <!-- Modale du conseil IA PAR ASSET (phase 3 réglages par asset). L'analyste
       local lit vécu + réglages courants + écart balayage de l'asset et
       PROPOSE — aucun bouton d'écriture ici, l'activation vit au labo avec
       ses garde-fous (règle des 30 par asset, relecture-vérif). -->
  <ModaleCadre v-if="ouverte && asset" :titre="`🤖 Conseil IA — ${asset}`" large @fermer="$emit('fermer')">
    <!-- En-tête : badges d'identité et de jugeabilité -->
    <div class="flex flex-wrap items-center gap-2 mb-3">
      <span class="text-[10px] px-2 py-0.5 rounded-full border border-white/20 bg-white/5 text-white font-mono font-bold">{{ strategie }}</span>
      <span
        class="text-[10px] px-2 py-0.5 rounded-full border font-mono font-bold"
        :class="analyse?.jugeable
          ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300'
          : 'border-amber-500/40 bg-amber-500/10 text-amber-300'"
        :title="`Règle des 30 trades PAR ASSET : ${analyse?.effectif ?? 0} clôture(s) sur ${asset}`"
      >{{ analyse?.jugeable ? `JUGEABLE · ${analyse.effectif} clôtures` : `NON JUGEABLE · ${analyse?.effectif ?? 0}/30` }}</span>
      <span
        v-if="analyse"
        class="text-[10px] px-2 py-0.5 rounded-full border font-mono font-bold"
        :class="analyse.confiance >= 70 ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300' : analyse.confiance >= 40 ? 'border-amber-500/40 bg-amber-500/10 text-amber-300' : 'border-white/20 bg-white/5 text-white'"
        title="Confiance que l'analyste accorde à son conseil"
      >{{ analyse.confiance }}/100</span>
      <span v-if="analyse && !enCours" class="text-[10px] text-white/60 ml-auto">
        {{ cache ? 'servi du cache du jour · ' : '' }}généré à {{ heure(analyse.generee_le) }}
      </span>
    </div>

    <!-- Chargement : l'analyste local met ~1 min -->
    <div v-if="enCours" class="py-8 text-center text-sm text-white">
      ⏳ L'analyste lit le vécu de {{ asset }}, ses réglages et le balayage… (~1 min)
    </div>

    <div v-else-if="erreur" class="rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-300">
      {{ erreur }}
    </div>

    <div v-else-if="analyse" class="flex flex-col gap-2.5">
      <!-- État de l'asset -->
      <div class="rounded-lg border border-white/15 bg-white/5 p-2.5">
        <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1">🧭 État de l'asset</p>
        <p class="text-[12px] text-white leading-relaxed">{{ analyse.etat }}</p>
      </div>

      <!-- LE conseil : l'encadré central -->
      <div class="rounded-lg border border-teal-500/30 bg-teal-500/10 p-2.5">
        <p class="text-[10px] font-bold uppercase tracking-wider text-teal-300 mb-1">→ Conseil proposé (tu décides seul)</p>
        <p class="text-[12px] text-white leading-relaxed">{{ analyse.conseil || '—' }}</p>
      </div>

      <!-- Chiffres décisoires : chaque chiffre avec sa phrase d'explication
           (fabriqués par le moteur — mêmes sources que la page Analyse). -->
      <div v-if="analyse.chiffres_cles.length" class="rounded-lg border border-white/10 bg-white/[0.03] p-2.5">
        <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1.5">🔢 Chiffres clés</p>
        <div class="flex flex-col gap-2">
          <div v-for="(c, i) in analyse.chiffres_cles" :key="i">
            <p class="text-[11px] font-mono text-white leading-snug">{{ c.chiffre }}</p>
            <p class="text-[10px] text-white/50 leading-snug mt-0.5">{{ c.explication }}</p>
          </div>
        </div>
      </div>

      <p class="text-[9px] text-white/60">
        L'analyste lit le vécu ($ réels composés + R distance), les réglages actuels (surcharge ou défaut global) et le
        balayage du labo — il propose, ne décide jamais (constitution du 24/08). Pour appliquer un réglage :
        Simulation → chip {{ asset }} seule → balayer → ⚡ Activer (écrit dans la surcharge de {{ asset }}).
        Cache du jour — Régénérer refait tourner l'analyste demain ou après un nouveau balayage.
      </p>
    </div>
  </ModaleCadre>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import ModaleCadre from '@/components/common/ModaleCadre.vue'
import { genererConseilAsset, type AnalyseIaAsset } from '@/composables/useAnalyses'

const props = defineProps<{ ouverte: boolean; strategie: string; asset: string }>()
defineEmits<{ (e: 'fermer'): void }>()

const analyse = ref<AnalyseIaAsset | null>(null)
const enCours = ref(false)
const erreur = ref('')
const cache = ref(false)

/// Heure HH:MM depuis un epoch secondes.
function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

watch(
  () => [props.ouverte, props.asset] as const,
  async ([ouverte, asset]) => {
    if (!ouverte || !asset) return
    analyse.value = null
    erreur.value = ''
    enCours.value = true
    const res = await genererConseilAsset(props.strategie, asset)
    enCours.value = false
    if (res) {
      analyse.value = res.analyse
      cache.value = res.en_cache
    } else {
      erreur.value = 'Analyste indisponible — vérifier que le serveur Ollama est démarré.'
    }
  },
  { immediate: true },
)
</script>
