<template>
  <span
    class="inline-flex items-center align-middle ml-1.5 cursor-help"
    @mouseenter="afficher"
    @mouseleave="masquer"
  >
    <svg
      class="w-3.5 h-3.5 transition-colors duration-150"
      :class="visible ? 'text-blue-400' : 'text-gray-500 hover:text-blue-400'"
      viewBox="0 0 20 20" fill="currentColor"
    >
      <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a.75.75 0 000 1.5h.253a.25.25 0 01.244.304l-.459 2.066A1.75 1.75 0 0010.747 15H11a.75.75 0 000-1.5h-.253a.25.25 0 01-.244-.304l.459-2.066A1.75 1.75 0 009.253 9H9z" clip-rule="evenodd" />
    </svg>
    <Teleport v-if="visible" to="body">
      <div
        class="fixed z-[9999] w-72 pointer-events-none -translate-x-1/2"
        :style="{ top: `${pos.top}px`, left: `${pos.left}px` }"
      >
        <!-- Flèche pointant vers l'icône (au-dessus du tooltip) -->
        <div class="flex justify-center mb-0">
          <div class="w-0 h-0 border-l-[6px] border-r-[6px] border-b-[6px] border-l-transparent border-r-transparent border-b-slate-900" />
        </div>
        <div class="rounded-xl border border-white/10 bg-slate-900/95 backdrop-blur-md shadow-2xl shadow-black/60 overflow-hidden">
          <!-- Description -->
          <div class="px-4 pt-3.5 pb-3" :class="niveaux?.length ? 'border-b border-white/5' : ''">
            <p class="text-[12px] text-white leading-relaxed">{{ texte }}</p>
          </div>
          <!-- Échelle interprétative -->
          <div v-if="niveaux?.length" class="px-3 py-2.5 space-y-1.5">
            <div
              v-for="n in niveaux" :key="n.label"
              class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg"
              :class="n.actif ? [CLS[n.couleur]?.bg, CLS[n.couleur]?.ring] : 'opacity-35'"
            >
              <span class="w-2 h-2 rounded-full flex-shrink-0" :class="CLS[n.couleur]?.dot ?? 'bg-gray-400'" />
              <span
                class="text-[11px] leading-tight flex-1"
                :class="n.actif ? (CLS[n.couleur]?.text ?? 'text-white') + ' font-semibold' : 'text-gray-400'"
              >{{ n.label }}</span>
              <span v-if="n.actif" class="text-[9px] text-gray-300 font-medium tracking-wide">← actuel</span>
            </div>
          </div>
        </div>
      </div>
    </Teleport>
  </span>
</template>

<script setup lang="ts">
import { ref } from 'vue'

export interface Niveau { label: string; couleur: string; actif: boolean }

defineProps<{ texte: string; niveaux?: Niveau[] }>()

const CLS: Record<string, { bg: string; ring: string; dot: string; text: string }> = {
  emerald: { bg: 'bg-emerald-500/15', ring: 'ring-1 ring-emerald-500/30', dot: 'bg-emerald-400', text: 'text-emerald-300' },
  yellow:  { bg: 'bg-yellow-500/15',  ring: 'ring-1 ring-yellow-500/30',  dot: 'bg-yellow-400',  text: 'text-yellow-300' },
  red:     { bg: 'bg-red-500/15',     ring: 'ring-1 ring-red-500/30',     dot: 'bg-red-400',     text: 'text-red-300'    },
  blue:    { bg: 'bg-blue-500/15',    ring: 'ring-1 ring-blue-500/30',    dot: 'bg-blue-400',    text: 'text-blue-300'   },
  gray:    { bg: 'bg-gray-500/15',    ring: 'ring-1 ring-gray-500/30',    dot: 'bg-gray-400',    text: 'text-gray-300'   },
}

const visible = ref(false)
const pos = ref({ top: 0, left: 0 })

function afficher(e: MouseEvent) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
  pos.value = { top: rect.bottom + 8, left: rect.left + rect.width / 2 }
  visible.value = true
}
function masquer() { visible.value = false }
</script>
