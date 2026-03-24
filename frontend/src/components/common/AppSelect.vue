<template>
  <div ref="root" class="app-select-root" @keydown="onKeydown">
    <button
      type="button"
      class="app-select-btn"
      :aria-expanded="ouvert"
      @click="toggle"
    >
      <span class="truncate">{{ labelSelectionne }}</span>
      <svg class="app-select-chevron" :class="{ 'rotate-180': ouvert }" viewBox="0 0 20 20" fill="currentColor">
        <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd"/>
      </svg>
    </button>

    <Teleport to="body">
      <div
        v-if="ouvert"
        ref="dropdown"
        class="app-select-dropdown"
        :style="dropdownStyle"
      >
        <button
          v-for="opt in options"
          :key="String(opt.value)"
          type="button"
          class="app-select-option"
          :class="{ 'app-select-option--active': opt.value === modelValue }"
          @mousedown.prevent
          @click="choisir(opt.value)"
        >
          {{ opt.label }}
        </button>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'

interface Option {
  label: string
  value: string | number
}

const props = defineProps<{
  modelValue: string | number
  options: Option[]
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
}>()

const ouvert = ref(false)
const root = ref<HTMLElement | null>(null)
const dropdown = ref<HTMLElement | null>(null)
const dropdownStyle = ref<Record<string, string>>({})

const labelSelectionne = computed(
  () => props.options.find(o => o.value === props.modelValue)?.label ?? String(props.modelValue)
)

function positionner() {
  if (!root.value) return
  const rect = root.value.getBoundingClientRect()
  dropdownStyle.value = {
    position: 'fixed',
    top: `${rect.bottom + 4}px`,
    left: `${rect.left}px`,
    minWidth: `${rect.width}px`,
    zIndex: '9999',
  }
}

async function toggle() {
  ouvert.value = !ouvert.value
  if (ouvert.value) {
    await nextTick()
    positionner()
  }
}

function choisir(val: string | number) {
  emit('update:modelValue', val)
  ouvert.value = false
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') ouvert.value = false
}

function fermerSiExterieur(e: MouseEvent) {
  const cible = e.target as Node
  if (
    root.value && !root.value.contains(cible) &&
    dropdown.value && !dropdown.value.contains(cible)
  ) {
    ouvert.value = false
  }
}

onMounted(() => document.addEventListener('mousedown', fermerSiExterieur))
onBeforeUnmount(() => document.removeEventListener('mousedown', fermerSiExterieur))
</script>

<style scoped>
.app-select-root {
  position: relative;
  display: inline-block;
  min-width: 140px;
}

.app-select-btn {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: rgb(31 41 55); /* gray-800 */
  border: 1px solid rgba(255 255 255 / 0.1);
  border-radius: 0.5rem;
  color: #fff;
  font-size: 0.875rem;
  cursor: pointer;
  text-align: left;
  transition: border-color 0.15s;
}
.app-select-btn:hover {
  border-color: rgba(255 255 255 / 0.3);
}

.app-select-chevron {
  width: 1rem;
  height: 1rem;
  flex-shrink: 0;
  color: rgb(156 163 175); /* gray-400 */
  transition: transform 0.15s;
}

.app-select-dropdown {
  background: #ffffff;
  border: 1px solid #d1d5db; /* gray-300 */
  border-radius: 0.5rem;
  box-shadow: 0 10px 25px rgba(0 0 0 / 0.35);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.app-select-option {
  padding: 0.5rem 0.75rem;
  text-align: left;
  background: transparent;
  border: none;
  color: #111827; /* gray-900 — texte noir */
  font-size: 0.875rem;
  cursor: pointer;
  transition: background 0.1s;
}
.app-select-option:hover {
  background: #f3f4f6; /* gray-100 */
}
.app-select-option--active {
  color: #92400e; /* amber-800 — lisible sur blanc */
  background: #fef3c7; /* amber-100 */
  font-weight: 600;
}
</style>
