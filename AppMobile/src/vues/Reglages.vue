<template>
  <div class="voile" @click.self="$emit('fermer')">
    <div class="feuille">
      <header>
        <span class="titre">⚙︎ Réglages</span>
        <button class="fermer" @click="$emit('fermer')">✕</button>
      </header>
      <div class="corps">
        <label class="champ">
          Serveur (URL Tailscale ou vide en dev)
          <input v-model="serveur" placeholder="http://100.x.x.x:8080" spellcheck="false" />
        </label>
        <label class="champ">
          Token d'accès (si posé côté serveur)
          <input v-model="token" type="password" placeholder="vide = ouvert" />
        </label>
        <button class="tester" @click="tester">Tester la connexion</button>
        <p v-if="resultat" :class="resultat.ok ? 'ok' : 'ko'">
          {{ resultat.ok ? '✓ Connecté — le relais répond' : `✗ ${resultat.erreur}` }}
        </p>

        <template v-if="android">
          <div class="separateur" />
          <button class="tester relais" :class="{ actif: relaisActif }" @click="basculerRelais">
            {{ relaisActif ? '⏹ Arrêter le relais en arrière-plan' : '▶ Activer le relais en arrière-plan' }}
          </button>
          <p v-if="messageRelais" class="note-relais">{{ messageRelais }}</p>
          <p class="infos">
            Service au premier plan : vérifie le serveur toutes les 10 min et notifie
            hors muette 21h→8h. Survit à la fermeture de l'app et au redémarrage du téléphone.
          </p>
        </template>

        <div class="separateur" />
        <div class="infos">
          <p>🌙 Muette : 21h → 8h (Paris) — fixe, par conception.</p>
          <p>L'app ne commande rien : la tour de contrôle reste l'app principale.</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api, reglages } from '../api'

defineEmits<{ (e: 'fermer'): void; (e: 'teste'): void }>()

const serveur = ref(reglages.serveur)
const token = ref(reglages.token)
const resultat = ref<{ ok: boolean; erreur?: string } | null>(null)

// Pont natif (MOB ③) : uniquement dans l'APK — le dev navigateur n'a pas
// window.__TAURI_INTERNALS__, la section reste cachée.
const android = '__TAURI_INTERNALS__' in window
const relaisActif = ref(false)
const messageRelais = ref('')

async function basculerRelais() {
  messageRelais.value = ''
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    if (relaisActif.value) {
      await invoke('plugin:relais|arreter')
    } else {
      await invoke('plugin:relais|demarrer', {
        serveur: serveur.value.replace(/\/+$/, ''),
        token: token.value,
      })
    }
    relaisActif.value = !relaisActif.value
    messageRelais.value = relaisActif.value
      ? 'Relais actif — notification permanente posée.'
      : 'Relais arrêté.'
  } catch (e) {
    messageRelais.value = e instanceof Error ? e.message : 'Pont natif indisponible'
  }
}

onMounted(async () => {
  if (!android) return
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const etat = await invoke<{ actif: boolean }>('plugin:relais|etat')
    relaisActif.value = etat.actif
  } catch {
    // état illisible : la bascule renseignera
  }
})

async function tester() {
  reglages.serveur = serveur.value
  reglages.token = token.value
  try {
    await api.etat()
    resultat.value = { ok: true }
  } catch (e) {
    resultat.value = { ok: false, erreur: e instanceof Error ? e.message : 'inaccessible' }
  }
}
</script>

<style scoped>
.voile {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: flex-end;
}
.feuille {
  width: 100%;
  max-width: 480px;
  background: #10142e;
  border-radius: 20px 20px 0 0;
  border: 1px solid var(--bordure);
  padding-bottom: calc(14px + env(safe-area-inset-bottom));
}
header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px;
  border-bottom: 1px solid var(--bordure);
}
.titre { font-size: 15px; font-weight: 700; }
.fermer { background: none; border: none; color: var(--texte-60); font-size: 16px; }
.corps { padding: 14px; display: flex; flex-direction: column; gap: 12px; }
.champ { display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--texte-60); }
.champ input {
  background: var(--carte);
  border: 1px solid var(--bordure);
  border-radius: 10px;
  padding: 10px 12px;
  color: var(--texte);
  font-size: 14px;
}
.tester {
  background: rgba(59, 130, 246, 0.2);
  color: #93c5fd;
  border: 1px solid rgba(59, 130, 246, 0.4);
  border-radius: 10px;
  padding: 10px;
  font-size: 13px;
  font-weight: 600;
}
.tester.relais.actif {
  background: rgba(239, 68, 68, 0.16);
  color: #fca5a5;
  border-color: rgba(239, 68, 68, 0.45);
}
.note-relais { font-size: 12px; color: var(--texte-60); }
.separateur { border-top: 1px solid var(--bordure); margin: 4px 0; }
.ok { color: var(--vert); font-size: 13px; }
.ko { color: var(--rouge); font-size: 13px; }
.infos { margin-top: 6px; font-size: 11px; color: var(--texte-40); line-height: 1.6; }
</style>
