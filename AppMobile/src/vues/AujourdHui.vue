<template>
  <main>
    <div v-if="marche.length" class="bandeau-marche">
      <div v-for="m in marche" :key="m.asset" class="ticker">
        <span class="asset">{{ m.asset }}</span>
        <span class="mono px">{{ prix2(m.prix) }}</span>
        <span class="mono var" :class="m.variation >= 0 ? 'v' : 'r'">{{ m.variation >= 0 ? '+' : '' }}{{ m.variation.toFixed(2) }} %</span>
      </div>
    </div>
    <button class="titre-section pliable" @click="plieAnnonces = !plieAnnonces">
      <span>📅 Événements High du jour<template v-if="plieAnnonces && donnees.annonces.length"> · {{ donnees.annonces.length }}</template></span>
      <span class="chevron" :class="{ ferme: plieAnnonces }" aria-hidden="true" />
    </button>
    <div v-if="!plieAnnonces" class="carte liste">
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

    <button class="titre-section pliable" @click="plieRapport = !plieRapport">
      <span>📊 Rapport d'activité</span>
      <span class="chevron" :class="{ ferme: plieRapport }" aria-hidden="true" />
    </button>
    <div v-if="!plieRapport" class="carte liste">
      <p v-if="rapports.length === 0" class="vide">Rapport indisponible hors ligne</p>
      <template v-else>
        <p class="sous-titre">Activité des dernières 24h</p>
        <table class="tbl">
          <thead><tr><th>Stratégie</th><th class="droite">Émis</th><th class="droite">Clôt.</th><th class="droite">±R</th></tr></thead>
          <tbody>
            <tr v-for="r in rapports" :key="r.id">
              <td class="strat">{{ r.icone }} {{ r.nom }}</td>
              <td class="droite">{{ r.emis }}</td>
              <td class="droite">{{ r.clotures }}</td>
              <td class="mono droite" :class="r.clotures === 0 ? '' : r.r >= 0 ? 'vert' : 'rouge'">
                {{ r.clotures === 0 ? '—' : (r.r >= 0 ? '+' : '−') + Math.abs(r.r).toFixed(1).replace('.', ',') }}
              </td>
            </tr>
          </tbody>
        </table>
      </template>
    </div>

    <button class="titre-section pliable" @click="plieEncours = !plieEncours">
      <span>📈 Signaux en cours<template v-if="plieEncours && enCours.length"> · {{ enCours.length }}</template></span>
      <span class="chevron" :class="{ ferme: plieEncours }" aria-hidden="true" />
    </button>
    <div v-if="!plieEncours" class="carte liste">
      <p v-if="enCours.length === 0" class="vide">Aucun signal en cours</p>
      <table class="tbl">
        <thead><tr><th>Stratégie</th><th>Asset</th><th>Sens</th><th>R</th><th>P/L latent</th></tr></thead>
        <tbody>
          <tr v-for="(c, i) in enCours" :key="i">
            <td class="strat">{{ nomStrategie(c.strategie) }}</td><td>{{ c.asset }}</td>
            <td :class="c.sens === 'Achat' ? 'vert' : 'rouge'">{{ c.sens === 'Achat' ? 'Buy' : c.sens === 'Vente' ? 'Sell' : 'Both' }}</td>
            <td class="mono pl" :class="c.r >= 0 ? 'vert' : 'rouge'">
              {{ (c.r >= 0 ? '+' : '') + c.r.toFixed(2) }}
            </td>
            <td class="mono pl" :class="(c.dollars ?? 0) >= 0 ? 'vert' : 'rouge'">
              {{ c.dollars !== null ? (c.dollars >= 0 ? '+' : '') + c.dollars.toFixed(2) + ' $' : '—' }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </main>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { api } from '../api'
import { prix2 } from '../format'
import { demoAujourdhui } from '../fixtures'
import type { ActifMarche, Annonce, Aujourdhui, RapportStrategie, SignalEnCours } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const donnees = ref<Aujourdhui>(demo ? demoAujourdhui : { date_paris: '', annonces: [], nuit: [] })
const marche = ref<ActifMarche[]>([])
const plieAnnonces = ref(false)
const plieRapport = ref(false)
const plieEncours = ref(false)
const rapports = ref<RapportStrategie[]>(demo ? [
  { id: 'SMC', nom: 'SMC', icone: '📐', emis: 13, clotures: 16, r: 4.4 },
  { id: 'straddle', nom: 'Straddle', icone: '⚡', emis: 5, clotures: 6, r: 0 },
  { id: 'rockets', nom: 'Rockets', icone: '🚀', emis: 2, clotures: 0, r: 0 },
  { id: 'kdj_halftrend', nom: 'KDJ', icone: '📈', emis: 5, clotures: 2, r: -2 },
] : [])
const enCours = ref<SignalEnCours[]>(demo ? [
  { strategie: 'SMC', asset: 'XAUUSD', sens: 'Achat', dollars: 8.74, r: 1.05, heure: 0 },
  { strategie: 'straddle', asset: 'BTC', sens: 'Double', dollars: 2.1, r: 0.2, heure: 0 },
  { strategie: 'rockets', asset: 'MTCH', sens: 'Achat', dollars: -1.4, r: -0.31, heure: 0 },
] : [])

defineEmits<{ (e: 'ouvrirAnnonce', a: Annonce): void }>()

/// Nom d'affichage (le rapport d'activité utilise déjà ces libellés).
function nomStrategie(s: string): string {
  return s === 'kdj_halftrend' ? 'KDJ' : s.charAt(0).toUpperCase() + s.slice(1)
}

async function toutCharger() {
  try { donnees.value = await api.aujourdhui() } catch { /* hors ligne */ }
  try {
    const r = await api.activite()
    rapports.value = r.rapports
    enCours.value = r.en_cours
  } catch { /* hors ligne */ }
  try { marche.value = (await api.marche()).actifs } catch { /* hors ligne */ }
}

let minuteur: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  if (demo) return
  void toutCharger()
  minuteur = setInterval(() => void toutCharger(), 60_000)
})
onUnmounted(() => { if (minuteur) clearInterval(minuteur) })
</script>

<style scoped>
.liste { padding: 4px 6px; }
.pliable {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: none;
  border: none;
  cursor: pointer;
  width: 100%;
  text-align: left;
}
/* Chevron 100 % CSS (bordures) — aucun glyphe requis : rendu garanti
   sur toutes les polices/plateformes (▾ manquait dans le headless). */
.chevron {
  width: 9px;
  height: 9px;
  border-right: 2px solid var(--texte-60);
  border-bottom: 2px solid var(--texte-60);
  transform: rotate(45deg);
  transition: transform 0.15s ease;
}
.chevron.ferme { transform: rotate(-45deg); }
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
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
.bandeau-marche { display: flex; gap: 6px; overflow-x: auto; padding: 2px 0 6px; }
.ticker { display: flex; flex-direction: column; gap: 1px; padding: 6px 10px; border: 1px solid var(--bordure); border-radius: 10px; background: var(--carte); min-width: 92px; }
.asset { font-size: 10px; font-weight: 700; color: var(--texte-40); }
.px { font-size: 13px; font-weight: 700; }
.var { font-size: 10px; }
.var.v { color: var(--vert); }
.var.r { color: var(--rouge); }
.sous-titre { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--texte-40); text-align: center; padding: 8px 0 2px; }
.droite { text-align: right; }
.bilan { display: flex; gap: 8px; align-items: baseline; font-size: 12px; white-space: nowrap; }
.blanc { color: var(--texte); }
.tbl { width: 100%; border-collapse: collapse; font-size: 12px; }
.tbl th { text-align: left; font-size: 10px; text-transform: uppercase; color: var(--texte-40); padding: 6px 6px 4px; border-bottom: 1px solid rgba(255,255,255,0.1); }
.tbl th.droite { text-align: right; }
.tbl td { padding: 6px; border-bottom: 1px solid rgba(255,255,255,0.05); }
.strat { font-weight: 700; }
.tbl td.vert { color: var(--vert); font-weight: 700; }
.tbl td.rouge { color: var(--rouge); font-weight: 700; }
.nombres { font-size: 12px; color: var(--texte-60); }
.pl { font-size: 13px; font-weight: 700; }
.pl.vert { color: var(--vert); }
.pl.rouge { color: var(--rouge); }
.brief { padding: 10px 12px; }
.resume { font-size: 13px; line-height: 1.55; }
.compte { margin: 10px 0 0 16px; font-size: 12px; color: var(--texte-60); line-height: 1.6; }
.detail {
  margin-top: 10px;
  width: 100%;
  padding: 8px;
  border: 1px solid var(--bordure);
  border-radius: 10px;
  background: none;
  color: var(--bleu);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
</style>
