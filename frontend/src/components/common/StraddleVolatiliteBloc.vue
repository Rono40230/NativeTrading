<template>
  <div class="h-full flex flex-col">
    <!-- Barre résumé collapsed -->
    <div class="glass-bar px-4 py-2.5 flex flex-col gap-1 cursor-pointer hover:bg-white/5 transition-colors flex-1"
      @click="modalSurveillanceOuverte = true">
      <div class="flex items-center justify-between">
        <p class="text-xs font-semibold uppercase tracking-widest text-white">⚡ Straddle</p>
        <span class="text-[10px] text-gray-600">▸</span>
      </div>
      <span class="text-xs text-gray-500">Session&nbsp;:
        <span :class="sessionLabel.cls" class="font-semibold">{{ sessionLabel.label }}</span>
        <span v-if="sessionLabel.heures" class="text-gray-500 ml-1 text-[10px]">· {{ sessionLabel.heures }}</span>
      </span>
      <!-- Asset le plus chaud -->
      <span v-if="meilleurPic" class="text-xs font-semibold">
        <span class="text-white">{{ meilleurPic.asset }}</span>
        <span class="font-mono ml-1"
          :class="meilleurPic.ratio_atr >= 2.0 ? 'text-red-400' : meilleurPic.ratio_atr >= 1.5 ? 'text-orange-400' : 'text-yellow-400'">{{
            meilleurPic.ratio_atr.toFixed(1) }}×</span>
        <span class="ml-1 text-[10px] text-gray-400">{{ labelCategorie(meilleurPic.categorie) }}</span>
      </span>
      <span v-else class="text-xs text-gray-600 italic">Aucun pic actif</span>
      <!-- Prochaine annonce -->
      <span v-if="prochaineAnnonce" class="text-xs"
        :class="dansMoins30min(prochaineAnnonce.date_heure) ? 'text-red-400 font-bold animate-pulse' : 'text-orange-300'">
        {{ dansMoins30min(prochaineAnnonce.date_heure) ? '⚠️' : '📅' }} {{ prochaineAnnonce.devise }} {{
          prochaineAnnonce.titre }} {{ countdownAnnonce(prochaineAnnonce.date_heure) }}
      </span>
    </div>

    <!-- Contenu complet en modal -->
    <ModalSurveillance :visible="modalSurveillanceOuverte" titre="⚡ Surveillance Volatilité — Straddle ML"
      @close="modalSurveillanceOuverte = false">
      <div class="flex items-center justify-between mb-4">
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-gray-500">Session&nbsp;:
            <span :class="sessionLabel.cls" class="font-semibold">{{ sessionLabel.label }}</span>
            <span v-if="sessionLabel.heures" class="text-gray-500 ml-1">· {{ sessionLabel.heures }}</span>
          </span>
          <span v-if="data?.resume.annonces_prochaines_90min.some(a => a.dans_min <= 30)"
            class="text-[10px] font-bold text-red-400 border border-red-500/40 px-1.5 py-0.5 rounded animate-pulse">⚠️
            ZONE À RISQUE</span>
        </div>
      </div>

      <div v-if="chargement && !data" class="flex items-center justify-center py-8 text-xs text-gray-600">
        <span class="animate-pulse">Chargement volatilité...</span>
      </div>
      <div v-else-if="erreur" class="py-4 text-xs text-red-400 text-center">{{ erreur }}</div>
      <template v-else-if="data">
        <div v-if="data.resume.annonces_prochaines_90min.length" class="mb-4 flex flex-wrap gap-1.5">
          <div v-for="(a, i) in data.resume.annonces_prochaines_90min" :key="i"
            class="flex items-center gap-1 rounded-lg px-2 py-1 text-[10px] font-semibold"
            :class="a.dans_min <= 30 ? 'bg-red-900/50 border border-red-500/40 text-red-300' : 'bg-orange-900/30 border border-orange-500/30 text-orange-300'">
            <span>{{ a.dans_min <= 30 ? '🔴' : '🟡' }}</span>
                <span>{{ a.devise }} — {{ a.nom ?? a.impact }}</span>
                <span class="text-gray-400">dans {{ a.dans_min }}min</span>
          </div>
        </div>

        <div v-if="picsRecents.length" class="overflow-x-auto">
          <table class="w-full text-[11px]">
            <thead>
              <tr class="text-gray-500 border-b border-white/10">
                <th class="pb-1.5 text-left pr-3">Asset</th>
                <th class="pb-1.5 text-left pr-3">TF</th>
                <th class="pb-1.5 text-right pr-3">Ratio ATR</th>
                <th class="pb-1.5 text-left pr-3">Catégorie</th>
                <th class="pb-1.5 text-left pr-3">Événement</th>
                <th class="pb-1.5 text-left pr-3">Session</th>
                <th class="pb-1.5 text-center">Signal</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(p, i) in picsRecents" :key="i" class="border-b border-white/5 hover:bg-white/5">
                <td class="py-1 pr-3 font-semibold text-white">{{ p.asset }}</td>
                <td class="py-1 pr-3 text-gray-400">{{ p.timeframe }}</td>
                <td class="py-1 pr-3 text-right font-mono"
                  :class="p.ratio_atr >= 2.0 ? 'text-red-400' : p.ratio_atr >= 1.5 ? 'text-orange-400' : 'text-yellow-400'">
                  {{ p.ratio_atr.toFixed(2) }}×
                </td>
                <td class="py-1 pr-3">
                  <span class="rounded px-1.5 py-0.5 font-semibold text-[10px]" :class="badgeCategorie(p.categorie)">
                    {{ labelCategorie(p.categorie) }}
                  </span>
                </td>
                <td class="py-1 pr-3 text-gray-300 max-w-[120px] truncate">{{ p.evenement_nom ?? '—' }}</td>
                <td class="py-1 pr-3 text-gray-400">{{ p.session_active }}</td>
                <td class="py-1 text-center">
                  <span v-if="p.signal_genere" class="text-emerald-400 font-bold">✓</span>
                  <span v-else class="text-gray-600">—</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else class="py-6 text-center text-xs text-gray-600 italic">
          Aucun pic de volatilité détecté dans les 2 dernières heures
        </div>

        <div class="mt-3 pt-2 border-t border-white/5 flex items-center justify-between text-[10px] text-gray-500">
          <span>{{ data.resume.pics_2h }} asset(s) actifs · {{ picsRecents.length }} pic(s)</span>
          <span v-if="derniereMaj">MAJ {{ formatHeure(derniereMaj) }}</span>
        </div>
      </template>
    </ModalSurveillance>

    <StraddleVolatiliteModal :visible="modalOuverte" @close="modalOuverte = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'
import type { StraddleVolatiliteLive, AnnonceCalendrier } from '@/services/api.types'
import { useAlerteStore } from '@/stores/alerte.store'
import StraddleVolatiliteModal from './StraddleVolatiliteModal.vue'
import ModalSurveillance from './ModalSurveillance.vue'

const alerteStore = useAlerteStore()
const data = ref<StraddleVolatiliteLive | null>(null)
const chargement = ref(false)
const erreur = ref<string | null>(null)
const derniereMaj = ref<number | null>(null)
const modalOuverte = ref(false)
const modalSurveillanceOuverte = ref(false)
const annoncesCalendrier = ref<AnnonceCalendrier[]>([])
let intervalId: ReturnType<typeof setInterval> | null = null

const picsRecents = computed(() => data.value?.pics ?? [])

const meilleurPic = computed(() =>
  picsRecents.value.length
    ? [...picsRecents.value].sort((a, b) => b.ratio_atr - a.ratio_atr)[0]
    : null
)

const prochaineAnnonce = computed<AnnonceCalendrier | null>(() => {
  const maintenant = Date.now()
  const dans24h = maintenant + 24 * 3_600_000
  const prochaines = annoncesCalendrier.value.filter(a => {
    const ts = new Date(a.date_heure).getTime()
    return !a.est_passe && ts > maintenant && ts <= dans24h
  })
  if (!prochaines.length) return null
  return prochaines.sort((a, b) => new Date(a.date_heure).getTime() - new Date(b.date_heure).getTime())[0]
})

function countdownAnnonce(iso: string): string {
  const diff = new Date(iso).getTime() - Date.now()
  if (diff <= 0) return 'en cours'
  const min = Math.floor(diff / 60_000)
  if (min < 60) return `dans ${min}min`
  const h = Math.floor(min / 60)
  const m = min % 60
  return `dans ${h}h${m > 0 ? ` ${m}min` : ''}`
}

function dansMoins30min(iso: string): boolean {
  const diff = new Date(iso).getTime() - Date.now()
  return diff > 0 && diff < 30 * 60_000
}

const sessionLabel = computed(() => {
  const h = new Date().getUTCHours()
  if (h >= 23 || h < 1) return { label: 'Tokyo', heures: '23h–01h UTC', cls: 'text-cyan-400' }
  if (h < 7) return { label: 'Sydney', heures: '01h–07h UTC', cls: 'text-teal-400' }
  if (h < 13) return { label: 'London', heures: '07h–13h UTC', cls: 'text-sky-400' }
  if (h < 16) return { label: 'London/NY Overlap', heures: '13h–16h UTC', cls: 'text-purple-400' }
  if (h < 22) return { label: 'New York', heures: '16h–22h UTC', cls: 'text-blue-400' }
  return { label: 'Off-session', heures: '', cls: 'text-gray-500' }
})

function badgeCategorie(cat: string): string {
  const map: Record<string, string> = {
    annonce_high: 'bg-red-900/60 text-red-300 border border-red-500/40',
    annonce_medium: 'bg-orange-900/50 text-orange-300 border border-orange-500/30',
    overlap_lnd_ny: 'bg-purple-900/50 text-purple-300 border border-purple-500/30',
    ny_open: 'bg-blue-900/50 text-blue-300 border border-blue-500/30',
    london_open: 'bg-sky-900/50 text-sky-300 border border-sky-500/30',
    tokyo_open: 'bg-cyan-900/50 text-cyan-300 border border-cyan-500/30',
    creneau_recurrent: 'bg-yellow-900/50 text-yellow-300 border border-yellow-500/30',
    choc_isole: 'bg-gray-800 text-gray-400 border border-gray-600/30',
  }
  return map[cat] ?? 'bg-gray-800 text-gray-400'
}

function labelCategorie(cat: string): string {
  const map: Record<string, string> = {
    annonce_high: '🔴 High Impact',
    annonce_medium: '🟡 Medium',
    overlap_lnd_ny: '🟣 Overlap',
    ny_open: '🔵 NY Open',
    london_open: '🔵 London',
    tokyo_open: '🩵 Tokyo',
    creneau_recurrent: '⭐ Récurrent',
    choc_isole: '⬜ Choc isolé',
  }
  return map[cat] ?? cat
}

function formatHeure(ts: number): string {
  return new Date(ts).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })
}

async function charger() {
  chargement.value = true
  erreur.value = null
  try {
    data.value = await apiService.getStraddleVolatiliteLive()
    derniereMaj.value = Date.now()
  } catch (e: unknown) {
    erreur.value = (e as Error).message
    alerteStore.afficherErreur(`Volatilité live: ${(e as Error).message}`)
  } finally {
    chargement.value = false
  }
}

async function chargerCalendrier() {
  try {
    annoncesCalendrier.value = await apiService.obtenirCalendrier(2)
  } catch {
    // non-bloquant
  }
}

onMounted(() => {
  charger()
  chargerCalendrier()
  intervalId = setInterval(charger, 60_000)
})

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId)
})
</script>

<style scoped>
.glass-card {
  @apply rounded-xl border border-white/10 bg-white/5 backdrop-blur-sm;
}

.glass-bar {
  @apply rounded-xl border-2 border-yellow-500/50 bg-white/5 backdrop-blur-sm;
}
</style>
