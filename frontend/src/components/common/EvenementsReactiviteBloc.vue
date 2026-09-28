<template>
  <!-- ÉVÉNEMENTS PRÉVISIBLES × RÉACTIVITÉ (28/09) — la taxonomie à
       l'horloge (ouvertures, fixes LBMA, réouverture CME, slots d'annonces
       US) croisée avec la réactivité mesurée de chaque asset : l'ATR des 3
       premières minutes de l'événement rapporté à l'habitude M1 (×N).
       Chaque événement vit dans son fuseau d'origine — l'heure Paris suit
       les bascules été/hiver, y compris les 2 fenêtres de l'année où
       l'Europe et les US ne basculent pas ensemble. -->
  <div class="glass-card px-4 py-2 flex flex-col gap-1.5">

    <!-- En-tête cliquable : repliable comme le bloc créneaux ci-dessous -->
    <div class="flex items-center justify-between shrink-0 gap-2 cursor-pointer select-none" @click="ouvert = !ouvert">
      <p class="text-[11px] font-semibold text-white uppercase tracking-widest">
        <span class="inline-block transition-transform" :class="ouvert ? 'rotate-90' : ''">▸</span>
        📯 Événements prévisibles × réactivité
        <span v-if="!ouvert && evenements.length" class="text-white font-normal normal-case tracking-normal">
          · {{ evenements.length }} événements
        </span>
      </p>
      <span class="text-[9px] text-white shrink-0">heures Paris · été/hiver géré</span>
    </div>

    <div v-if="ouvert && chargement" class="text-center text-white text-xs py-3">Calcul…</div>
    <div v-else-if="ouvert && !evenements.length" class="text-center text-white text-xs py-3">Aucune donnée</div>

    <template v-if="ouvert && evenements.length">
      <!-- Une ligne par événement, triée par pointe décroissante (le
           classement vient du backend : max_ratio). -->
      <div class="flex flex-col gap-1.5 max-h-[46vh] overflow-y-auto pr-0.5">
        <div v-for="ev in evenementsTriees" :key="ev.ident"
             class="rounded-lg border border-white/10 bg-black/20 px-2.5 py-1.5 flex flex-col gap-1">
          <div class="flex items-baseline justify-between gap-x-3 gap-y-0.5 flex-wrap">
            <p class="text-[11px] font-bold text-white leading-snug">
              {{ ev.nom }}
              <PopoverInfo :titre="ev.nom" :texte="texteInfo(ev)">
                <span class="k-info cursor-help">ⓘ</span>
              </PopoverInfo>
            </p>
            <span class="text-[9px] font-mono text-white shrink-0">
              <span class="font-bold">{{ ev.prochaine_heure_paris ?? '—' }}</span> Paris
              <span class="text-white/50">· {{ ev.heure_locale }} {{ nomFuseau(ev.fuseau) }}</span>
              <span v-if="prochaineCourt(ev)" class="text-white/50"> · proch. {{ prochaineCourt(ev) }}</span>
            </span>
          </div>
          <div class="flex flex-wrap items-center gap-1">
            <span v-for="l in lignesVisibles(ev)" :key="ev.ident + l.asset"
                  class="text-[9px] font-mono font-bold rounded px-1.5 py-[1px] border"
                  :class="[classeRatio(l.ratio), estDuPerimetre(l.asset) ? '' : 'opacity-40']"
                  :title="`${l.asset} : ATR ${l.atr} vs habitude ${l.habitude} — ${l.minutes} min observées`">
              {{ l.asset }} ×{{ l.ratio.toFixed(1) }}
            </span>
            <button v-if="ev.reactivite.length > VISIBLES && !deplie.has(ev.ident)"
                    class="text-[9px] text-white/70 hover:text-white underline underline-offset-2 cursor-pointer"
                    @click.stop="deplier(ev.ident)">
              +{{ ev.reactivite.length - VISIBLES }}
            </button>
          </div>
        </div>
      </div>
      <p class="text-[8px] text-white leading-snug">
        ATR des {{ fenetre }} premières minutes de l'événement vs habitude M1 de l'asset ({{ periode }} jours) — ×2 = deux fois le range normal.
        Chips en clair : assets du périmètre straddle (modale 🎯 Choix des Assets & créneaux) ; chips atténuées : le reste du pipeline, pour la découverte.
        L'ordre des événements suit la meilleure réaction DANS le périmètre.
        Heures Paris avec bascules été/hiver (annonces US 8:30 : 14:30 presque toute l'année, 13:30 pendant les entre-bascules).
      </p>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { EvenementPrevisible, EvenementReactif, ReponseEvenements } from '@/services/api.service'
import { lirePerimetreStraddle } from '@/composables/usePerimetreStraddle'
import PopoverInfo from './PopoverInfo.vue'

/// Déplié par défaut quand monté dans la modale Créneaux (l'info nouvelle
/// se lit sans clic ; le bloc créneaux statistiques reste dessous).
const props = defineProps<{ ouvertDefaut?: boolean }>()

const VISIBLES = 8
const ouvert = ref(!!props.ouvertDefaut)
const chargement = ref(true)
const evenements = ref<EvenementPrevisible[]>([])
/// Sélection de la modale 🎯 Choix des Assets & créneaux — null pendant le
/// chargement : aucune atténuation. Liste vide (ou fetch en échec) = idem.
const perimetre = ref<string[] | null>(null)

function estDuPerimetre(asset: string): boolean {
  return perimetre.value === null || perimetre.value.length === 0 || perimetre.value.includes(asset)
}

/// Le classement des événements suit la meilleure réaction des assets du
/// périmètre (repli global si le périmètre n'a aucune ligne mesurée).
const evenementsTriees = computed<EvenementPrevisible[]>(() => {
  const cle = (ev: EvenementPrevisible) => {
    const dans = ev.reactivite.filter(l => perimetre.value?.includes(l.asset) ?? false)
    const source = dans.length ? dans : ev.reactivite
    return Math.max(0, ...source.map(l => l.ratio))
  }
  return [...evenements.value].sort((a, b) => cle(b) - cle(a))
})
const periode = ref(120)
const fenetre = ref(3)
/// Lignes dépliées (au-delà des 8 premières) — un Set réactif par ident.
let deplie = ref(new Set<string>())

function deplier(ident: string) {
  deplie.value.add(ident)
  deplie.value = new Set(deplie.value)
}

function lignesVisibles(ev: EvenementPrevisible): EvenementReactif[] {
  return deplie.value.has(ev.ident) ? ev.reactivite : ev.reactivite.slice(0, VISIBLES)
}

/// L'intensité se lit d'un coup d'œil : ≥×3 rouge chaud, ≥×2 orange,
/// ≥×1,5 ambre — en dessous, l'événement n'est pas une source de range.
function classeRatio(r: number): string {
  if (r >= 3) return 'text-rose-300 bg-rose-500/15 border-rose-400/40'
  if (r >= 2) return 'text-orange-300 bg-orange-500/15 border-orange-400/40'
  if (r >= 1.5) return 'text-amber-300 bg-amber-500/10 border-amber-400/30'
  return 'text-white/70 bg-white/5 border-white/15'
}

function nomFuseau(fuseau: string): string {
  if (fuseau === 'America/New_York') return 'NY'
  if (fuseau === 'Europe/London') return 'Londres'
  return ''
}

/// Prochaine occurrence en heure Paris : « mar. 14:30 ».
function prochaineCourt(ev: EvenementPrevisible): string {
  if (!ev.prochaine_ts) return ''
  return new Intl.DateTimeFormat('fr-FR', {
    timeZone: 'Europe/Paris', weekday: 'short', hour: '2-digit', minute: '2-digit',
  }).format(new Date(ev.prochaine_ts * 1000))
}

/// Le popover complète le détail par la tête du classement.
function texteInfo(ev: EvenementPrevisible): string {
  const tete = ev.reactivite.slice(0, 3)
    .map(l => `${l.asset} ×${l.ratio.toFixed(1)}`)
    .join(' · ')
  const jours = ev.jours.length === 5 ? 'lun–ven' : 'dimanche'
  return `${ev.detail}\n\nJours : ${jours} (${ev.heure_locale} ${nomFuseau(ev.fuseau) || 'Paris'}).\nPlus réactifs : ${tete || '—'}.`
}

onMounted(async () => {
  try {
    const rep: ReponseEvenements = await apiService.obtenirMatriceEvenements()
    evenements.value = rep.evenements
    periode.value = rep.periode_jours
    fenetre.value = rep.fenetre_minutes
  } catch { evenements.value = [] }
  // Périmètre en parallèle : il ne bloque pas l'affichage de la matrice.
  perimetre.value = await lirePerimetreStraddle()
  chargement.value = false
})
</script>
