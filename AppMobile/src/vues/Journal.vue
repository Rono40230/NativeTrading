<template>
  <main>
    <p class="titre-section">🎯 Alertes SMC</p>
    <p class="sous-titre">Dernière heure écoulée</p>
    <div class="carte liste">
      <p v-if="alertes.length === 0" class="vide">Aucune alerte SMC sur la dernière heure</p>
      <button v-for="a in alertes" :key="a.id" class="ligne" @click="$emit('ouvrir', a.id)">
        <span class="ico">{{ a.type === 'smc_contact' ? '🎯' : '📍' }}</span>
        <span class="corps">
          <span class="titre">{{ a.asset ?? '—' }} - {{ a.tf ?? '—' }}</span>
          <span class="sous" :class="a.achat ? 'vert' : 'rouge'">
            {{ a.type === 'smc_contact' ? 'Contact' : 'Approche' }}<template v-if="a.achat !== undefined"> · {{ a.achat ? 'Achat' : 'Vente' }}</template>
          </span>
        </span>
        <span class="heure mono">{{ heure(a.ts) }}</span>
      </button>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '../api'
import { demoJournal } from '../fixtures'
import type { Alerte } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const alertes = ref<Alerte[]>([])

defineEmits<{ (e: 'ouvrir', id: number): void }>()

function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

/// Page de veille SMC (owner 20/10) : approches + contacts de la DERNIÈRE
/// HEURE seulement, au format « asset - TF ». Le clic ouvre le détail :
/// niveaux du bloc (zone, entrée, SL) + le graphique capturé à l'alerte.
/// Garde : si le serveur ignore encore type/fenetre, on filtre ici.
function garder(a: Alerte): boolean {
  if (a.type !== 'smc_approche' && a.type !== 'smc_contact') return false
  return a.ts >= Date.now() / 1000 - 3600
}

onMounted(async () => {
  if (demo) {
    alertes.value = demoJournal.filter(garder)
    return
  }
  try {
    const rep = await api.journal(undefined, 'smc_approche,smc_contact', 60)
    alertes.value = rep.alertes.filter(garder)
  } catch {
    // hors ligne : bandeau rouge de App.vue
  }
})
</script>

<style scoped>
.sous-titre {
  font-size: 11px;
  color: var(--texte-40);
  margin: -4px 14px 8px;
}
.liste { padding: 4px 6px; }
.ligne {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 8px;
  background: none;
  border: none;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  color: var(--texte);
  text-align: left;
}
.ligne:last-of-type { border-bottom: none; }
.ico { font-size: 17px; }
.corps { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.titre { font-size: 14px; font-weight: 600; }
.sous { font-size: 11px; font-weight: 600; letter-spacing: 0.03em; }
.sous.vert { color: var(--vert); }
.sous.rouge { color: var(--rouge); }
.heure { font-size: 11px; color: var(--texte-40); }
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
</style>
