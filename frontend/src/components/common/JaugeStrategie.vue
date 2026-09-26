<template>
  <!-- JAUGE STRATÉGIE (refonte cockpit 24/09) : un instrument analogique
       du six-pack par stratégie. Cadrant 270°, arcs gravés vert (0 → +50 %)
       et rouge (−50 % → 0), aiguille blanche à contre-poids, fenêtre
       digitale du capital (style altimètre), Σ R en contre-affichage,
       plaque gravée + lampe d'état + vis d'angle. Auto-alimentée (analyse
       + capital + signaux, 60 s). La navigation et la pastille Telegram
       sont portées par la carte-colonne. -->
  <div class="flex flex-col items-center gap-1 select-none">
    <!-- ══ L'instrument ══ -->
    <div class="relative">
      <svg viewBox="0 0 200 200" class="w-[150px] h-[150px] md:w-[170px] md:h-[170px] drop-shadow-[0_6px_10px_rgba(0,0,0,0.5)]">

        <!-- Bezel métallique (anneau biseauté) -->
        <defs>
          <linearGradient id="bezel" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#4b5563" />
            <stop offset="35%" stop-color="#1f2937" />
            <stop offset="65%" stop-color="#374151" />
            <stop offset="100%" stop-color="#111827" />
          </linearGradient>
          <radialGradient id="cadran" cx="50%" cy="42%" r="65%">
            <stop offset="0%" stop-color="#111826" />
            <stop offset="78%" stop-color="#0a0e18" />
            <stop offset="100%" stop-color="#060910" />
          </radialGradient>
          <linearGradient id="verre" x1="0%" y1="0%" x2="65%" y2="100%">
            <stop offset="0%" stop-color="rgba(255,255,255,0.10)" />
            <stop offset="38%" stop-color="rgba(255,255,255,0.02)" />
            <stop offset="100%" stop-color="rgba(255,255,255,0)" />
          </linearGradient>
        </defs>
        <circle cx="100" cy="100" r="98" fill="url(#bezel)" />
        <circle cx="100" cy="100" r="98" fill="none" stroke="#0b0f19" stroke-width="1.5" />
        <circle cx="100" cy="100" r="88" fill="url(#cadran)" stroke="#000" stroke-width="2" />

        <!-- Arcs gravés : rouge (−50 → 0) puis vert (0 → +50) -->
        <path :d="arc(-135, 0)" fill="none" stroke="#dc2626" stroke-width="7" opacity="0.85" />
        <path :d="arc(0, 135)" fill="none" stroke="#10b981" stroke-width="7" opacity="0.85" />

        <!-- Graduations gravées (tous les 10 %, index aux extrêmes/zéro) -->
        <g stroke="rgba(255,255,255,0.75)" stroke-width="1.6">
          <line v-for="t in graduations" :key="t.angle"
            :x1="polaire(70, t.angle).x" :y1="polaire(70, t.angle).y"
            :x2="polaire(t.index ? 60 : 65, t.angle).x" :y2="polaire(t.index ? 60 : 65, t.angle).y" />
        </g>

        <!-- Chiffres gravés aux index -->
        <text v-for="t in graduations.filter(g => g.index)" :key="'lbl' + t.angle"
          :x="polaire(51, t.angle).x" :y="polaire(51, t.angle).y + 3"
          text-anchor="middle" class="fill-white/60" style="font-size: 9px; font-weight: 700; font-family: ui-monospace, monospace">{{ t.label }}</text>

        <!-- Aiguille : pivot central, transitions physiques. Dessinée SOUS les
             affichages numériques : elle glisse derrière les fenêtres, comme
             un vrai instrument où les compteurs restent lisibles. -->
        <g :style="{ transform: `rotate(${angleAiguille}deg)`, transformOrigin: '100px 100px', transition: 'transform 900ms cubic-bezier(0.2, 0.8, 0.3, 1)' }">
          <polygon points="100,32 103.2,100 96.8,100" fill="#f8fafc" stroke="#cbd5e1" stroke-width="0.5" />
          <polygon points="100,112 103,128 97,128" fill="#94a3b8" />
        </g>
        <circle cx="100" cy="100" r="7" fill="#1f2937" stroke="#9ca3af" stroke-width="1.5" />
        <circle cx="100" cy="100" r="2.4" fill="#e5e7eb" />

        <!-- Fenêtre digitale (capital $, style altimètre) -->
        <rect x="62" y="52" width="76" height="21" rx="3" fill="#020409" stroke="rgba(255,255,255,0.18)" stroke-width="1" />
        <text x="100" y="67" text-anchor="middle" :class="couleurCapitale"
          style="font-size: 13px; font-weight: 700; font-family: ui-monospace, monospace">{{ capitalCourt }}</text>

        <!-- Contre-affichage Σ R distance -->
        <rect x="76" y="106" width="48" height="15" rx="2.5" fill="#020409" stroke="rgba(255,255,255,0.10)" stroke-width="0.8" />
        <text x="100" y="117" text-anchor="middle" :class="rTotal >= 0 ? 'fill-emerald-400' : 'fill-red-400'"
          style="font-size: 9.5px; font-weight: 700; font-family: ui-monospace, monospace">{{ rFormate }}</text>
        <text x="100" y="136" text-anchor="middle" class="fill-white/40" style="font-size: 6.5px; font-weight: 700; letter-spacing: 1.5px">Σ R DIST</text>

        <!-- Reflet de verre -->
        <ellipse cx="80" cy="62" rx="52" ry="30" fill="url(#verre)" transform="rotate(-18 80 62)" />
      </svg>
    </div>

    <!-- ══ La plaque gravée sous l'instrument ══ -->
    <div class="plaque relative flex flex-col items-center gap-0.5 px-3 pt-1 pb-1.5 rounded-md">
      <!-- Vis d'angle -->
      <span class="vis gauche" /><span class="vis droite" />
      <p class="text-[10px] font-bold tracking-[0.2em] uppercase text-white/75">{{ icone }} {{ nom }}</p>
      <div class="flex items-center gap-2">
        <span class="flex items-center gap-1 text-[8px] font-bold tracking-wider uppercase"
              :class="etat === 'Officielle' ? 'text-emerald-300' : etat === 'Observation' ? 'text-amber-300' : 'text-white/50'">
          <span class="w-1.5 h-1.5 rounded-full" :class="lampeEtat" />{{ etat }}
        </span>
        <PopoverInfo v-if="enCours > 0" :titre="`${enCours} signal(s) en cours`" :texte="detailEnCours || '—'">
          <span class="flex items-center gap-1 text-[8px] font-bold tracking-wider uppercase text-sky-300 cursor-help">
            <span class="w-1.5 h-1.5 rounded-full bg-sky-400 animate-pulse" />{{ enCours }} EN COURS
          </span>
        </PopoverInfo>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { http } from '@/services/http.client'
import { chargerAnalyse } from '@/composables/useAnalyses'
import { estTradeOuvert } from '@/composables/useSignalFormat'
import PopoverInfo from './PopoverInfo.vue'

const props = defineProps<{
  id: string
  nom: string
  icone: string
}>()

interface AnalyseApi { r_distance_total?: number; nb_trades?: number; taux_reussite?: number }
const rTotal = ref(0)
const capitalActuel = ref<number | null>(null)
const capitalDepart = ref(1000)
const etat = ref('—')
const enCours = ref(0)
const detailEnCours = ref('')

async function sonder() {
  try {
    const a = await chargerAnalyse(props.id) as unknown as (AnalyseApi & { etat?: string })
    rTotal.value = a.r_distance_total ?? 0
  } catch { /* silencieux */ }
  try {
    const c = await http.get(`/api/strategies/${props.id}/capital`)
    capitalActuel.value = c.data?.capital_actuel ?? null
    capitalDepart.value = c.data?.capital_depart ?? 1000
  } catch { /* silencieux */ }
  try {
    const r = await http.get('/api/strategies')
    const s = (r.data as { id: string; etat: string }[]).find(x => x.id === props.id)
    if (s) etat.value = s.etat
  } catch { /* silencieux */ }
  try {
    const r = await http.get('/api/signaux', { params: { limit: 150 } })
    // La lampe ne compte que les trades OUVERTS — un ordre posé en attente
    // (entrée non touchée) n'est pas « en cours » (même sémantique que le
    // tableau de la page stratégie).
    const ouverts = (r.data as { strategie: string; statut: string; verdict: string | null; heure_entree: number | null; asset: string; timeframe: string }[])
      .filter(s => s.strategie === props.id && estTradeOuvert(s))
    enCours.value = ouverts.length
    detailEnCours.value = ouverts.map(a => `${a.asset} ${a.timeframe}`).join('\n')
  } catch { /* silencieux */ }
}

/// Perf capital mappée sur [−50 %, +50 %] — l'aiguille: −135° → +135°.
const angleAiguille = computed(() => {
  const perf = capitalActuel.value === null ? 0 : (capitalActuel.value / capitalDepart.value - 1) * 100
  const t = Math.max(-50, Math.min(50, perf)) / 100 + 0.5
  return -135 + t * 270
})

const capitalCourt = computed(() =>
  capitalActuel.value === null ? '—' : Math.round(capitalActuel.value).toLocaleString('fr-FR') + ' $')
const couleurCapitale = computed(() => {
  if (capitalActuel.value === null) return 'fill-white/60'
  return capitalActuel.value >= capitalDepart.value ? 'fill-emerald-400' : 'fill-red-400'
})
const rFormate = computed(() =>
  `${rTotal.value >= 0 ? '+' : '−'}${Math.abs(rTotal.value).toFixed(1)} R`)
const lampeEtat = computed(() =>
  etat.value === 'Officielle' ? 'bg-emerald-400' : etat.value === 'Observation' ? 'bg-amber-400' : 'bg-white/30')

// ── Géométrie du cadran ─────────────────────────────────────────────────
function polaire(r: number, angleDeg: number) {
  const a = angleDeg * Math.PI / 180
  return { x: 100 + r * Math.sin(a), y: 100 - r * Math.cos(a) }
}
/// Chemin d'arc gravé (grand arc implicite < 180°, nos arcs font 135°).
function arc(depuis: number, vers: number): string {
  const r = 76
  const d = polaire(r, depuis), f = polaire(r, vers)
  return `M ${d.x} ${d.y} A ${r} ${r} 0 0 1 ${f.x} ${f.y}`
}
/// Graduations : tous les 10 % (27°), index à −50 / 0 / +50.
const graduations = Array.from({ length: 11 }, (_, i) => {
  const angle = -135 + i * 27
  const val = -50 + i * 10
  return { angle, index: val % 50 === 0, label: `${val > 0 ? '+' : ''}${val}` }
})

let poll: ReturnType<typeof setInterval> | null = null
onMounted(() => {
  void sonder()
  poll = setInterval(sonder, 60_000)
})
onUnmounted(() => { if (poll !== null) clearInterval(poll) })
</script>

<style scoped>
/* Plaque gravée sous l'instrument : métal mat, vis d'angle. */
.plaque {
  background: linear-gradient(180deg, #1c2431 0%, #131a26 100%);
  border: 1px solid rgba(255, 255, 255, 0.12);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.06), 0 3px 6px rgba(0, 0, 0, 0.45);
}
.vis {
  position: absolute;
  width: 5px; height: 5px;
  border-radius: 9999px;
  background: radial-gradient(circle at 35% 30%, #6b7280, #1f2937 70%);
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.6);
  top: 4px;
}
.vis.gauche { left: 4px; }
.vis.droite { right: 4px; }
</style>
