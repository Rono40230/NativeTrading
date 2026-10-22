<template>
  <div class="voile" @click.self="$emit('fermer')">
    <div class="feuille">
      <header>
        <span class="ico">{{ icone }}</span>
        <span class="titre">{{ alerte?.titre ?? '…' }}</span>
        <button class="fermer" @click="$emit('fermer')">✕</button>
      </header>

      <div v-if="alerte" class="corps">
        <!-- SMC approche / contact : la capture + les niveaux -->
        <template v-if="alerte.type === 'smc_approche' || alerte.type === 'smc_contact'">
          <img v-if="alerte.png && !demo" :src="urlPng" class="capture" alt="Graphique de la zone" />
          <p v-else-if="alerte.png" class="capture-factice">
            📷 Capture du graphique (bougies + zone) — rendue par le serveur à
            l'émission de l'alerte ; visible après relance de l'app principale.
          </p>
          <div class="grille">
            <div><label>Sens</label><b :class="alerte.payload.achat ? 'vert' : 'rouge'">{{ alerte.payload.achat ? 'ACHAT' : 'VENTE' }}</b></div>
            <div><label>Zone</label><b class="mono">{{ prix2(p.zone_bas) }} – {{ prix2(p.zone_haut) }}</b></div>
            <div v-if="niveaux"><label>Entrée</label><b class="mono">{{ prix2(niveaux.entree) }}</b></div>
            <div v-if="niveaux"><label>SL</label><b class="mono rouge">{{ prix2(niveaux.sl) }}</b></div>
          </div>
        </template>

        <!-- Événement High -->
        <template v-else-if="alerte.type === 'evenement'">
          <div class="grille">
            <div><label>Heure</label><b class="mono">{{ p.heure_paris }}</b></div>
            <div v-if="p.devise"><label>Devise</label><b>{{ p.devise }}</b></div>
            <div v-if="p.rappel_min"><label>Rappel</label><b class="mono">T-{{ p.rappel_min }} min</b></div>
          </div>
        </template>

        <!-- Presse -->
        <template v-else-if="alerte.type === 'presse'">
          <div class="grille">
            <div><label>Score news fonda</label><b class="mono">{{ p.score ?? '—' }}</b></div>
          </div>
          <p v-if="briefContenu" class="brief-presse">{{ briefContenu }}</p>
          <button v-else class="bouton-brief" :disabled="enCours" @click="genererBrief">
            {{ enCours ? 'Génération… (~1 min)' : '⚡ Générer le contexte de marché' }}
          </button>
          <p v-if="erreurBrief" class="note">{{ erreurBrief }}</p>
        </template>

        <!-- Santé -->
        <template v-else-if="alerte.type === 'sante'">
          <div class="grille">
            <div><label>Retard</label><b class="mono">{{ duree(p.retard_min) }}</b></div>
            <div><label>Seuil</label><b class="mono">{{ duree(p.seuil_min) }}</b></div>
          </div>
          <p class="note">Problème entre l'app et un fournisseur de données — vérifier 📦 dans l'app principale.</p>
        </template>

        <!-- Digest -->
        <template v-else-if="alerte.type === 'digest'">
          <div v-if="p.synthese" class="synthese">
            <p class="resume">{{ p.synthese.resume }}</p>
            <ul><li v-for="c in p.synthese.ce_qui_compte" :key="c">{{ c }}</li></ul>
          </div>
          <div v-if="p.nuit?.length" class="bloc">
            <label>🌙 Pendant la nuit</label>
            <p v-for="n in p.nuit" :key="n.ts" class="ligne-nuit">{{ n.titre }}</p>
          </div>
        </template>

        <p class="horodatage">{{ horodatage }}</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '../api'
import { prix2 } from '../format'
import { demoBriefPresse, demoAlerte } from '../fixtures'
import type { AlerteComplete, TypeAlerte } from '../types'

const props = defineProps<{ id: number; directe?: AlerteComplete | null }>()
defineEmits<{ (e: 'fermer'): void }>()

const demo = new URLSearchParams(location.search).has('demo')
const alerte = ref<AlerteComplete | null>(null)
const briefContenu = ref('')
const enCours = ref(false)
const erreurBrief = ref('')
// Affordance dev : ?demo=1&brief=1 montre le détail presse après clic.
if (demo && new URLSearchParams(location.search).has('brief')) {
  briefContenu.value = demoBriefPresse.contenu
}

const icones: Record<TypeAlerte, string> = {
  digest: '🌅', evenement: '📅', presse: '📰', smc_approche: '📍',
  smc_contact: '🎯', prix: '🔔', sante: '🩸',
  signal: '📈',
}

const icone = computed(() => (alerte.value ? icones[alerte.value.type] ?? '•' : '•'))
const p = computed(() => alerte.value?.payload ?? {})
const urlPng = computed(() => api.urlPng(props.id))
const horodatage = computed(() =>
  alerte.value
    ? new Date(alerte.value.ts * 1000).toLocaleString('fr-FR', { dateStyle: 'full', timeStyle: 'short' })
    : '',
)

/// Entrée/SL dérivés des bornes de zone : ACHAT → entrée au bord haut
/// (touché en premier par un prix au-dessus), SL sous le bord bas ; VENTE
/// en miroir. Même convention que le moteur SMC de l'app principale.
const niveaux = computed(() => {
  const q = p.value
  if (q.achat === undefined || q.zone_haut === undefined || q.zone_bas === undefined) return null
  return q.achat
    ? { entree: q.zone_haut, sl: q.zone_bas }
    : { entree: q.zone_bas, sl: q.zone_haut }
})

/// Minutes → durée lisible (« 45 min », « 9h », « 10h20 ») ; −1 = jamais
/// alimenté. Miroir de duree_lisible() côté serveur.
function duree(min?: number): string {
  if (min === undefined) return '—'
  if (min < 0) return 'jamais alimenté'
  if (min < 60) return `${min} min`
  const h = Math.floor(min / 60)
  const m = min % 60
  return m === 0 ? `${h}h` : `${h}h${String(m).padStart(2, '0')}`
}

/// « ⚡ Générer » : même brief presse que la Revue de presse de l'app
/// principale (sélection 24 h → LLM → markdown), PAS le briefing de
/// session de l'onglet Aujourd'hui (owner 17/10).
async function genererBrief() {
  enCours.value = true
  erreurBrief.value = ''
  try {
    if (demo) {
      await new Promise(r => setTimeout(r, 900))
      briefContenu.value = demoBriefPresse.contenu
    } else {
      briefContenu.value = (await api.genererBrief()).contenu
    }
  } catch {
    erreurBrief.value = 'Brief indisponible (Ollama ou serveur) — réessayer plus tard.'
  } finally {
    enCours.value = false
  }
}

onMounted(async () => {
  if (props.directe) {
    alerte.value = props.directe
    return
  }
  if (demo) {
    alerte.value = demoAlerte(props.id)
    return
  }
  try {
    alerte.value = await api.alerte(props.id)
  } catch {
    // détail inaccessible : la feuille reste sur le titre seul
  }
})
</script>

<style scoped>
.voile {
  position: fixed;
  inset: 0;
  z-index: 40;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: flex-end;
}
.feuille {
  width: 100%;
  max-width: 480px;
  max-height: 88vh;
  overflow-y: auto;
  background: #10142e;
  border-radius: 20px 20px 0 0;
  border: 1px solid var(--bordure);
  padding-bottom: env(safe-area-inset-bottom);
}
header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px;
  border-bottom: 1px solid var(--bordure);
  position: sticky;
  top: 0;
  background: #10142e;
}
.ico { font-size: 20px; }
.titre { flex: 1; font-size: 14px; font-weight: 700; line-height: 1.3; }
.fermer { background: none; border: none; color: var(--texte-60); font-size: 16px; }
.corps { padding: 14px; display: flex; flex-direction: column; gap: 14px; }
.capture { width: 100%; border-radius: 10px; border: 1px solid var(--bordure); }
.capture-factice {
  font-size: 12px;
  color: var(--texte-60);
  line-height: 1.5;
  padding: 14px 12px;
  border: 1px dashed var(--bordure);
  border-radius: 10px;
}
.grille { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
.grille div { background: var(--carte); border: 1px solid var(--bordure); border-radius: 10px; padding: 8px 10px; }
.grille label { display: block; font-size: 10px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--texte-40); margin-bottom: 3px; }
.grille b { font-size: 14px; }
.vert { color: var(--vert); }
.rouge { color: var(--rouge); }
.note { font-size: 12px; color: var(--texte-60); line-height: 1.5; }
.resume { font-size: 13px; line-height: 1.55; }
.synthese ul { margin: 8px 0 0 16px; font-size: 12px; color: var(--texte-60); line-height: 1.6; }
.bloc label { font-size: 11px; text-transform: uppercase; color: var(--texte-40); }
.ligne-nuit { font-size: 13px; padding: 6px 0; border-bottom: 1px solid rgba(255, 255, 255, 0.05); }
.horodatage { font-size: 11px; color: var(--texte-40); }
.bouton-brief {
  padding: 11px;
  background: rgba(59, 130, 246, 0.14);
  border: 1px solid rgba(59, 130, 246, 0.45);
  border-radius: 10px;
  color: var(--bleu);
  font-size: 13px;
  font-weight: 600;
}
.bouton-brief:disabled { opacity: 0.55; }
.brief-presse {
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  padding: 12px;
  background: var(--carte);
  border: 1px solid var(--bordure);
  border-radius: 10px;
}
</style>
