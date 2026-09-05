<template>
  <!-- Journal de bord d'un trade — fil de notes horodatées (append-only,
       suppression possible). Ouverte par l'icône 📝 de l'historique. -->
  <div v-if="ouvert" class="fixed inset-0 z-50 flex items-center justify-center bg-black/40" @click.self="$emit('fermer')">
    <div class="w-full max-w-lg p-5 space-y-3 rounded-2xl border border-white/10 bg-[#16181d] shadow-2xl">
      <div class="flex items-center justify-between">
        <div>
          <h3 class="font-bold text-white">📝 Journal de bord</h3>
          <p class="text-[11px] text-white">{{ titre }}</p>
        </div>
        <button class="text-white hover:text-white transition" @click="$emit('fermer')">✕</button>
      </div>

      <!-- Fil des entrées (chrono croissant) -->
      <div class="max-h-64 overflow-y-auto space-y-2 pr-1">
        <p v-if="!entrees.length" class="text-xs text-white py-3 text-center">
          Aucune note — décris le contexte, ton ressenti, ta décision.
        </p>
        <div v-for="e in entrees" :key="e.id" class="rounded-lg border border-white/10 bg-white/5 px-2.5 py-2 flex items-start gap-2">
          <span class="text-[10px] font-mono text-white/60 shrink-0 pt-0.5">{{ heure(e.cree_le) }}</span>
          <p class="text-xs text-white leading-relaxed flex-1 whitespace-pre-wrap">{{ e.note }}</p>
          <button
            class="text-white/40 hover:text-red-400 text-xs shrink-0 transition-colors"
            title="Supprimer cette entrée"
            @click="supprimer(e.id)"
          >✕</button>
        </div>
      </div>

      <!-- Nouvelle entrée -->
      <div class="flex flex-col gap-2 border-t border-white/10 pt-3">
        <textarea
          v-model="texte"
          rows="2" maxlength="2000"
          placeholder="Contexte, ressenti, décision…"
          class="bg-black/30 border border-white/10 rounded-lg px-2.5 py-2 text-xs text-white resize-none focus:outline-none focus:ring-1 focus:ring-blue-500/50"
          @keydown.enter.exact.prevent="ajouter()"
        ></textarea>
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-white/50">Entrée ajoutée par ⏎ — un journal ne se réécrit pas.</span>
          <button
            class="ml-auto px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold transition-colors disabled:opacity-40"
            :disabled="enCours || !texte.trim()"
            @click="ajouter()"
          >{{ enCours ? '…' : 'Ajouter' }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { http } from '@/services/http.client'

const props = defineProps<{
  ouvert: boolean
  signalId: string
  titre: string
}>()
const emit = defineEmits<{ fermer: []; note: [] }>()

interface Entree { id: number; signal_id: string; note: string; cree_le: number }
const entrees = ref<Entree[]>([])
const texte = ref('')
const enCours = ref(false)

function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleString('fr-FR', { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' })
}

async function charger() {
  try {
    const res = await http.get<{ entrees: Entree[] }>(`/api/journal/${props.signalId}`)
    entrees.value = res.data.entrees ?? []
  } catch {
    entrees.value = []
  }
}

watch(() => props.ouvert, (o) => { if (o) { texte.value = ''; void charger() } })

async function ajouter() {
  const t = texte.value.trim()
  if (!t || enCours.value) return
  enCours.value = true
  try {
    await http.post(`/api/journal/${props.signalId}`, { texte: t })
    texte.value = ''
    await charger()
    emit('note')
  } catch { /* silencieux : le fil réaffiche l'état réel */ } finally {
    enCours.value = false
  }
}

async function supprimer(id: number) {
  try {
    await http.delete(`/api/journal/entree/${id}`)
    await charger()
    emit('note')
  } catch { /* silencieux */ }
}
</script>
