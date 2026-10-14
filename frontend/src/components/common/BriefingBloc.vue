<template>
  <!-- Briefing du jour (owner 13/10 — KTA #11) : les TABLEAUX factuels sont
       calculés par l'app (zéro hallucination), le LLM ne produit que la
       hiérarchie « ce qui compte aujourd'hui ». Cache du jour côté backend. -->
  <div class="flex flex-col gap-3">
    <div class="flex items-center gap-2 flex-wrap">
      <span v-if="briefing?.synthese" class="text-[10px] font-mono px-1.5 py-0.5 rounded-full border font-bold"
            :class="(briefing.synthese.confiance ?? 0) >= 70 ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-300' : 'border-white/20 bg-white/5 text-white'">{{ briefing.synthese.confiance }}/100</span>
      <span v-if="enCache" class="text-[10px] text-white/40">cache du jour</span>
      <button class="ml-auto text-[10px] px-2.5 py-1 rounded-lg font-semibold transition-colors disabled:opacity-40"
              :class="enCours ? 'bg-white/10 text-white' : 'bg-sky-500/20 text-sky-300 hover:bg-sky-500/30'"
              :disabled="enCours" @click="generer(true)">
        {{ enCours ? '⏳ Analyse… (~1 min)' : briefing?.synthese ? '↻ Régénérer' : '⚡ Générer le briefing' }}</button>
    </div>

    <!-- Synthèse LLM : la hiérarchie du jour -->
    <div v-if="briefing?.synthese" class="rounded-lg border border-sky-500/25 bg-sky-500/5 p-2.5">
      <p class="text-[10px] font-bold uppercase tracking-wider text-sky-300 mb-1">🎯 Ce qui compte aujourd'hui</p>
      <ol class="text-[11px] text-white space-y-1 list-decimal list-inside leading-snug">
        <li v-for="(l, i) in briefing.synthese.ce_qui_compte" :key="i">{{ l }}</li>
      </ol>
      <p class="text-[11px] text-white/80 mt-2 leading-relaxed">{{ briefing.synthese.resume }}</p>
    </div>
    <p v-else-if="erreur" class="text-xs text-red-300">{{ erreur }}</p>

    <!-- Données factuelles (calcul app) — 2 colonnes (owner 14/10) :
         GAUCHE = événements du jour ; DROITE = annonces + positions + presse. -->
    <div v-if="briefing" class="grid grid-cols-1 md:grid-cols-2 gap-2.5 items-start">
      <div v-if="briefing.evenements.length" class="rounded-lg border border-white/15 bg-white/5 p-2.5">
        <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1">⏰ Événements armés du jour</p>
        <table class="w-full text-[10px]">
          <tbody>
            <tr v-for="(e, i) in briefing.evenements" :key="i" class="border-b border-white/5 last:border-0">
              <td class="py-0.5 pr-2 font-mono text-white whitespace-nowrap">{{ e.heure_paris }}</td>
              <td class="py-0.5 font-mono text-white/90 whitespace-nowrap">{{ e.asset }}</td>
              <td class="py-0.5 text-white/70">{{ e.evenement }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="flex flex-col gap-2.5">
        <div v-if="briefing.annonces.length" class="rounded-lg border border-white/15 bg-white/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1">📅 Annonces High — 24 h</p>
          <table class="w-full text-[10px]">
            <tbody>
              <tr v-for="(a, i) in briefing.annonces" :key="i" class="border-b border-white/5 last:border-0">
                <td class="py-0.5 pr-2 font-mono text-white whitespace-nowrap">{{ a.heure_paris }}</td>
                <td class="py-0.5 pr-2 font-mono text-amber-300">{{ a.devise }}</td>
                <td class="py-0.5 text-white/70">{{ a.titre }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="briefing.positions.length" class="rounded-lg border border-white/15 bg-white/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1">📍 Positions ouvertes</p>
          <div class="overflow-x-auto">
            <table class="w-full text-[10px]">
              <thead>
                <tr class="text-white/50 uppercase">
                  <th class="text-left pb-1 font-semibold">Strat.</th>
                  <th class="text-left pb-1 font-semibold">Asset</th>
                  <th class="text-right pb-1 font-semibold">Entrée</th>
                  <th class="text-right pb-1 font-semibold">Heure</th>
                  <th class="text-right pb-1 font-semibold">±R</th>
                  <th class="text-right pb-1 font-semibold">$</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(p, i) in briefing.positions" :key="i" class="border-t border-white/5">
                  <td class="py-0.5 pr-2 text-white/70 whitespace-nowrap">{{ p.strategie }}</td>
                  <td class="py-0.5 pr-2 font-mono text-white whitespace-nowrap">{{ p.asset }} <span :class="p.direction === 'Long' ? 'text-emerald-400' : p.direction === 'Short' ? 'text-red-400' : 'text-white/50'">{{ p.direction === 'Long' ? '▲' : p.direction === 'Short' ? '▼' : '⇅' }}</span></td>
                  <td class="py-0.5 pr-2 text-right font-mono text-white/90 whitespace-nowrap">{{ p.entree.toFixed(2) }}</td>
                  <td class="py-0.5 pr-2 text-right font-mono text-white/70 whitespace-nowrap">{{ heureParis(p.heure_entree) }}</td>
                  <td class="py-0.5 pr-2 text-right font-mono whitespace-nowrap" :class="rFlottant(p) >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ rFlottantTxt(p) }}</td>
                  <td class="py-0.5 text-right font-mono whitespace-nowrap" :class="(p.dollars ?? 0) >= 0 ? 'text-emerald-400' : 'text-red-400'">{{ p.dollars == null ? '—' : dollarsTxt(p.dollars) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div v-if="briefing.presse.length" class="rounded-lg border border-white/15 bg-white/5 p-2.5">
          <p class="text-[10px] font-bold uppercase tracking-wider text-white/70 mb-1">📰 Presse marquante</p>
          <ul class="text-[10px] text-white/85 space-y-1">
            <li v-for="(a, i) in briefing.presse" :key="i" class="flex gap-1.5">
              <span class="font-mono" :class="a.score >= 75 ? 'text-emerald-300' : 'text-white/50'">{{ a.score }}</span>
              <span class="leading-snug">{{ a.titre }}</span>
            </li>
          </ul>
        </div>
      </div>
    </div>
        <p class="text-[10px] text-white">Tableaux = calcul de l'app (aucune invention) ; la synthèse hiérarchise — l'IA propose, le propriétaire décide.</p>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { http } from '@/services/http.client'

interface PositionIa {
  strategie: string; asset: string; direction: string; entree: number; stop_loss: number
  heure_entree: number; r_actuel: number; r_court: number; dollars: number | null
}
interface BriefingIa {
  evenements: { heure_paris: string; asset: string; evenement: string }[]
  annonces: { heure_paris: string; devise: string; titre: string }[]
  positions: PositionIa[]
  presse: { titre: string; score: number; theme: string }[]
  synthese: { resume: string; ce_qui_compte: string[]; confiance: number } | null
}

const briefing = ref<BriefingIa | null>(null)
const enCours = ref(false)
const enCache = ref(false)
const erreur = ref('')

/// forcer=true (↻/⚡) : régénère en ignorant le cache du jour.
async function generer(forcer = false) {
  enCours.value = true
  erreur.value = ''
  try {
    const res = await http.post<{ en_cache: boolean; briefing: BriefingIa }>(
      '/api/briefing/ia', { forcer }, { timeout: 180_000 })
    briefing.value = res.data.briefing
    enCache.value = res.data.en_cache
  } catch {
    erreur.value = 'Analyste indisponible — vérifier que le serveur Ollama est démarré.'
  }
  enCours.value = false
}

function heureParis(epoch: number): string {
  if (!epoch) return '—'
  return new Date(epoch * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit', timeZone: 'Europe/Paris' })
}
function rFlottant(p: PositionIa): number {
  return p.direction === 'Both' ? Math.max(p.r_actuel, p.r_court) : p.r_actuel
}
function rFlottantTxt(p: PositionIa): string {
  if (p.direction === 'Both') return `L ${p.r_actuel >= 0 ? '+' : ''}${p.r_actuel.toFixed(1)} / S ${p.r_court >= 0 ? '+' : ''}${p.r_court.toFixed(1)}`
  return `${p.r_actuel >= 0 ? '+' : ''}${p.r_actuel.toFixed(1)}`
}
function dollarsTxt(v: number): string {
  return `${v >= 0 ? '+' : '−'}${Math.abs(v).toFixed(0)} $`
}

onMounted(() => generer(false))
</script>
