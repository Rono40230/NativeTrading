<template>
  <div class="app">
    <!-- Bandeau état : connexion maison + muette -->
    <header class="bandeau">
      <span class="point" :class="connecte ? 'vert' : 'rouge'" />
      <span class="etat">{{ connecte ? 'connecté à la maison' : 'hors ligne' }}</span>
      <span v-if="muette" class="muette">🌙 muette</span>
      <button class="engrenage" @click="reglagesOuverts = true">⚙︎</button>
    </header>

    <AujourdHui
      v-if="onglet === 'aujourdhui'"
      @ouvrirAnnonce="ouvrirAnnonce"
      @ouvrirAlerte="ouvrirAlerte"
    />
    <Presse v-else-if="onglet === 'presse'" />
    <Journal v-else-if="onglet === 'journal'" @ouvrir="ouvrirAlerte" />
    <Prix v-else-if="onglet === 'prix'" @ouvrir="ouvrirAlerte" />
    <Zones v-else-if="onglet === 'zones'" />
    <Calendrier v-else />

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
      <button :class="{ actif: onglet === 'presse' }" @click="onglet = 'presse'">
        <span class="ico">📰</span>Presse
      </button>
      <button :class="{ actif: onglet === 'journal' }" @click="onglet = 'journal'">
        <span class="ico">🎯<span v-if="nonLus.smc" class="pastille">{{ nonLus.smc > 9 ? '9+' : nonLus.smc }}</span></span>Alertes SMC
      </button>
      <button :class="{ actif: onglet === 'prix' }" @click="onglet = 'prix'">
        <span class="ico">🔔<span v-if="nonLus.prix" class="pastille">{{ nonLus.prix > 9 ? '9+' : nonLus.prix }}</span></span>Prix
      </button>
      <button :class="{ actif: onglet === 'zones' }" @click="onglet = 'zones'">
        <span class="ico">🔭</span>Zones
      </button>
      <button :class="{ actif: onglet === 'calendrier' }" @click="onglet = 'calendrier'">
        <span class="ico">📅</span>Semaine
      </button>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { api, muetteActive } from './api'
import { nonLus, rafraichirBadges, marquerVus } from './badges'
import { demoJournal } from './fixtures'
import type { AlerteComplete, Annonce } from './types'
import AujourdHui from './vues/AujourdHui.vue'
import Presse from './vues/Presse.vue'
import Journal from './vues/Journal.vue'
import Prix from './vues/Prix.vue'
import Zones from './vues/Zones.vue'
import Calendrier from './vues/Calendrier.vue'
import DetailAlerte from './vues/DetailAlerte.vue'
import Reglages from './vues/Reglages.vue'

const demo = new URLSearchParams(location.search).has('demo')
const params = new URLSearchParams(location.search)
const ongletInitial =
  params.get('tab') === 'journal' ? 'journal'
  : params.get('tab') === 'presse' ? 'presse'
  : params.get('tab') === 'prix' ? 'prix'
  : params.get('tab') === 'zones' ? 'zones'
  : params.get('tab') === 'calendrier' ? 'calendrier'
  : 'aujourdhui'
const onglet = ref<'aujourdhui' | 'presse' | 'journal' | 'prix' | 'zones' | 'calendrier'>(ongletInitial)
const detailId = ref<number | null>(params.get('detail') ? Number(params.get('detail')) : null)
const detailDirecte = ref<AlerteComplete | null>(null)
const reglagesOuverts = ref(false)
const connecte = ref(demo) // démo = connecté par construction
const muette = ref(muetteActive())
let minuteur: ReturnType<typeof setInterval> | null = null
let minuteurBadges: ReturnType<typeof setInterval> | null = null

/// Ouvrir un onglet = consulter ses alertes : badge remis à zéro.
watch(
  onglet,
  (o) => {
    if (o === 'journal') marquerVus('smc')
    if (o === 'prix') marquerVus('prix')
  },
  { immediate: true },
)

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
  void rafraichirBadges()
  demarrerRelaisSiVoulu()
  void verifierNotifOuverte()
  minuteur = setInterval(() => {
    muette.value = muetteActive()
    void testerConnexion()
  }, 30_000)
  minuteurBadges = setInterval(() => void rafraichirBadges(), 60_000)
  // Deep-link notif → détail : au retour au premier plan (notif tapée app
  // déjà ouverte), l'id posé par l'intent est consommé et le détail s'ouvre.
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) void verifierNotifOuverte()
  })
})
onUnmounted(() => {
  if (minuteur) clearInterval(minuteur)
  if (minuteurBadges) clearInterval(minuteurBadges)
})

/// Notification qui a ouvert/ramené l'app → DetailAlerte. Consommé en
/// un appel (l'id ne ré-ouvre pas le détail deux fois) ; hors Android le
/// pont natif échoue silencieusement.
async function verifierNotifOuverte() {
  if (demo || !('__TAURI_INTERNALS__' in window)) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const r = await invoke<{ id: number }>('plugin:relais|derniereNotif')
    if (r && r.id > 0) ouvrirAlerte(r.id)
  } catch {
    // pont natif indisponible — rien à faire
  }
}

/// Service Android en avant plan (owner 22/10 : notification garantie,
/// même app fermée) : démarré dès l'ouverture sauf arrêt volontaire via
/// les Réglages (drapeau nt_relais_stop). Idempotent côté natif ; sur
/// navigateur de dev l'appel échoue silencieusement.
async function demarrerRelaisSiVoulu() {
  if (demo || !('__TAURI_INTERNALS__' in window)) return
  if (localStorage.getItem('nt_relais_stop')) return
  const serveur = localStorage.getItem('nt_serveur')
  if (!serveur) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('plugin:relais|demarrer', {
      serveur: serveur.replace(/\/+$/, ''),
      token: localStorage.getItem('nt_token') ?? '',
    })
  } catch {
    // pont natif indisponible ou service déjà lancé — rien à faire
  }
}

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
.ico { font-size: 18px; position: relative; }
.pastille {
  position: absolute;
  top: -6px;
  right: -12px;
  min-width: 16px;
  height: 16px;
  padding: 0 3px;
  border-radius: 999px;
  background: var(--rouge);
  color: #fff;
  font-size: 9px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
