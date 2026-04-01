<template>
  <div
    v-if="open"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/70"
    @click.self="$emit('close')"
  >
    <div class="rounded-xl border border-white/10 p-6 w-full max-w-lg flex flex-col gap-5" style="background: #0d1117;">

      <!-- Header -->
      <div class="flex items-center justify-between">
        <h2 class="text-base font-bold">⬇ Export signaux</h2>
        <button class="text-gray-400 hover:text-white text-xl leading-none" @click="$emit('close')">×</button>
      </div>

      <!-- Format -->
      <div class="flex gap-2">
        <button
          v-for="f in ['CSV', 'PDF']" :key="f"
          class="flex-1 py-1.5 rounded-lg text-sm border transition-all"
          :class="format === f ? 'border-emerald-500 bg-emerald-600/20 text-emerald-400 font-semibold' : 'border-white/10 text-gray-400 hover:bg-white/5'"
          @click="format = f"
        >{{ f }}</button>
      </div>

      <!-- Filtres -->
      <div class="grid grid-cols-2 gap-3">

        <!-- Stratégie -->
        <div class="flex flex-col gap-1">
          <label class="label">Stratégie</label>
          <select v-model="filtres.strategie" class="select-champ">
            <option value="">Toutes</option>
            <option value="Straddle">⚡ Straddle</option>
            <option value="SmcDirectional">🧠 SMC Directionnel</option>
            <option value="Rockets">🚀 Rockets</option>
          </select>
        </div>

        <!-- Statut -->
        <div class="flex flex-col gap-1">
          <label class="label">Statut</label>
          <select v-model="filtres.statut" class="select-champ">
            <option value="">Tous</option>
            <option value="en_cours">⏳ En cours</option>
            <option value="clotures">✅ Clôturés</option>
          </select>
        </div>

        <!-- Direction (désactivé pour Rockets) -->
        <div class="flex flex-col gap-1">
          <label class="label">Direction</label>
          <select v-model="filtres.direction" class="select-champ" :disabled="filtres.strategie === 'Rockets'">
            <option value="">Toutes</option>
            <option value="LONG">📈 LONG</option>
            <option value="SHORT">📉 SHORT</option>
          </select>
        </div>

        <!-- Résultat -->
        <div class="flex flex-col gap-1">
          <label class="label">Résultat</label>
          <select v-model="filtres.verdict" class="select-champ">
            <option value="">Tous</option>
            <option value="TP1">TP1</option>
            <option value="TP2">TP2</option>
            <option value="TP3">TP3</option>
            <option value="SL">SL</option>
            <option value="expire">Expiré</option>
          </select>
        </div>

        <!-- Asset -->
        <div class="col-span-2 flex flex-col gap-1">
          <label class="label">Asset <span class="text-gray-600">(vide = tous)</span></label>
          <select v-model="filtres.asset" class="select-champ" :disabled="filtres.strategie === 'Rockets'">
            <option value="">Tous les assets</option>
            <option v-for="a in assetsDispos" :key="a" :value="a">{{ a }}</option>
          </select>
        </div>

        <!-- Période -->
        <div class="flex flex-col gap-1">
          <label class="label">Période</label>
          <select v-model="periodeRapide" class="select-champ" @change="appliquerPeriode">
            <option value="tout">Tout l'historique</option>
            <option value="7j">7 derniers jours</option>
            <option value="30j">30 derniers jours</option>
            <option value="90j">90 derniers jours</option>
            <option value="custom">Plage personnalisée</option>
          </select>
        </div>

        <!-- Séparateur (CSV uniquement) -->
        <div class="flex flex-col gap-1" v-if="format === 'CSV'">
          <label class="label">Séparateur</label>
          <select v-model="filtres.separateur" class="select-champ">
            <option value=",">, (virgule — standard)</option>
            <option value=";">; (point-virgule — Excel FR)</option>
          </select>
        </div>

        <!-- Dates custom -->
        <template v-if="periodeRapide === 'custom'">
          <div class="flex flex-col gap-1">
            <label class="label">Date début</label>
            <input type="date" v-model="dateDebut" class="select-champ" />
          </div>
          <div class="flex flex-col gap-1">
            <label class="label">Date fin</label>
            <input type="date" v-model="dateFin" class="select-champ" />
          </div>
        </template>

        <!-- Limite -->
        <div class="col-span-2 flex flex-col gap-1">
          <label class="label">Nombre max de lignes</label>
          <select v-model.number="filtres.limit" class="select-champ">
            <option :value="500">500</option>
            <option :value="1000">1 000</option>
            <option :value="2000">2 000</option>
            <option :value="5000">5 000</option>
            <option :value="10000">10 000</option>
          </select>
        </div>
      </div>

      <!-- Note Rockets -->
      <p v-if="filtres.strategie === 'Rockets'" class="text-xs text-yellow-400/80 bg-yellow-900/10 border border-yellow-500/20 rounded-lg px-3 py-2">
        🚀 Les Rockets sont exportées dans un fichier séparé (<code>rockets.csv</code>) avec leurs colonnes spécifiques (phase, ratio volume, prix peak…).
      </p>

      <!-- Erreur -->
      <p v-if="erreur" class="text-xs text-red-400">{{ erreur }}</p>

      <!-- Actions -->
      <div class="flex gap-3 justify-end">
        <button class="btn-annuler" @click="$emit('close')">Annuler</button>
        <button class="btn-exporter" :disabled="chargement" @click="exporter">
          <span v-if="chargement">⏳ Export en cours…</span>
          <span v-else>⬇ Télécharger {{ format }}</span>
        </button>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { apiService } from '@/services/api.service'
import { useAlerteStore } from '@/stores/alerte.store'
import { useExportPdf } from '@/composables/useExportPdf'

const props = defineProps<{
  open: boolean
  assetsDispos: string[]
}>()
defineEmits<{ close: [] }>()

const alerteStore = useAlerteStore()
const { genererPdf } = useExportPdf()
const chargement  = ref(false)
const erreur      = ref('')
const format      = ref<'CSV' | 'PDF'>('CSV')
const periodeRapide = ref('tout')
const dateDebut   = ref('')
const dateFin     = ref('')

const filtres = reactive({
  limit:      2000,
  strategie:  '',
  statut:     '',
  direction:  '',
  asset:      '',
  verdict:    '',
  separateur: ';',
  depuis_ts:  undefined as number | undefined,
  jusqu_ts:   undefined as number | undefined,
})

watch(() => filtres.strategie, (val) => {
  if (val === 'Rockets') {
    filtres.direction = ''
    filtres.asset     = ''
  }
})

function appliquerPeriode() {
  const now = Math.floor(Date.now() / 1000)
  if (periodeRapide.value === 'tout')   { filtres.depuis_ts = undefined; filtres.jusqu_ts = undefined }
  if (periodeRapide.value === '7j')     { filtres.depuis_ts = now - 7   * 86400; filtres.jusqu_ts = undefined }
  if (periodeRapide.value === '30j')    { filtres.depuis_ts = now - 30  * 86400; filtres.jusqu_ts = undefined }
  if (periodeRapide.value === '90j')    { filtres.depuis_ts = now - 90  * 86400; filtres.jusqu_ts = undefined }
  if (periodeRapide.value === 'custom') { filtres.depuis_ts = undefined; filtres.jusqu_ts = undefined }
}

watch([dateDebut, dateFin], () => {
  if (periodeRapide.value !== 'custom') return
  filtres.depuis_ts = dateDebut.value ? Math.floor(new Date(dateDebut.value).getTime() / 1000) : undefined
  filtres.jusqu_ts  = dateFin.value   ? Math.floor(new Date(dateFin.value).getTime()   / 1000) : undefined
})

async function exporter() {
  erreur.value     = ''
  chargement.value = true
  const estRockets = filtres.strategie === 'Rockets'
  try {
    const blob = await apiService.exporterCsv({ ...filtres })
    if (format.value === 'PDF') {
      const nomPdf = estRockets ? 'rockets.pdf' : 'signaux.pdf'
      await genererPdf(blob, filtres.separateur, nomPdf, filtres.strategie)
      alerteStore.afficherSucces(`Export PDF « ${nomPdf} » téléchargé`)
    } else {
      const nomCsv = estRockets ? 'rockets.csv' : 'signaux.csv'
      const url    = URL.createObjectURL(blob)
      const a      = document.createElement('a')
      a.href       = url
      a.download   = nomCsv
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      alerteStore.afficherSucces(`Export « ${nomCsv} » téléchargé`)
    }
  } catch (e: unknown) {
    erreur.value = `Erreur export : ${(e as Error).message}`
    alerteStore.afficherErreur(erreur.value)
  } finally {
    chargement.value = false
  }
}
</script>

<style scoped>
.label        { @apply text-xs text-gray-400; }
.select-champ { @apply bg-white border border-gray-300 text-black text-sm rounded-lg px-3 py-2 w-full disabled:opacity-40; }
.select-champ option { @apply text-black bg-white; }
.btn-annuler  { @apply px-4 py-2 rounded-lg text-sm text-gray-400 hover:text-white border border-white/10 hover:bg-white/5 transition-all; }
.btn-exporter { @apply px-4 py-2 rounded-lg text-sm font-semibold bg-emerald-600 hover:bg-emerald-500 text-white transition-all disabled:opacity-40 disabled:cursor-not-allowed; }
</style>
