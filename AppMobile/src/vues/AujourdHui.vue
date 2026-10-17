<template>
  <main>
    <p class="titre-section">📅 Événements High du jour</p>
    <div class="carte liste">
      <p v-if="donnees.annonces.length === 0" class="vide">Aucun événement High aujourd'hui</p>
      <button
        v-for="a in donnees.annonces"
        :key="a.ts"
        class="ligne"
        @click="$emit('ouvrirAnnonce', a)"
      >
        <span class="heure mono">{{ a.heure_paris }}</span>
        <span class="titre">{{ a.titre }}</span>
        <span class="devise">{{ a.devise }}</span>
      </button>
    </div>

    <p class="titre-section">🌙 Pendant la nuit</p>
    <div class="carte liste">
      <p v-if="donnees.nuit.length === 0" class="vide">Nuit calme — rien à signaler</p>
      <p v-for="n in donnees.nuit" :key="n.ts" class="ligne statique">
        <span class="heure mono">{{ heure(n.ts) }}</span>
        <span class="titre">{{ n.titre }}</span>
      </p>
    </div>

    <p class="titre-section">📰 Presse marquante</p>
    <div class="carte liste">
      <p v-if="donnees.presse.length === 0" class="vide">Aucun gros titre à 100 aujourd'hui</p>
      <p v-for="(p, i) in donnees.presse" :key="i" class="ligne statique">
        <span class="score mono">{{ p.score }}</span>
        <span class="titre">{{ p.titre }}</span>
      </p>
      <div class="contexte">
        <label>🌍 Contexte marché (IA)</label>
        <p v-if="!donnees.synthese" class="vide">Synthèse pas encore générée — le briefing de 8h30 s'en charge</p>
        <template v-else>
          <p class="resume">{{ donnees.synthese.resume }}</p>
          <ul class="compte">
            <li v-for="c in donnees.synthese.ce_qui_compte" :key="c">{{ c }}</li>
          </ul>
        </template>
      </div>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '../api'
import { demoAujourdhui } from '../fixtures'
import type { Annonce, Aujourdhui } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const donnees = ref<Aujourdhui>(demo ? demoAujourdhui : { date_paris: '', annonces: [], presse: [], nuit: [], synthese: null })

defineEmits<{ (e: 'ouvrirAnnonce', a: Annonce): void }>()

function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

onMounted(async () => {
  if (demo) return
  try {
    donnees.value = await api.aujourdhui()
  } catch {
    // hors ligne : l'état reste celui du dernier chargement
  }
})
</script>

<style scoped>
.liste { padding: 4px 6px; }
.ligne {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 8px;
  background: none;
  border: none;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  color: var(--texte);
  text-align: left;
  font-size: 13px;
}
.ligne:last-of-type { border-bottom: none; }
.ligne.statique { cursor: default; }
.heure { color: var(--bleu); font-weight: 700; font-size: 13px; min-width: 42px; }
.devise { color: var(--texte-40); font-size: 11px; }
.titre { flex: 1; }
.score { color: var(--ambre); font-weight: 700; min-width: 30px; }
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
.contexte {
  margin: 6px 4px 6px;
  padding: 10px;
  border: 1px solid var(--bordure);
  border-radius: 10px;
  background: var(--carte);
}
.contexte label { display: block; font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--texte-40); margin-bottom: 6px; }
.resume { font-size: 13px; line-height: 1.5; }
.compte { margin: 10px 0 0 16px; font-size: 12px; color: var(--texte-60); line-height: 1.6; }
</style>
