<template>
  <main>
    <p class="titre-section">🔭 Zones en approche</p>
    <div class="carte liste">
      <p v-if="enBande.length === 0" class="vide">Horizon clair — aucune zone armée dans la bande d'approche (0,5 ATR)</p>
      <p v-for="(z, i) in enBande" :key="i" class="ligne statique">
        <span class="ico" :class="z.achat ? 'vert' : 'rouge'">{{ z.achat ? '🟢' : '🔴' }}</span>
        <span class="corps">
          <span class="titre">{{ z.asset }} - {{ z.tf }} · {{ z.achat ? 'Achat' : 'Vente' }}</span>
          <span class="sous mono">{{ prix2(z.zone_bas) }} – {{ prix2(z.zone_haut) }} · à {{ z.distance_atr.toFixed(2) }} ATR ({{ z.distance_pct.toFixed(2) }} %)</span>
        </span>
      </p>
    </div>
    <p class="note">Le radar : zones armées que le prix approche — celles qui déclencheront les prochaines alertes SMC.</p>
  </main>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { api } from '../api'
import { prix2 } from '../format'
import type { ZoneBande } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const enBande = ref<ZoneBande[]>(demo ? [
  { asset: 'XAUUSD', tf: 'M15', achat: true, zone_haut: 4158.65, zone_bas: 4155.32, distance_atr: 0.38, distance_pct: 0.09, prix: 4151.2 },
] : [])

let minuteur: ReturnType<typeof setInterval> | null = null

async function charger() {
  if (demo) return
  try {
    enBande.value = (await api.zones()).en_bande
  } catch { /* hors ligne */ }
}

onMounted(() => {
  void charger()
  minuteur = setInterval(() => void charger(), 60_000)
})
onUnmounted(() => { if (minuteur) clearInterval(minuteur) })
</script>

<style scoped>
.liste { padding: 4px 6px; }
.ligne { display: flex; align-items: center; gap: 10px; width: 100%; padding: 10px 8px; border-bottom: 1px solid rgba(255,255,255,0.05); text-align: left; }
.ligne:last-of-type { border-bottom: none; }
.ico { font-size: 15px; }
.corps { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.titre { font-size: 14px; font-weight: 600; }
.sous { font-size: 11px; color: var(--texte-60); }
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
.note { font-size: 11px; color: var(--texte-40); margin: 8px 14px; }
</style>
