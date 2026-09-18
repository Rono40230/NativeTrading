<template>
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- Volet gauche : sélection des assets -->
      <div class="flex flex-col gap-3">
        <div class="flex items-center gap-2">
          <p class="text-xs font-bold text-white uppercase tracking-wider">Assets surveillés</p>
          <span class="text-[10px] text-white/50 ml-auto">{{ selection.length }} sélectionné(s)</span>
        </div>
        <p class="text-[10px] text-amber-300/80 border-l-2 border-amber-400/40 pl-2">
          Rappel des décisions du 15/09 : BTC est la seule crypto du straddle ; tier 1 = annonces US sur les majeures.
        </p>
        <div v-for="cat in categories" :key="cat.type" class="flex flex-col gap-1">
          <p class="text-[11px] font-semibold uppercase tracking-wide" :class="cat.couleur">{{ cat.label }}</p>
          <div class="flex flex-wrap gap-1">
            <label
              v-for="a in cat.assets" :key="a"
              class="flex items-center gap-1.5 cursor-pointer rounded border px-2 py-1 text-xs transition select-none"
              :class="selection.includes(a)
                ? 'border-emerald-500/40 bg-emerald-500/10 text-white'
                : 'border-white/10 bg-white/[0.02] text-white/60 hover:bg-white/[0.06] hover:border-white/20'"
            >
              <input type="checkbox" class="hidden" :checked="selection.includes(a)" @change="basculer(a)" />
              {{ a }}
            </label>
          </div>
        </div>
        <div class="flex items-center gap-2 mt-1">
          <button class="text-xs px-3 py-1.5 rounded-lg font-semibold border border-white/10 transition-colors disabled:opacity-40"
                  :class="modifie ? 'bg-emerald-600/20 text-emerald-300 hover:bg-emerald-600/30' : 'bg-white/5 text-white'"
                  :disabled="!modifie || enCours" @click="enregistrer">
            {{ enCours ? '⏳…' : '✅ Enregistrer le périmètre' }}
          </button>
          <span v-if="message" class="text-[11px]" :class="erreur ? 'text-red-400' : 'text-emerald-300'">{{ message }}</span>
        </div>
      </div>

      <!-- Volet droit : créneaux armés + raccourcis -->
      <div class="flex flex-col gap-3">
        <div class="flex items-center gap-2">
          <p class="text-xs font-bold text-white uppercase tracking-wider">Créneaux armés ({{ creneauxArmes.length }}/{{ plafond }})</p>
          <button class="ml-auto text-[10px] px-2 py-1 rounded-lg bg-yellow-500/20 text-yellow-400 font-semibold hover:bg-yellow-500/30 transition"
                  title="Remplit les slots libres avec les têtes de la file ARMER (assets du périmètre uniquement)"
                  @click="armerFile">{{ armet ? '⏳…' : '⚡ Armer la file' }}</button>
        </div>
        <div v-if="creneauxArmes.length" class="flex flex-col gap-1.5 overflow-y-auto max-h-72 pr-1">
          <div v-for="c in creneauxArmes" :key="c.asset + c.jour + c.heure"
               class="rounded-lg border px-3 py-2 text-xs flex items-center gap-2 flex-wrap"
               :class="c.hors_perimetre
                 ? 'border-amber-500/40 bg-amber-500/10'
                 : 'border-emerald-500/30 bg-emerald-500/10'">
            <span class="font-bold text-white">{{ c.asset }} · {{ JOURS[c.jour - 1] }} {{ c.heure }}h</span>
            <span v-if="c.hors_perimetre" class="text-[9px] font-bold px-1.5 py-0.5 rounded-full bg-amber-500/20 text-amber-300 border border-amber-500/40"
                  title="Cet asset n'est plus dans le périmètre — le créneau restera armé mais ne tirera pas">⚠ HORS PÉRIMÈTRE — ignoré</span>
            <span class="ml-auto text-white/70">{{ c.occurrences }} tirage(s) · Σ{{ c.somme_r >= 0 ? '+' : '' }}{{ c.somme_r.toFixed(2) }}R</span>
          </div>
        </div>
        <p v-else class="text-xs text-white/60 py-2 text-center">Aucun créneau armé — la file attend ton clic.</p>
        <RouterLink to="/straddle"
          class="self-start text-[10px] px-2.5 py-1.5 rounded-lg bg-yellow-500/20 text-yellow-400 font-semibold hover:bg-yellow-500/30 transition"
          @click="$emit('fermer')">→ Agenda complet sur la page Straddle</RouterLink>
      </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { http } from '@/services/http.client'

interface AssetApi { id: string; type: 'crypto' | 'metal' | 'forex' | 'indice'; actif?: boolean }
interface CreneauArme { asset: string; jour: number; heure: number; occurrences: number; somme_r: number; hors_perimetre: boolean }


const JOURS = ['lundi', 'mardi', 'mercredi', 'jeudi', 'vendredi', 'samedi', 'dimanche']
const plafond = 8

const assets = ref<AssetApi[]>([])
const selection = ref<string[]>([])
const selectionInitiale = ref<string[]>([])
const creneauxArmes = ref<CreneauArme[]>([])
const enCours = ref(false)
const armet = ref(false)
const message = ref('')
const erreur = ref(false)

const categories = computed(() => [
  { type: 'crypto', label: '🪙 Cryptos', couleur: 'text-yellow-400', assets: assets.value.filter(a => a.type === 'crypto').map(a => a.id) },
  { type: 'metal', label: '🥇 Métaux', couleur: 'text-amber-400', assets: assets.value.filter(a => a.type === 'metal').map(a => a.id) },
  { type: 'forex', label: '💱 Forex', couleur: 'text-blue-400', assets: assets.value.filter(a => a.type === 'forex').map(a => a.id) },
  { type: 'indice', label: '📈 Indices', couleur: 'text-purple-400', assets: assets.value.filter(a => a.type === 'indice').map(a => a.id) },
])

const modifie = computed(() =>
  JSON.stringify([...selection.value].sort()) !== JSON.stringify([...selectionInitiale.value].sort()))

async function charger() {
  message.value = ''
  try {
    const [resA, resP, resC] = await Promise.all([
      http.get<AssetApi[]>('/api/assets'),
      http.get<{ assets: string[] }>('/api/straddle/perimetre'),
      http.get<{ slots: CreneauArme[] }>('/api/straddle/creneaux-ia'),
    ])
    assets.value = (resA.data ?? []).filter(a => a.actif !== false)
    selection.value = [...resP.data.assets]
    selectionInitiale.value = [...resP.data.assets]
    creneauxArmes.value = (resC.data.slots ?? []).filter(s => (s as unknown as { arme?: boolean }).arme)
      .map(s => ({ ...s, hors_perimetre: s.hors_perimetre ?? false }))
  } catch { /* modale vide */ }
}

function basculer(a: string) {
  const i = selection.value.indexOf(a)
  if (i >= 0) selection.value.splice(i, 1)
  else selection.value.push(a)
}

async function enregistrer() {
  enCours.value = true
  message.value = ''
  try {
    const r = await http.put<{ creneaux_armes_hors_perimetre: string[] }>('/api/straddle/perimetre', { assets: selection.value })
    selectionInitiale.value = [...selection.value]
    const hors = r.data?.creneaux_armes_hors_perimetre ?? []
    message.value = hors.length
      ? `✓ Appliqué au prochain tick (≤ 60 s) — ${hors.length} créneau(x) armé(s) hors périmètre : ${hors.join(', ')}`
      : '✓ Périmètre enregistré — appliqué au prochain tick (≤ 60 s)'
    erreur.value = false
    await charger()
    message.value = hors.length
      ? `✓ Appliqué ≤ 60 s — créneaux armés hors périmètre : ${hors.join(', ')}`
      : '✓ Périmètre enregistré — appliqué au prochain tick (≤ 60 s)'
  } catch (e) {
    erreur.value = true
    message.value = "❌ Échec de l'enregistrement"
  }
  enCours.value = false
}

async function armerFile() {
  armet.value = true
  try {
    await http.post('/api/straddle/creneaux-ia/armer-file')
    await charger()
  } catch { /* message dans l'agenda */ }
  armet.value = false
}

onMounted(charger)
</script>
