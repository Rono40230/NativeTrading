<template>
  <main>
    <p class="titre-section">🎚️ Niveaux armés</p>
    <div class="carte liste">
      <p v-if="niveaux.length === 0" class="vide">Aucun niveau armé dans l'app principale</p>
      <p v-for="n in niveaux" :key="`${n.asset}-${n.prix}`" class="ligne statique">
        <span class="fleche" :class="n.sens === 'au_dessus' ? 'vert' : 'rouge'">
          {{ n.sens === 'au_dessus' ? '↑' : '↓' }}
        </span>
        <span class="titre">{{ n.asset }}</span>
        <span class="niveau mono">{{ prix2(n.prix) }}</span>
      </p>
    </div>

    <p class="titre-section">🔔 Déclenchées (24 h)</p>
    <div class="carte liste">
      <p v-if="groupes.length === 0" class="vide">
        Aucune alerte prix déclenchée — les niveaux armés apparaissent ici au toucher
      </p>
      <template v-for="g in groupes" :key="g.jour">
        <p class="jour">{{ g.jour }}</p>
        <button v-for="a in g.alertes" :key="a.id" class="ligne" @click="$emit('ouvrir', a.id)">
          <span class="ico">🔔</span>
          <span class="corps">
            <span class="titre">{{ a.titre }}<b v-if="a.niveau" class="mono"> · seuil {{ prix2(a.niveau) }}</b></span>
          </span>
          <span class="heure mono">{{ heure(a.ts) }}</span>
        </button>
      </template>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '../api'
import { prix2 } from '../format'
import type { Alerte, NiveauPrix } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const niveaux = ref<NiveauPrix[]>(
  demo ? [{ asset: 'XAUUSD', prix: 4183.54, sens: 'au_dessus', cree_le: Date.now() / 1000 - 3600 }] : [],
)
const alertes = ref<Alerte[]>([])

defineEmits<{ (e: 'ouvrir', id: number): void }>()

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

onMounted(async () => {
  if (demo) return
  try {
    // Nouvel endpoint (niveaux armés + déclenchées 24 h).
    const r = await api.prix()
    niveaux.value = r.niveaux
    alertes.value = r.declenchees
  } catch {
    // Ancien binaire (endpoint absent) : niveaux via l'endpoint de
    // l'app principale, déclenchées via le journal filtré.
    try {
      const armees = await api.alertesPrixPrincipales()
      niveaux.value = armees
        .filter((a) => a.active)
        .map((a) => ({ asset: a.asset, prix: a.prix, sens: a.sens, cree_le: a.cree_le }))
    } catch {
      // niveaux indisponibles tant que run.sh n'est pas relancé
    }
    try {
      const rep = await api.journal(undefined, 'prix')
      alertes.value = rep.alertes.filter((a) => a.type === 'prix')
    } catch {
      // hors ligne : bandeau rouge de App.vue
    }
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
.ligne.statique { cursor: default; }
.fleche { font-size: 16px; font-weight: 700; }
.fleche.vert { color: var(--vert); }
.fleche.rouge { color: var(--rouge); }
.ico { font-size: 17px; }
.corps { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.titre { font-size: 13px; font-weight: 600; }
.niveau { margin-left: auto; font-size: 13px; color: var(--texte-60); }
.heure { font-size: 11px; color: var(--texte-40); }
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
</style>
