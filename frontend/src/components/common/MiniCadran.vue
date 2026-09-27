<template>
  <!-- MINI-CADRAN v3 (owner 26/09) : réplique miniature des jauges — bezel
       métallique, cadran sombre, arcs colorés, aiguille à contre-poids,
       graduations, fenêtre digitale. Le flex-1 vit sur le WRAPPER (pas dans
       PopoverInfo qui ne le transmet pas). -->
  <div class="flex-1 min-w-0 flex flex-col items-center">
    <PopoverInfo :titre="titre" :texte="texte">
      <div class="flex flex-col items-center gap-0.5 w-full cursor-help">
      <svg viewBox="0 0 140 82" class="w-full">
        <defs>
          <linearGradient id="mini-bezel" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#4b5563" />
            <stop offset="35%" stop-color="#1f2937" />
            <stop offset="65%" stop-color="#374151" />
            <stop offset="100%" stop-color="#111827" />
          </linearGradient>
          <radialGradient id="mini-cadran" cx="50%" cy="40%" r="65%">
            <stop offset="0%" stop-color="#111826" />
            <stop offset="78%" stop-color="#0a0e18" />
            <stop offset="100%" stop-color="#060910" />
          </radialGradient>
        </defs>

        <!-- Bezel : arc métallique -->
        <path d="M 12 70 A 58 58 0 0 1 128 70" fill="none" stroke="url(#mini-bezel)" stroke-width="4" />

        <!-- Cadran : face sombre -->
        <path d="M 14 70 A 56 56 0 0 1 126 70 Z" fill="url(#mini-cadran)" stroke="#000" stroke-width="0.8" />

        <!-- Zones colorées gravées -->
        <path v-for="(z, i) in zonesArc" :key="i"
          :d="z.path" fill="none" :stroke="z.couleur" stroke-width="4" opacity="0.85" />

        <!-- Graduations -->
        <g stroke="rgba(255,255,255,0.45)" stroke-width="0.8">
          <line v-for="n in 7" :key="'t'+n"
            :x1="polaire(44, angleGrad(n)).x" :y1="polaire(44, angleGrad(n)).y"
            :x2="polaire(n % 2 === 1 ? 38 : 40, angleGrad(n)).x" :y2="polaire(n % 2 === 1 ? 38 : 40, angleGrad(n)).y" />
        </g>

        <!-- Chiffres aux index -->
        <text v-for="n in [1, 4, 7]" :key="'l'+n"
          :x="polaire(32, angleGrad(n)).x" :y="polaire(32, angleGrad(n)).y + 2"
          text-anchor="middle" fill="rgba(255,255,255,0.4)"
          style="font-size: 6px; font-weight: 700; font-family: ui-monospace, monospace">{{ gradLabel(n) }}</text>

        <!-- Fenêtre digitale : score -->
        <rect x="49" y="52" width="42" height="14" rx="2" fill="#020409" stroke="rgba(255,255,255,0.15)" stroke-width="0.6" />
        <text x="70" y="62" text-anchor="middle" :class="couleurTexte"
          style="font-size: 9px; font-weight: 700; font-family: ui-monospace, monospace">{{ digital }}</text>

        <!-- Aiguille : pivot central, rotation animée -->
        <g :style="{ transform: `rotate(${angle}deg)`, transformOrigin: '70px 70px', transition: 'transform 800ms cubic-bezier(0.2,0.8,0.3,1)' }">
          <polygon points="70,22 71.8,70 68.2,70" fill="#f8fafc" stroke="#cbd5e1" stroke-width="0.3" />
          <polygon points="70,76 71.2,82 68.8,82" fill="#94a3b8" />
        </g>
        <circle cx="70" cy="70" r="3.5" fill="#1f2937" stroke="#9ca3af" stroke-width="0.8" />
        <circle cx="70" cy="70" r="1.2" fill="#e5e7eb" />
      </svg>

      <!-- Libellé → interprétation sur UNE ligne -->
      <span class="text-[8px] font-bold text-center leading-none whitespace-nowrap truncate w-full">
        <span class="uppercase tracking-wider text-white/60">{{ libelle }}</span>
        <span class="text-white/25 mx-0.5">→</span>
        <span :class="couleurInterp">{{ interpretation }}</span>
      </span>
      </div>
    </PopoverInfo>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import PopoverInfo from './PopoverInfo.vue'

export interface ZoneArc { debut: number; fin: number; couleur: string }
export interface InterpArc { seuil: number; texte: string; couleur: string }

const props = withDefaults(defineProps<{
  titre: string
  texte: string
  libelle: string
  valeur: number
  min: number
  max: number
  digital: string
  couleurTexte?: string
  zones: ZoneArc[]
  interpretations: InterpArc[]
}>(), { couleurTexte: 'fill-white' })

/// Position normalisée 0-1.
const t = computed(() => Math.max(0, Math.min(1, (props.valeur - props.min) / (props.max - props.min))))
const angle = computed(() => -90 + t.value * 180)

/// Centre du cadran : (70, 70), rayon des zones : 48.
const CX = 70, CY = 70, R = 48

function polaire(r: number, angleDeg: number) {
  const a = angleDeg * Math.PI / 180
  return { x: CX + r * Math.sin(a), y: CY - r * Math.cos(a) }
}

/// Chemin d'arc pour une zone (valeurs brutes → angles).
function arcZone(depuisVal: number, versVal: number): string {
  const a1 = ((depuisVal - props.min) / (props.max - props.min)) * 180 - 90
  const a2 = ((versVal - props.min) / (props.max - props.min)) * 180 - 90
  const d = polaire(R, a1), f = polaire(R, a2)
  return `M ${d.x.toFixed(1)} ${d.y.toFixed(1)} A ${R} ${R} 0 0 1 ${f.x.toFixed(1)} ${f.y.toFixed(1)}`
}

const zonesArc = computed(() =>
  props.zones.map(z => ({ couleur: z.couleur, path: arcZone(z.debut, z.fin) }))
)

/// Angle de la graduation n (1 à 7 : réparties sur 180°).
function angleGrad(n: number): number {
  return -90 + (n - 1) * 30
}

/// Libellé de graduation : min au 1, milieu au 4, max au 7.
function gradLabel(n: number): string {
  if (n === 1) return String(props.min)
  if (n === 4) return String(Math.round((props.min + props.max) / 2))
  if (n === 7) return String(props.max)
  return ''
}

/// Interprétation : la dernière dont seuil ≤ t (%).
const interp = computed(() => {
  const pct = t.value * 100
  const valides = props.interpretations.filter(i => i.seuil <= pct)
  return valides[valides.length - 1] ?? props.interpretations[0]
})
const interpretation = computed(() => interp.value?.texte ?? '')
const couleurInterp = computed(() => interp.value?.couleur ?? 'text-white/60')
</script>
