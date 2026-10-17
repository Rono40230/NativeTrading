<template>
  <main>
    <p class="titre-section">📋 Journal des alertes</p>
    <div class="carte liste">
      <p v-if="groupes.length === 0" class="vide">Aucune alerte pour l'instant — le relais vient de naître</p>
      <template v-for="g in groupes" :key="g.jour">
        <p class="jour">{{ g.jour }}</p>
        <button v-for="a in g.alertes" :key="a.id" class="ligne" @click="$emit('ouvrir', a.id)">
          <span class="ico">{{ icone(a.type) }}</span>
          <span class="corps">
            <span class="titre">{{ a.titre }}</span>
            <span v-if="a.asset" class="meta">{{ a.asset }}<template v-if="a.tf"> · {{ a.tf }}</template></span>
          </span>
          <span class="heure mono">{{ heure(a.ts) }}</span>
        </button>
      </template>
      <button v-if="encore" class="plus" @click="chargerSuite">Charger plus</button>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '../api'
import { demoJournal } from '../fixtures'
import type { Alerte, TypeAlerte } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const alertes = ref<Alerte[]>(demo ? demoJournal : [])
const encore = ref(false)

defineEmits<{ (e: 'ouvrir', id: number): void }>()

const icones: Record<TypeAlerte, string> = {
  digest: '🌅',
  evenement: '📅',
  presse: '📰',
  smc_approche: '📍',
  smc_contact: '🎯',
  prix: '🔔',
  sante: '🩸',
}

function icone(t: string): string {
  return icones[t as TypeAlerte] ?? '•'
}

function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

function jour(ts: number): string {
  const d = new Date(ts * 1000)
  const auj = new Date()
  const meme = d.toDateString() === auj.toDateString()
  const hier = new Date(auj.getTime() - 86_400_000)
  if (meme) return "Aujourd'hui"
  if (d.toDateString() === hier.toDateString()) return 'Hier'
  return d.toLocaleDateString('fr-FR', { day: 'numeric', month: 'long' })
}

const groupes = computed(() => {
  const g: { jour: string; alertes: Alerte[] }[] = []
  for (const a of alertes.value) {
    const j = jour(a.ts)
    const dernier = g[g.length - 1]
    if (dernier && dernier.jour === j) dernier.alertes.push(a)
    else g.push({ jour: j, alertes: [a] })
  }
  return g
})

async function chargerSuite() {
  if (demo) return
  const dernier = alertes.value[alertes.value.length - 1]
  try {
    const rep = await api.journal(dernier?.id)
    alertes.value.push(...rep.alertes)
    encore.value = rep.alertes.length === 60
  } catch {
    encore.value = false
  }
}

onMounted(async () => {
  if (demo) return
  try {
    const rep = await api.journal()
    alertes.value = rep.alertes
    encore.value = rep.alertes.length === 60
  } catch {
    // hors ligne : bandeau rouge de App.vue
  }
})
</script>

<style scoped>
.liste { padding: 4px 6px; }
.jour {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--texte-40);
  padding: 10px 8px 4px;
}
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
.ico { font-size: 17px; }
.corps { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.titre { font-size: 13px; line-height: 1.3; }
.meta { font-size: 11px; color: var(--texte-40); }
.heure { font-size: 11px; color: var(--texte-40); }
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
.plus {
  width: 100%;
  padding: 12px;
  background: none;
  border: none;
  color: var(--bleu);
  font-size: 12px;
  font-weight: 600;
}
</style>
