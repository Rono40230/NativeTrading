<template>
  <main>
    <button class="titre-section pliable" @click="plieBrief = !plieBrief">
      <span>🌍 Briefing du jour</span>
      <span class="chevron" :class="{ ferme: plieBrief }" aria-hidden="true" />
    </button>
    <div v-if="!plieBrief" class="carte">
      <p v-if="encours" class="vide">Génération en cours — l'IA locale travaille, 1 à 2 min…</p>
      <template v-else-if="brief">
        <p class="meta">{{ heure(brief.genere_le) }} · {{ brief.nb_articles }} dépêches</p>
        <div class="contenu">
          <template v-for="(b, i) in blocs" :key="i">
            <hr v-if="b.titre && i > 0" class="separation" />
            <p v-if="b.titre" class="btitre">{{ b.titre }}</p>
            <p v-else-if="b.texte.trim()" class="btexte">{{ b.texte }}</p>
          </template>
        </div>
      </template>
      <p v-else class="vide">Pas encore de brief aujourd'hui</p>
      <button class="generer" :disabled="encours" @click="generer">
        ⚡ {{ brief ? 'Regénérer' : 'Générer le briefing' }}
      </button>
      <p v-if="erreur" class="erreur">{{ erreur }}</p>
    </div>

    <p class="titre-section">📰 Articles des dernières 24 h</p>
    <template v-for="s in sections" :key="s.id">
      <button class="titre-section pliable" @click="basculer(s.id)">
        <span>{{ s.emoji }} {{ s.nom }}<template v-if="!pliees[s.id] && s.articles.length"> · {{ s.articles.length }}</template></span>
        <span class="chevron" :class="{ ferme: pliees[s.id] }" aria-hidden="true" />
      </button>
      <div v-if="!pliees[s.id]" class="carte liste">
        <p v-if="s.articles.length === 0" class="vide">Aucun article dans cette section</p>
        <button
          v-for="(a, i) in s.articles"
          :key="i"
          class="ligne lien"
          @click="ouvrir(a.url)"
        >
          <span class="score mono">{{ a.score }}</span>
          <span class="corps">
            <span class="titre">{{ a.titre }}</span>
          </span>
          <span class="fleche-lien">↗</span>
        </button>
      </div>
    </template>
  </main>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '../api'
import { demoPresse } from '../fixtures'
import type { BriefPresse } from '../types'

const demo = new URLSearchParams(location.search).has('demo')
const articles = ref(demo ? demoPresse.articles : [])
const brief = ref<BriefPresse | null>(demo ? demoPresse.brief : null)
const encours = ref(false)
const erreur = ref('')
const android = '__TAURI_INTERNALS__' in window
const plieBrief = ref(false)
const pliees = ref<Record<string, boolean>>({})

function basculer(id: string) {
  pliees.value = { ...pliees.value, [id]: !pliees.value[id] }
}

function heure(ts: number): string {
  return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

/// Ligne de titre markdown (« **Titre** : » ou « ### Titre ») → libellé
/// nu, rendu en gras souligné (owner 20/10 — jamais d'astérisques visibles)
/// avec ligne horizontale de séparation entre sections (owner 22/10).
function ligneTitre(l: string): string | null {
  const l2 = l.trim()
  const gras = l2.match(/^\*\*(.+?)\s*:?\*\*\s*:?$/)
  if (gras) return gras[1].trim()
  const dieses = l2.match(/^#{1,4}\s+(.+)$/)
  if (dieses) return dieses[1].replace(/\*\*/g, '').trim()
  return null
}

const blocs = computed(() =>
  (brief.value?.contenu ?? '').split('\n').map((l) => {
    const t = ligneTitre(l)
    return t ? { titre: t, texte: '' } : { titre: '', texte: l }
  }),
)

/// 4 sections par thème (owner 22/10) : macro, cryptos, métaux, techno.
const SECTIONS = [
  { id: 'macro', nom: 'Macro', emoji: '🏛️' },
  { id: 'cryptos', nom: 'Cryptos', emoji: '₿' },
  { id: 'metaux', nom: 'Métaux', emoji: '⛏️' },
  { id: 'techno', nom: 'Techno', emoji: '💡' },
] as const

const sections = computed(() =>
  SECTIONS.map((s) => ({ ...s, articles: articles.value.filter((a) => a.section === s.id) })),
)

/// Ouvrir l'article : navigateur externe sur Android (plugin opener),
/// nouvel onglet dans la fenêtre de dev navigateur.
async function ouvrir(url: string) {
  if (!url) return
  try {
    if (android) {
      const { openUrl } = await import('@tauri-apps/plugin-opener')
      await openUrl(url)
    } else {
      window.open(url, '_blank', 'noopener')
    }
  } catch {
    window.open(url, '_blank', 'noopener')
  }
}

/// ⚡ Générer : la MÊME génération que la page 📰 de l'app principale
/// (POST /api/mobile/brief) — brief archivé côté serveur.
async function generer() {
  if (demo) {
    erreur.value = 'Mode démo — pas de génération'
    return
  }
  encours.value = true
  erreur.value = ''
  try {
    const r = await api.genererBrief()
    brief.value = { contenu: r.contenu, nb_articles: r.nb_articles, genere_le: Math.floor(Date.now() / 1000) }
  } catch (e) {
    erreur.value = `Échec : ${e instanceof Error ? e.message : 'inconnu'}`
  } finally {
    encours.value = false
  }
}

onMounted(async () => {
  if (demo) return
  try {
    const r = await api.presse()
    articles.value = r.articles
    brief.value = r.brief
  } catch {
    // hors ligne : bandeau rouge de App.vue
  }
})
</script>

<style scoped>
.carte { padding: 10px 12px; }
.meta { font-size: 11px; color: var(--texte-40); margin-bottom: 8px; }
.contenu { font-size: 13px; line-height: 1.55; overflow-wrap: anywhere; }
.separation { border: none; border-top: 1px solid var(--bordure); margin: 14px 0 0; }
.btitre {
  font-size: 13px;
  font-weight: 700;
  text-decoration: underline;
  margin: 4px 0;
}
.separation + .btitre { margin-top: 8px; }
.btexte { font-size: 13px; line-height: 1.55; white-space: pre-wrap; margin: 0 0 4px; }
.generer {
  margin-top: 10px;
  width: 100%;
  padding: 10px;
  border: 1px solid var(--bordure);
  border-radius: 10px;
  background: none;
  color: var(--bleu);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.generer:disabled { opacity: 0.5; }
.erreur { margin-top: 8px; font-size: 12px; color: var(--rouge); }
.section { margin-bottom: 12px; padding: 4px 6px; }
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
/* Chevron 100 % CSS (bordures) — même rendu garanti que Aujourd'hui. */
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
}
.ligne:last-of-type { border-bottom: none; }
.ligne.lien { cursor: pointer; }
.score { color: var(--ambre); font-weight: 700; min-width: 30px; font-size: 13px; }
.corps { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.titre { font-size: 13px; line-height: 1.3; }
.fleche-lien { color: var(--texte-40); font-size: 14px; }
.vide { color: var(--texte-40); font-size: 13px; padding: 10px 8px; }
</style>
