<template>
  <!-- Briefing du jour (owner 13/10 — KTA #11) : les TABLEAUX factuels sont
       calculés par l'app (zéro hallucination), le LLM ne produit que la
       hiérarchie « ce qui compte aujourd'hui ». Cache du jour côté backend. -->
  <div class="flex flex-col gap-3">
    <div class="flex items-center gap-2 flex-wrap">
      <button class="ml-auto text-[10px] px-2.5 py-1 rounded-lg font-semibold transition-colors disabled:opacity-40"
              :class="enCours ? 'bg-white/10 text-white' : 'bg-sky-500/20 text-sky-300 hover:bg-sky-500/30'"
              :disabled="enCours" @click="generer(true)">
        {{ enCours ? '⏳ Analyse… (~1 min)' : briefing?.synthese ? '↻ Régénérer' : '⚡ Générer le briefing' }}</button>
    </div>

    <!-- Synthèse LLM (refonte owner 23/10) : les six sections titrées ;
         repli resume/ce_qui_compte pour un cache d'ancien format. -->
    <div v-if="briefing?.synthese?.sections?.length" class="rounded-lg border border-sky-500/25 bg-sky-500/5 p-2.5">
      <template v-for="(s, i) in briefing.synthese.sections" :key="i">
        <hr v-if="i > 0" class="border-white/10 my-2" />
        <p class="text-[11px] font-bold uppercase tracking-wider text-sky-300 mb-1">{{ s.titre }}</p>
        <p v-for="(ligne, j) in s.texte.split('\n')" :key="j" class="text-[12px] text-white/90 leading-relaxed">{{ ligne }}</p>
      </template>
    </div>
    <div v-else-if="briefing?.synthese" class="rounded-lg border border-sky-500/25 bg-sky-500/5 p-2.5">
      <p class="text-[10px] font-bold uppercase tracking-wider text-sky-300 mb-1">🎯 Ce qui compte aujourd'hui</p>
      <ol class="text-[12px] text-white space-y-1 list-decimal list-inside leading-snug">
        <li v-for="(l, i) in briefing.synthese.ce_qui_compte" :key="i">{{ l }}</li>
      </ol>
      <p class="text-[12px] text-white/80 mt-2 leading-relaxed">{{ briefing.synthese.resume }}</p>
    </div>
    <p v-else-if="erreur" class="text-xs text-red-300">{{ erreur }}</p>

    <!-- Données factuelles (calcul app) — owner 23/10 : événements armés
         retirés de l'affichage (ils vivent dans la section ② du LLM),
         annonces High puis presse en 4 colonnes. -->
    <div v-if="briefing" class="flex flex-col gap-2.5">
        <div v-if="briefing.annonces.length" class="rounded-lg border border-sky-500/25 bg-sky-500/5 p-2.5">
          <p class="text-[11px] font-bold uppercase tracking-wider text-sky-300 mb-1">📅 Annonces High — 24 h</p>
          <table class="w-full text-[11px]">
            <tbody>
              <tr v-for="(a, i) in briefing.annonces" :key="i" class="border-b border-white/5 last:border-0">
                <td class="py-0.5 pr-2 font-mono text-white whitespace-nowrap">{{ a.heure_paris }}</td>
                <td class="py-0.5 pr-2 font-mono text-amber-300">{{ a.devise }}</td>
                <td class="py-0.5 text-white/70">{{ a.titre }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div v-if="briefing.presse.length" class="rounded-lg border border-sky-500/25 bg-sky-500/5 p-2.5">
          <p class="text-[11px] font-bold uppercase tracking-wider text-sky-300 mb-1.5">📰 Presse marquante</p>
          <div class="grid grid-cols-2 xl:grid-cols-4 gap-2.5">
            <div v-for="col in colonnesPresse" :key="col.nom" class="flex flex-col gap-1">
              <p class="text-[11px] font-extrabold uppercase tracking-wider text-sky-300/90">{{ col.emoji }} {{ col.nom }}</p>
              <button v-for="(a, i) in col.articles" :key="i" class="text-left text-[12px] leading-snug flex gap-1.5 hover:bg-white/5 rounded px-0.5 -mx-0.5 transition-colors" :disabled="!a.url" @click="ouvrirExterne(a.url)">
                <span class="font-mono shrink-0 text-[11px]" :class="a.score >= 75 ? 'text-emerald-300' : 'text-white/50'">{{ a.score }}</span>
                <span class="text-white/85 underline decoration-white/25 hover:decoration-sky-300 hover:text-sky-200">{{ a.titre }}</span>
              </button>
              <p v-if="!col.articles.length" class="text-[11px] text-white/40">—</p>
            </div>
          </div>
        </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { http } from '@/services/http.client'

interface BriefingIa {
  evenements: { heure_paris: string; asset: string; evenement: string }[]
  annonces: { heure_paris: string; devise: string; titre: string }[]
  presse: { titre: string; url: string; score: number; theme: string }[]
  synthese: { resume: string; ce_qui_compte: string[]; sections?: { titre: string; texte: string }[] } | null
}

const briefing = ref<BriefingIa | null>(null)

/// Ouvre une URL externe : commande Tauri `ouvrir_url` (même mécanisme que
/// la page Presse) ; `window.open` en repli dev navigateur.
async function ouvrirExterne(url: string) {
  if (!url) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('ouvrir_url', { url })
  } catch {
    window.open(url, '_blank', 'noopener')
  }
}

/// Presse en 4 colonnes (owner 23/10) — techno = thème base « actions US ».
const colonnesPresse = computed(() => [
  { nom: 'Macro', emoji: '🏛️', articles: (briefing.value?.presse ?? []).filter(a => a.theme === 'macro') },
  { nom: 'Crypto', emoji: '₿', articles: (briefing.value?.presse ?? []).filter(a => a.theme === 'crypto') },
  { nom: 'Métaux', emoji: '⛏️', articles: (briefing.value?.presse ?? []).filter(a => a.theme === 'metaux') },
  { nom: 'Techno', emoji: '💡', articles: (briefing.value?.presse ?? []).filter(a => a.theme === 'actions_us') },
])
const enCours = ref(false)
const erreur = ref('')

/// forcer=true (↻/⚡) : régénère en ignorant le cache du jour.
async function generer(forcer = false) {
  enCours.value = true
  erreur.value = ''
  try {
    const res = await http.post<{ en_cache: boolean; briefing: BriefingIa }>(
      '/api/briefing/ia', { forcer }, { timeout: 180_000 })
    briefing.value = res.data.briefing
  } catch {
    erreur.value = 'Analyste indisponible — vérifier que le serveur Ollama est démarré.'
  }
  enCours.value = false
}

onMounted(() => generer(false))
</script>
