<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import StraddleMonitoringML from '@/components/common/StraddleMonitoringML.vue'
import RocketsMonitoringML from '@/components/common/RocketsMonitoringML.vue'
import SmcMonitoringML from '@/components/common/SmcMonitoringML.vue'
import MlInsightsView from '@/views/MlInsightsView.vue'

type PromptsGroupe = Record<string, Record<string, any>>
type StrMap = Record<string, string>
type BoolMap = Record<string, boolean>

const ongletActif = ref('prompts')
const prompts = ref<PromptsGroupe | null>(null)
const chargement = ref(false)
const erreur = ref('')
const expansions = ref<BoolMap>({})
const editValues = ref<StrMap>({})
const enCours = ref<BoolMap>({})

const onglets = [
  { id: 'prompts',      label: '📝 Prompts IA' },
  { id: 'metriques',    label: '📉 Métriques ML' },
  { id: 'ml_insights',  label: '🤖 Dashboard LLM' },
]

const catConfig: Record<string, any> = {
  straddle: { label: 'Straddle', icon: '⚡', border: 'border-purple-500/30', text: 'text-purple-400' },
  smc: { label: 'SMC Directionnel', icon: '📊', border: 'border-blue-500/30', text: 'text-blue-400' },
  rockets: { label: 'Rockets', icon: '🚀', border: 'border-orange-500/30', text: 'text-orange-400' },
  outils_ia: { label: 'Outils IA', icon: '🧠', border: 'border-emerald-500/30', text: 'text-emerald-400' }
}

async function chargerPrompts() {
  chargement.value = true
  erreur.value = ''
  try {
    const rep = await apiService.getPrompts()
    prompts.value = rep
    for (const groupe of Object.values(rep) as Record<string, any>[]) {
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
    await apiService.putPrompt(promptId, editValues.value[promptId])
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
    await apiService.deletePrompt(promptId)
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
  <div class="flex flex-col h-full p-6 gap-5 bg-[#0a0e27]">

    <!-- En-tête -->
    <div class="shrink-0">
      <h1 class="text-xl font-bold text-white">Configuration & Métriques IA</h1>
      <p class="text-sm text-white mt-1">
        Gérez vos prompts centraux et analysez l'état du réseau ML.
      </p>
    </div>

    <!-- Onglets principaux -->
    <div class="flex gap-1 border-b border-white/10 shrink-0 flex-wrap">
      <button
        v-for="o in onglets"
        :key="o.id"
        @click="ongletActif = o.id"
        :class="ongletActif === o.id
          ? 'border-b-2 border-blue-500 text-white bg-white/5'
          : 'text-white hover:text-white hover:bg-white/5'"
        class="px-4 py-2 text-sm font-medium rounded-t transition-colors"
      >
        {{ o.label }}
      </button>
    </div>

    <!-- Erreur globale -->
    <div v-if="erreur" class="text-red-400 text-sm p-3 rounded-lg bg-red-500/10 border border-red-500/20 shrink-0">
      {{ erreur }}
    </div>

    <!-- CONTENU PROMPTS -->
    <div v-if="ongletActif === 'prompts'" class="flex-1 min-h-0 overflow-y-auto custom-scrollbar pr-2">
      <div v-if="chargement" class="text-white text-sm animate-pulse">Chargement des prompts…</div>
      
      <div v-else-if="prompts" class="grid grid-cols-1 xl:grid-cols-2 gap-4 h-full">
        <!-- 4 Blocs de catégories -->
        <div v-for="(catDef, catKey) in catConfig" :key="catKey" 
             class="glass-card p-4 flex flex-col gap-3 rounded-xl border bg-white/5"
             :class="catDef.border">
             
          <h2 class="font-bold flex items-center gap-2 text-base" :class="catDef.text">
            <span>{{ catDef.icon }}</span> {{ catDef.label }}
          </h2>
          
          <div class="flex flex-col gap-3 overflow-y-auto custom-scrollbar pr-1 flex-1 min-h-0">
            <template v-if="prompts[catKey]">
              <div
                v-for="(prompt, cle) in prompts[catKey]"
                :key="cle"
                class="rounded-lg border overflow-hidden shrink-0 bg-black/20"
                :class="prompt.modifie ? 'border-orange-500/30' : 'border-white/10'"
              >
                <!-- En-tête cliquable -->
                <div
                  class="flex items-center justify-between px-3 py-2 cursor-pointer hover:bg-white/5 transition-colors"
                  @click="basculer(prompt.id)"
                >
                  <div class="flex items-center gap-2 min-w-0 flex-wrap">
                    <span class="text-white text-sm font-medium">{{ prompt.label }}</span>
                    <span class="text-[10px] px-1.5 py-0.5 rounded bg-blue-500/15 text-blue-300 border border-blue-500/20">
                      {{ prompt.usage }}
                    </span>
                    <span v-if="prompt.modifie" class="text-[10px] px-1.5 py-0.5 rounded bg-orange-500/15 text-orange-300 border border-orange-500/20">
                      ✏ modifié
                    </span>
                  </div>
                  <span class="text-white text-xs ml-2 shrink-0">
                    {{ expansions[prompt.id] ? '▲' : '▼' }}
                  </span>
                </div>

                <!-- Description -->
                <p class="px-3 pb-2 text-[11px] text-white">
                  {{ prompt.description }}
                </p>

                <!-- Zone d'édition -->
                <div v-if="expansions[prompt.id]" class="border-t border-white/10">
                  <textarea
                    v-model="editValues[prompt.id]"
                    class="w-full text-[11px] text-white font-mono p-3 resize-y outline-none border-0 bg-black/60"
                    style="line-height: 1.6; min-height: 150px;"
                    spellcheck="false"
                  />
                  <div class="flex items-center justify-end gap-2 px-3 py-2 border-t border-white/5 bg-black/40">
                    <button
                      v-if="prompt.modifie"
                      @click="restaurer(prompt.id)"
                      :disabled="enCours[prompt.id]"
                      class="px-2 py-1 text-[10px] rounded bg-orange-500/15 text-orange-300 border border-orange-500/20 hover:bg-orange-500/25 transition-colors disabled:opacity-40"
                    >
                      ↩ Défaut
                    </button>
                    <button
                      @click="sauvegarder(prompt.id)"
                      :disabled="enCours[prompt.id] || editValues[prompt.id] === prompt.contenu"
                      class="px-2 py-1 text-[10px] rounded bg-emerald-600/20 text-emerald-400 border border-emerald-500/20 hover:bg-emerald-600/30 transition-colors disabled:opacity-40"
                    >
                      {{ enCours[prompt.id] ? '…' : '💾 Sauvegarder' }}
                    </button>
                  </div>
                </div>
              </div>
            </template>
            <div v-else class="text-xs text-white italic p-2">Aucun prompt trouvé.</div>
          </div>

        </div>
      </div>
    </div>

    <!-- CONTENUS METRIQUES & INSIGHTS -->
    <div v-if="ongletActif === 'metriques'" class="flex-1 min-h-0 overflow-y-auto custom-scrollbar">
      <div class="grid grid-cols-1 xl:grid-cols-3 gap-4 h-full">
        <!-- Straddle -->
        <div class="glass-card flex flex-col rounded-xl border border-purple-500/30 bg-white/5 overflow-hidden">
          <div class="p-4 border-b border-white/10 shrink-0">
             <h2 class="font-bold flex items-center gap-2 text-base text-purple-400"><span>⚡</span> Straddle</h2>
          </div>
          <div class="flex-1 min-h-0 overflow-y-auto p-4 custom-scrollbar relative">
             <StraddleMonitoringML compact />
          </div>
        </div>

        <!-- SMC -->
        <div class="glass-card flex flex-col rounded-xl border border-blue-500/30 bg-white/5 overflow-hidden">
          <div class="p-4 border-b border-white/10 shrink-0">
             <h2 class="font-bold flex items-center gap-2 text-base text-blue-400"><span>📊</span> SMC Directionnel</h2>
          </div>
          <div class="flex-1 min-h-0 overflow-y-auto p-4 custom-scrollbar relative">
             <SmcMonitoringML compact />
          </div>
        </div>

        <!-- Rockets -->
        <div class="glass-card flex flex-col rounded-xl border border-orange-500/30 bg-white/5 overflow-hidden">
          <div class="p-4 border-b border-white/10 shrink-0">
             <h2 class="font-bold flex items-center gap-2 text-base text-orange-400"><span>🚀</span> Rockets</h2>
          </div>
          <div class="flex-1 min-h-0 overflow-y-auto p-4 custom-scrollbar relative">
             <RocketsMonitoringML compact />
          </div>
        </div>
      </div>
    </div>

    <div v-else-if="ongletActif === 'ml_insights'" class="flex-1 min-h-0 overflow-y-auto custom-scrollbar p-1">
      <MlInsightsView />
    </div>

  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }
</style>
