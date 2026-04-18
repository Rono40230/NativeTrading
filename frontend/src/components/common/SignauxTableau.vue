<template>
  <div class="flex-1 min-h-0 flex flex-col gap-4">

    <!-- Filtres -->
    <div class="glass-card p-3 flex items-center gap-3 flex-wrap shrink-0">
      <span class="text-xs text-gray-500">{{ listeActive.length }} signal{{ listeActive.length !== 1 ? 's' : '' }}</span>
      <div class="flex gap-2 ml-auto">
        <button class="btn-sm" @click="charger">🔄 Actualiser</button>
        <button v-if="strategie !== 'Rockets'" class="btn-sm bg-purple-700 hover:bg-purple-600" @click="analyseOuverte = true">📊 Analyse</button>
      </div>
    </div>

    <!-- Tableau -->
    <div class="glass-card overflow-x-hidden overflow-y-auto flex-1 min-h-0">
      <div v-if="chargement && !listeActive.length" class="text-center text-gray-500 py-10">Chargement…</div>
      <div v-else-if="!listeActive.length" class="text-center text-gray-500 py-10">Aucun signal correspondant</div>
      <table v-else class="w-full text-sm">
        <thead>
          <tr class="text-gray-400 text-xs uppercase border-b border-white/10">
            <th class="px-3 py-3 text-left">#</th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('asset')">Asset <span>{{ icone('asset') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('timeframe')">TF / Phase <span>{{ icone('timeframe') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('direction')">Direction <span>{{ icone('direction') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('score')">Score <span>{{ icone('score') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_entree')">Entrée <span>{{ icone('prix_entree') }}</span></th>
            <th class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('stop_loss')">SL <span>{{ icone('stop_loss') }}</span></th>
            <th class="px-3 py-3 text-right">TP1</th>
            <th class="px-3 py-3 text-right">TP2</th>
            <th class="px-3 py-3 text-right">TP3</th>
            <th v-if="filtreStatut !== 'cloturees'" class="px-3 py-3 text-right">Prix actuel</th>
            <th v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-right cursor-pointer hover:text-white select-none" @click="trierPar('prix_verdict')">Sortie <span>{{ icone('prix_verdict') }}</span></th>
            <th class="px-3 py-3 text-center">IA</th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('verdict')">Résultat <span>{{ icone('verdict') }}</span></th>
            <th class="px-3 py-3 text-left cursor-pointer hover:text-white select-none" @click="trierPar('cree_le')">Ouvert le <span>{{ icone('cree_le') }}</span></th>
            <th v-if="strategie === 'SmcDirectional'" class="px-3 py-3 text-center w-10"></th>
            <th v-if="strategie === 'Rockets' && filtreStatut === 'en_cours'" class="px-3 py-3 text-center w-20">Annuler</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="(s, i) in signauxTries" :key="s.id">
          <tr class="border-b border-white/5 hover:bg-white/5 transition-colors">
            <td class="px-3 py-3 text-gray-500">{{ i + 1 }}</td>
            <td class="px-3 py-3 font-semibold text-white">{{ s.asset }}</td>
            <td class="px-3 py-3 text-gray-400">{{ s.timeframe }}</td>
            <td class="px-3 py-3">
              <span class="badge" :class="s.direction === 'LONG' ? 'badge-green' : s.direction === 'SHORT' ? 'badge-red' : 'badge-blue'">{{ s.direction }}</span>
            </td>
            <td class="px-3 py-3 text-right font-mono text-gray-300">{{ s.score.toFixed(0) }}</td>
            <td class="px-3 py-3 text-right font-mono text-white">{{ formatNombre(s.prix_entree) }}</td>
            <td class="px-3 py-3 text-right font-mono text-red-400">{{ formatNombre(s.stop_loss) }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-400">{{ formatNombre(s.take_profit[0]) }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-300">{{ s.take_profit[1] ? formatNombre(s.take_profit[1]) : '—' }}</td>
            <td class="px-3 py-3 text-right font-mono text-emerald-200">{{ s.take_profit[2] ? formatNombre(s.take_profit[2]) : '—' }}</td>
            <td v-if="filtreStatut !== 'cloturees'" class="px-3 py-3 text-right font-mono" :class="classePrix(s)">{{ prixStore.getPrix(s.asset) !== null ? formatNombre(prixStore.getPrix(s.asset)!) : '—' }}</td>
            <td v-if="filtreStatut !== 'en_cours'" class="px-3 py-3 text-right font-mono text-white">{{ s.prix_verdict ? formatNombre(s.prix_verdict) : '—' }}</td>
            <td class="px-3 py-3 text-center">
              <span v-if="s.llm_conviction !== null" class="inline-flex items-center justify-center w-8 h-8 rounded-full text-xs font-bold cursor-help" :class="classeConviction(s.llm_conviction)" :title="s.llm_raison ?? ''">{{ s.llm_conviction }}</span>
              <span v-else class="text-gray-700 text-xs">—</span>
            </td>
            <td class="px-3 py-3">
              <span class="badge" :class="classeResultat(s)">{{ labelResultat(s) }}</span>
            </td>
            <td class="px-3 py-3 text-gray-500 text-xs">{{ formatDate(s.cree_le) }}</td>
            <td v-if="strategie === 'SmcDirectional'" class="px-3 py-3 text-center">
              <button class="text-blue-400 hover:text-blue-200 text-sm transition-colors" title="Analyser ce signal avec l'IA" @click="analyserSignal(s)">🔍</button>
            </td>
            <td v-if="strategie === 'Rockets' && filtreStatut === 'en_cours' && s.statut !== 'Fermé'" class="px-3 py-3 text-center">
              <button
                class="text-xs px-2 py-1 rounded border border-red-700/50 bg-red-900/20 text-red-400 hover:bg-red-900/50 hover:text-red-300 transition-all disabled:opacity-30"
                :disabled="annulationEnCours.has(s.id)"
                @click="demanderAnnulation(s)"
              >{{ annulationEnCours.has(s.id) ? '…' : 'Annuler' }}</button>
            </td>
          </tr>
          <!-- Sous-ligne jambes Straddle : uniquement pour signaux actifs (sans verdict) -->
          <tr v-if="strategie === 'Straddle' && s.direction === 'Both' && s.verdict === null"
              :key="`${s.id}-legs`"
              class="border-b border-white/5 bg-white/2">
            <td colspan="99" class="px-4 pb-2 pt-0">
              <div class="flex items-center gap-3 text-[11px]">
                <span class="text-emerald-500 font-bold tracking-wide">LONG</span>
                <span class="text-gray-500">SL</span>
                <span class="font-mono text-red-300">{{ formatNombre(s.sl_long_effectif ?? s.stop_loss) }}</span>
                <span v-for="tp in ['tp1','tp2','tp3']" :key="`long-${tp}`"
                      class="font-mono"
                      :class="(s.tps_long_atteints ?? []).includes(tp) ? 'text-emerald-400' : 'text-gray-700'">
                  {{ tp.toUpperCase() }}{{ (s.tps_long_atteints ?? []).includes(tp) ? ' ✓' : '' }}
                </span>
                <span class="text-white/15 mx-1">┃</span>
                <span class="text-red-400 font-bold tracking-wide">SHORT</span>
                <span class="text-gray-500">SL</span>
                <span class="font-mono text-red-300">{{ formatNombre(s.sl_short_effectif ?? (s.sl_short ?? 0)) }}</span>
                <span v-for="tp in ['tp1','tp2','tp3']" :key="`short-${tp}`"
                      class="font-mono"
                      :class="(s.tps_short_atteints ?? []).includes(tp) ? 'text-emerald-400' : 'text-gray-700'">
                  {{ tp.toUpperCase() }}{{ (s.tps_short_atteints ?? []).includes(tp) ? ' ✓' : '' }}
                </span>
                <span v-if="lotPourSignal(s)" class="text-white/15 mx-1">┃</span>
                <span v-if="lotPourSignal(s)" class="text-yellow-400/70">Lot : <span class="font-mono font-bold text-yellow-300">{{ lotPourSignal(s) }}</span></span>                <template v-if="labelHeureEntree(s)">
                  <span class="text-white/15 mx-1">┃</span>
                  <span class="badge badge-yellow text-[10px]">{{ labelHeureEntree(s) }}</span>
                </template>              </div>
            </td>
          </tr>
          <!-- Sous-ligne lot SMC : uniquement pour signaux actifs -->
          <tr v-else-if="strategie === 'SmcDirectional' && s.verdict === null && lotPourSignal(s)"
              :key="`${s.id}-lot`"
              class="border-b border-white/5 bg-white/2">
            <td colspan="99" class="px-4 pb-2 pt-0">
              <div class="flex items-center gap-2 text-[11px]">
                <span class="text-yellow-400/70">Lot : <span class="font-mono font-bold text-yellow-300">{{ lotPourSignal(s) }}</span></span>
                <span class="text-white/15">—</span>
                <span class="text-gray-600">{{ (settingsStore.capitalDepart * ((assetParamsStore.liste.find(p => p.asset === s.asset)?.risque_pct ?? 0) / 100)).toFixed(0) }} $ risqués</span>
              </div>
            </td>
          </tr>
          </template>
        </tbody>
      </table>
    </div>

    <!-- Modales analyse -->
    <StraddleAnalyseModal v-if="strategie === 'Straddle'" :open="analyseOuverte" :signaux="signaux" @close="analyseOuverte = false" />
    <SmcAnalyseModal v-if="strategie === 'SmcDirectional'" :open="analyseOuverte" :signaux="signaux" @close="analyseOuverte = false" />
    <RocketsAnalyseModal v-if="strategie === 'Rockets'" :open="analyseOuverte" :rockets="rocketsRaw" @close="analyseOuverte = false" />

    <!-- Modale de confirmation annulation Rocket -->
    <Teleport to="body">
      <Transition name="modal-fade">
        <div v-if="signalAnnuler" class="fixed inset-0 z-50 flex items-center justify-center">
          <!-- Backdrop -->
          <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" @click="signalAnnuler = null" />
          <!-- Fenêtre -->
          <div class="relative z-10 w-80 rounded-xl border border-red-700/40 bg-[#0f1629] shadow-2xl p-5 flex flex-col gap-4">
            <div class="flex items-center gap-2">
              <span class="text-red-400 text-lg">⚠️</span>
              <span class="text-xs uppercase font-bold text-white">Confirmer l'annulation</span>
            </div>
            <div class="text-xs text-gray-400 space-y-1">
              <p>Tu vas annuler le trade suivant :</p>
              <div class="bg-white/5 rounded-lg px-3 py-2 space-y-1 border border-white/10">
                <div class="flex justify-between">
                  <span class="text-gray-500">Asset</span>
                  <span class="text-white font-bold font-mono">{{ signalAnnuler.asset }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-gray-500">Direction</span>
                  <span class="font-bold" :class="signalAnnuler.direction === 'LONG' ? 'text-emerald-400' : 'text-red-400'">{{ signalAnnuler.direction }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-gray-500">Entrée</span>
                  <span class="text-white font-mono">{{ formatNombre(signalAnnuler.prix_entree) }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-gray-500">SL</span>
                  <span class="text-red-300 font-mono">{{ formatNombre(signalAnnuler.stop_loss) }}</span>
                </div>
                <div class="flex justify-between">
                  <span class="text-gray-500">TP1</span>
                  <span class="text-emerald-300 font-mono">{{ formatNombre(signalAnnuler.take_profit[0]) }}</span>
                </div>
              </div>
              <p class="text-yellow-400/80 pt-1">Cette action est irréversible.</p>
            </div>
            <div class="flex gap-2 justify-end">
              <button
                class="text-xs px-3 py-1.5 rounded border border-white/10 bg-white/5 text-gray-400 hover:bg-white/10 hover:text-white transition-all"
                @click="signalAnnuler = null"
              >Garder</button>
              <button
                class="text-xs px-3 py-1.5 rounded border border-red-700/50 bg-red-900/30 text-red-400 hover:bg-red-900/60 hover:text-red-300 transition-all"
                @click="confirmerAnnulation"
              >Confirmer l'annulation</button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useSignauxTableau } from '@/composables/useSignauxTableau'
import { formatDate, formatNombre } from '@/composables/useSignalFormat'
import type { Signal } from '@/services/api.types'
import StraddleAnalyseModal from '@/components/common/StraddleAnalyseModal.vue'
import SmcAnalyseModal from '@/components/common/SmcAnalyseModal.vue'
import RocketsAnalyseModal from '@/components/RocketsAnalyseModal.vue'

const props = defineProps<{ strategie: 'SmcDirectional' | 'Straddle' | 'Rockets' }>()

const {
  signaux, rocketsRaw, chargement, analyseOuverte,
  filtreStatut, annulationEnCours, listeActive, signauxTries,
  charger, annuler, trierPar, icone, analyserSignal,
  classeConviction, classePrix, labelResultat, classeResultat, lotPourSignal,
  prixStore, assetParamsStore, settingsStore,
} = useSignauxTableau(props.strategie)

// ── Modale confirmation annulation ───────────────────────────────────────────
const signalAnnuler = ref<Signal | null>(null)

function demanderAnnulation(s: Signal) {
  signalAnnuler.value = s
}

async function confirmerAnnulation() {
  if (!signalAnnuler.value) return
  const s = signalAnnuler.value
  signalAnnuler.value = null
  await annuler(s)
}

// ── Heure d'entrée Straddle ───────────────────────────────────────────────────
function labelHeureEntree(s: Signal): string | null {
  if (!s.heure_entree) return null
  const resteSec = s.heure_entree - Math.floor(Date.now() / 1000)
  if (resteSec > 0) {
    const min = Math.ceil(resteSec / 60)
    return `⏱ dans ${min}min`
  }
  return 'Entrée active'
}
</script>

<style scoped>
.glass-card { @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm; }
.btn-sm { @apply bg-gray-700 hover:bg-gray-600 text-white text-sm px-3 py-1.5 rounded-lg transition-all; }
.filtre-btn { @apply text-xs px-3 py-1.5 rounded-lg border border-white/10 bg-white/5 text-gray-400 hover:bg-white/10 hover:text-white transition-all; }
.filtre-btn-actif { @apply bg-blue-600/30 border-blue-500/50 text-blue-300; }
.badge { @apply text-xs font-bold px-2 py-0.5 rounded-full; }
.badge-green  { @apply bg-emerald-900/60 text-emerald-300; }
.badge-red    { @apply bg-red-900/60 text-red-300; }
.badge-blue   { @apply bg-blue-900/60 text-blue-300; }
.badge-gray   { @apply bg-gray-700/60 text-gray-400; }
.badge-orange { @apply bg-orange-900/60 text-orange-400; }
.badge-yellow { @apply bg-yellow-900/60 text-yellow-300; }

.modal-fade-enter-active,
.modal-fade-leave-active { transition: opacity 0.15s ease; }
.modal-fade-enter-from,
.modal-fade-leave-to { opacity: 0; }
</style>
