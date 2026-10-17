<template>
  <div class="app">
    <!-- Bandeau état : connexion maison + muette -->
    <header class="bandeau">
      <span class="point" :class="connecte ? 'vert' : 'rouge'" />
      <span class="etat">{{ connecte ? 'connecté à la maison' : 'hors ligne' }}</span>
      <span v-if="muette" class="muette">🌙 muette</span>
      <button class="engrenage" @click="reglagesOuverts = true">⚙︎</button>
    </header>

    <AujourdHui v-if="onglet === 'aujourdhui'" @ouvrirAnnonce="ouvrirAnnonce" />
    <Journal v-else @ouvrir="ouvrirAlerte" />

    <!-- Vue détail en plein écran (au-dessus des onglets) -->
    <DetailAlerte
      v-if="detailId !== null || detailDirecte !== null"
      :id="detailId ?? 0"
      :directe="detailDirecte"
      @fermer="fermerDetail"
    />

    <!-- Réglages -->
    <Reglages v-if="reglagesOuverts" @fermer="reglagesOuverts = false" @teste="testerConnexion" />

    <!-- Onglets fixes -->
    <nav class="onglets">
      <button :class="{ actif: onglet === 'aujourdhui' }" @click="onglet = 'aujourdhui'">
        <span class="ico">☀️</span>Aujourd'hui
      </button>
      <button :class="{ actif: onglet === 'journal' }" @click="onglet = 'journal'">
        <span class="ico">📋</span>Journal
      </button>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { api, muetteActive } from './api'
import { demoJournal } from './fixtures'
import type { AlerteComplete, Annonce } from './types'
import AujourdHui from './vues/AujourdHui.vue'
import Journal from './vues/Journal.vue'
import DetailAlerte from './vues/DetailAlerte.vue'
import Reglages from './vues/Reglages.vue'

const demo = new URLSearchParams(location.search).has('demo')
const params = new URLSearchParams(location.search)
const ongletInitial = params.get('tab') === 'journal' ? 'journal' : 'aujourdhui'
const onglet = ref<'aujourdhui' | 'journal'>(ongletInitial)
const detailId = ref<number | null>(params.get('detail') ? Number(params.get('detail')) : null)
const detailDirecte = ref<AlerteComplete | null>(null)
const reglagesOuverts = ref(false)
const connecte = ref(demo) // démo = connecté par construction
const muette = ref(muetteActive())
let minuteur: ReturnType<typeof setInterval> | null = null

function ouvrirAlerte(id: number) {
  detailDirecte.value = null
  detailId.value = id
}

/// Événement High cliqué sur l'onglet Aujourd'hui : il n'y a pas (encore)
/// de ligne d'alerte en base — le détail est bâti sur l'annonce elle-même.
function ouvrirAnnonce(a: Annonce) {
  detailId.value = null
  detailDirecte.value = {
    id: -a.ts,
    ts: a.ts,
    type: 'evenement',
    titre: `${a.titre} · ${a.heure_paris}`,
    asset: null,
    tf: null,
    png: false,
    payload: { heure_paris: a.heure_paris, devise: a.devise, titre: a.titre, rappel_min: 10 },
  }
}

function fermerDetail() {
  detailId.value = null
  detailDirecte.value = null
}

async function testerConnexion() {
  if (demo) {
    connecte.value = true
    return
  }
  try {
    await api.etat()
    connecte.value = true
  } catch {
    connecte.value = false
  }
}

onMounted(() => {
  void testerConnexion()
  minuteur = setInterval(() => {
    muette.value = muetteActive()
    void testerConnexion()
  }, 30_000)
})
onUnmounted(() => {
  if (minuteur) clearInterval(minuteur)
})

void demoJournal // référencé pour le mode démo côté vues
</script>

<style scoped>
.bandeau {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  background: rgba(10, 14, 39, 0.92);
  backdrop-filter: blur(8px);
  border-bottom: 1px solid var(--bordure);
}
.point { width: 9px; height: 9px; border-radius: 50%; }
.point.vert { background: var(--vert); box-shadow: 0 0 6px var(--vert); }
.point.rouge { background: var(--rouge); }
.etat { font-size: 12px; color: var(--texte-60); }
.muette {
  font-size: 11px;
  color: var(--ambre);
  border: 1px solid rgba(245, 158, 11, 0.4);
  border-radius: 999px;
  padding: 1px 8px;
}
.engrenage {
  margin-left: auto;
  background: none;
  border: none;
  color: var(--texte-60);
  font-size: 20px;
  cursor: pointer;
}
.onglets {
  position: fixed;
  bottom: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 100%;
  max-width: 480px;
  display: flex;
  background: rgba(10, 14, 39, 0.96);
  border-top: 1px solid var(--bordure);
  padding-bottom: env(safe-area-inset-bottom);
}
.onglets button {
  flex: 1;
  padding: 12px 0 10px;
  background: none;
  border: none;
  color: var(--texte-40);
  font-size: 11px;
  font-weight: 600;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}
.onglets button.actif { color: var(--texte); }
.ico { font-size: 18px; }
</style>
