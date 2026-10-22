<template>
  <main>
    <p class="titre-section">📅 Semaine à venir</p>
    <div class="carte liste">
      <p v-if="groupes.length === 0" class="vide">Aucun événement High dans les 7 prochains jours</p>
      <template v-for="g in groupes" :key="g.jour">
        <p class="jour">{{ g.jour }}</p>
        <p v-for="(e, i) in g.evenements" :key="i" class="ligne statique">
          <span class="heure mono">{{ e.heure_paris.slice(6) }}</span>
          <span class="titre">{{ e.titre }}</span>
          <span class="devise">{{ e.devise }}</span>
        </p>
      </template>
    </div>
    <p class="note">Annonces à impact High uniquement — le reste vit dans le calendrier de l'app principale.</p>
  </main>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '../api'
import type { EvenementCal } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const evenements = ref<EvenementCal[]>(demo ? [
  { ts: Date.now() / 1000 + 3600, heure_paris: '22/10 14h30', devise: 'USD', titre: 'CPI m/m' },
  { ts: Date.now() / 1000 + 86400, heure_paris: '23/10 16h00', devise: 'USD', titre: 'Demandes hebdo chômage' },
] : [])

const groupes = computed(() => {
  const g: { jour: string; evenements: EvenementCal[] }[] = []
  for (const e of evenements.value) {
    const j = e.heure_paris.slice(0, 5)
    const dernier = g[g.length - 1]
    if (dernier && dernier.jour === j) dernier.evenements.push(e)
    else g.push({ jour: j, evenements: [e] })
  }
  return g
})

onMounted(async () => {
  if (demo) return
  try { evenements.value = (await api.calendrier()).evenements } catch { /* hors ligne */ }
})
</script>

<style scoped>
.liste { padding: 4px 6px; }
.jour { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--texte-40); padding: 10px 8px 4px; }
.ligne { display: flex; align-items: center; gap: 10px; width: 100%; padding: 9px 8px; border-bottom: 1px solid rgba(255,255,255,0.05); text-align: left; }
.ligne:last-of-type { border-bottom: none; }
.heure { color: var(--bleu); font-weight: 700; font-size: 12px; }
.titre { flex: 1; font-size: 13px; }
.devise { color: var(--texte-40); font-size: 11px; }
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
.note { font-size: 11px; color: var(--texte-40); margin: 8px 14px; }
</style>
