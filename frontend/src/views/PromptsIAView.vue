<script setup lang="ts">
import { ref, onMounted } from 'vue'
import axios from 'axios'

type PromptsGroupe = Record<string, Record<string, unknown>>
type StrMap = Record<string, string>
type BoolMap = Record<string, boolean>

const BASE_URL = 'http://localhost:8080'

const ongletActif = ref('rockets')
const prompts = ref<PromptsGroupe | null>(null)
const chargement = ref(false)
const erreur = ref('')
const expansions = ref<BoolMap>({})
const editValues = ref<StrMap>({})
const enCours = ref<BoolMap>({})

const onglets = [
  { id: 'rockets',   label: '🚀 Stratégie Rockets' },
  { id: 'smc',       label: '📊 Stratégie SMC' },
  { id: 'straddle',  label: '⚡ Stratégie Volatilité' },
  { id: 'outils_ia', label: '🧠 Outils IA' },
]

async function chargerPrompts() {
  chargement.value = true
  erreur.value = ''
  try {
    const rep = await axios.get(`${BASE_URL}/api/prompts`)
    prompts.value = rep.data
    for (const groupe of Object.values(rep.data) as Record<string, any>[]) {
      for (const prompt of Object.values(groupe) as any[]) {
        editValues.value[prompt.id] = prompt.contenu
      }
    }
  } catch (err: any) {
    erreur.value = `Échec chargement : ${err.message}`
  } finally {
    chargement.value = false
  }
}

function basculer(id: string) {
  expansions.value[id] = !expansions.value[id]
}

async function sauvegarder(promptId: string) {
  enCours.value[promptId] = true
  try {
    await axios.put(`${BASE_URL}/api/prompts/${promptId}`, {
      contenu: editValues.value[promptId]
    })
    await chargerPrompts()
  } catch (err: any) {
    erreur.value = `Échec sauvegarde : ${err.message}`
  } finally {
    enCours.value[promptId] = false
  }
}

async function restaurer(promptId: string) {
  enCours.value[promptId] = true
  try {
    await axios.delete(`${BASE_URL}/api/prompts/${promptId}`)
    await chargerPrompts()
  } catch (err: any) {
    erreur.value = `Échec restauration : ${err.message}`
  } finally {
    enCours.value[promptId] = false
  }
}

onMounted(chargerPrompts)
</script>

<template>
  <div class="flex flex-col h-full p-6 gap-5" style="background: #0a0e27;">

    <!-- En-tête -->
    <div>
      <h1 class="text-xl font-bold text-white">Prompts IA</h1>
      <p class="text-sm text-gray-400 mt-1">
        Prompts utilisés par l'IA locale (Ollama) — modifiables et persistants
      </p>
    </div>

    <!-- Onglets -->
    <div class="flex gap-2 flex-wrap shrink-0">
      <button
        v-for="o in onglets"
        :key="o.id"
        @click="ongletActif = o.id"
        :class="[
          'px-4 py-2 rounded-lg text-sm font-medium transition-colors',
          ongletActif === o.id
            ? 'bg-blue-600 text-white shadow-lg shadow-blue-500/20'
            : 'bg-white/5 text-gray-400 hover:bg-white/10 hover:text-white'
        ]"
      >
        {{ o.label }}
      </button>
    </div>

    <!-- Erreur globale -->
    <div v-if="erreur" class="text-red-400 text-sm p-3 rounded-lg bg-red-500/10 border border-red-500/20 shrink-0">
      {{ erreur }}
    </div>

    <!-- Chargement -->
    <div v-if="chargement" class="text-gray-400 text-sm animate-pulse">
      Chargement des prompts…
    </div>

    <!-- Liste des prompts de l'onglet actif -->
    <div v-else-if="prompts && prompts[ongletActif]" class="flex flex-col gap-3 overflow-y-auto pr-1 flex-1 min-h-0">
      <div
        v-for="(prompt, cle) in prompts[ongletActif]"
        :key="cle"
        class="rounded-xl border overflow-hidden shrink-0"
        :class="prompt.modifie ? 'border-orange-500/30' : 'border-white/10'"
        style="background: rgba(255,255,255,0.04);"
      >
        <!-- En-tête cliquable -->
        <div
          class="flex items-center justify-between px-4 py-3 cursor-pointer hover:bg-white/5 transition-colors"
          @click="basculer(prompt.id)"
        >
          <div class="flex items-center gap-2 min-w-0 flex-wrap">
            <span class="text-white font-medium">{{ prompt.label }}</span>
            <span class="text-xs px-2 py-0.5 rounded-full bg-blue-500/15 text-blue-300 border border-blue-500/20">
              {{ prompt.usage }}
            </span>
            <span v-if="prompt.modifie" class="text-xs px-2 py-0.5 rounded-full bg-orange-500/15 text-orange-300 border border-orange-500/20">
              ✏ modifié
            </span>
          </div>
          <span class="text-gray-500 text-xs ml-3 shrink-0">
            {{ expansions[prompt.id] ? '▲' : '▼' }}
          </span>
        </div>

        <!-- Description -->
        <p class="px-4 pb-3 text-sm text-gray-400 leading-relaxed">
          {{ prompt.description }}
        </p>

        <!-- Zone d'édition (expandable) -->
        <div v-if="expansions[prompt.id]" class="border-t border-white/10">
          <textarea
            v-model="editValues[prompt.id]"
            class="w-full text-xs text-gray-200 font-mono p-4 resize-y outline-none border-0"
            style="background: rgba(0,0,0,0.45); line-height: 1.65; min-height: 220px; max-height: 520px;"
            spellcheck="false"
          />
          <div class="flex items-center justify-end gap-2 px-4 py-2 border-t border-white/5">
            <button
              v-if="prompt.modifie"
              @click="restaurer(prompt.id)"
              :disabled="enCours[prompt.id]"
              class="px-3 py-1.5 text-xs rounded-lg bg-orange-500/15 text-orange-300 border border-orange-500/20 hover:bg-orange-500/25 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            >
              ↩ Restaurer défaut
            </button>
            <button
              @click="sauvegarder(prompt.id)"
              :disabled="enCours[prompt.id] || editValues[prompt.id] === prompt.contenu"
              class="px-3 py-1.5 text-xs rounded-lg bg-green-600/20 text-green-300 border border-green-500/20 hover:bg-green-600/30 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
            >
              {{ enCours[prompt.id] ? '…' : '💾 Sauvegarder' }}
            </button>
          </div>
        </div>

      </div>
    </div>

  </div>
</template>
