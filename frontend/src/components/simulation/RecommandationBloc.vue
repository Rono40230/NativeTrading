<template>
  <!-- Étape 12 (roadmap audit) — l'advisory honnête : le meilleur essai du
       labo (R total max, effectif ≥ 30) vs la configuration actuelle, avec
       activation AU CLIC propriétaire (écriture + relecture-vérifiée côté
       backend). Aucune suggestion inventée : uniquement du mesuré. -->
  <div v-if="charge" class="rounded-lg border px-3 py-2 flex items-center gap-3 flex-wrap"
       :class="reco?.meilleur ? 'border-emerald-500/30 bg-emerald-500/5' : 'border-white/10 bg-white/[0.02]'">
    <span class="text-[11px] font-bold uppercase tracking-wider text-white">🎯 Recommandation</span>
    <span v-if="asset" class="text-[9px] font-bold px-2 py-0.5 rounded-full border border-amber-400/30 bg-amber-400/10 text-amber-300">PAR ASSET {{ asset }}</span>
    <template v-if="reco?.meilleur">
      <span class="text-[11px] text-white">
        Essai <span class="font-mono font-bold text-white">{{ reco.meilleur.id.slice(6, 14) }}</span> :
        <span class="font-bold text-emerald-300">{{ fmtR(reco.meilleur.r_total) }} R</span>
        sur {{ reco.meilleur.nb_trades }} trades (WR {{ Math.round(reco.meilleur.taux_reussite * 100) }} %)
      </span>
      <span v-if="reco.delta_r !== null" class="text-[11px]" :class="(reco.delta_r ?? 0) > 0 ? 'text-emerald-300 font-bold' : 'text-white/60'">
        {{ (reco.delta_r ?? 0) > 0 ? `+${fmtR(reco.delta_r ?? 0)} R vs ta config actuelle` : '≈ ta config actuelle' }}
      </span>
      <span v-else class="text-[10px] text-white/50">aucun essai à ta config actuelle — juge sur les chiffres de l'essai</span>
      <button v-if="reco.activable && (reco.delta_r === null || (reco.delta_r ?? 0) > 0)" :disabled="enCours"
              class="ml-auto text-[10px] px-2.5 py-1 rounded-lg font-semibold bg-emerald-600/25 text-emerald-200 hover:bg-emerald-600/40 transition disabled:opacity-40"
              :title="reco.delta_r === null
                ? 'Aucun essai témoin de ta config actuelle — cet essai se recommande sur ses propres chiffres. Écrit ses paramètres dans les réglages réels, relit et vérifie.'
                : 'Écrit les paramètres de cet essai dans les réglages réels, relit et vérifie — appliqué au prochain cycle moteur.'"
              @click="activer">{{ enCours ? '⏳…' : '⚡ Activer cette config' }}</button>
      <span v-else-if="!reco.activable" class="ml-auto text-[9px] text-white/40">espace de simulation — activation non applicable</span>
    </template>
    <span v-else class="text-[11px] text-white/60">{{ reco?.message || 'Aucun essai éligible.' }}</span>
    <span v-if="msg" class="text-[10px] w-full" :class="erreur ? 'text-red-300' : 'text-emerald-300'">{{ msg }}</span>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { http } from '@/services/http.client'

const props = defineProps<{ strategie: string; asset?: string }>()

interface Recommandation {
  meilleur: { id: string; r_total: number; nb_trades: number; taux_reussite: number } | null
  delta_r: number | null
  activable: boolean
  message: string
}

const reco = ref<Recommandation | null>(null)
const charge = ref(false)
const enCours = ref(false)
const msg = ref('')
const erreur = ref(false)

function fmtR(r: number): string {
  return r >= 0 ? `+${r.toFixed(1)}` : r.toFixed(1)
}

async function charger() {
  try {
    const r = await http.get<Recommandation>(`/api/strategies/${props.strategie}/recommandation`, {
      params: props.asset ? { asset: props.asset } : undefined,
    })
    reco.value = r.data
    charge.value = true
  } catch { charge.value = false }
}

async function activer() {
  if (!reco.value?.meilleur) return
  enCours.value = true
  msg.value = ''
  try {
    const r = await http.post<{ ok: boolean; verifie: boolean }>(
      `/api/strategies/${props.strategie}/recommandation/activer`,
      { essai_id: reco.value.meilleur.id },
      // PAR ASSET : l'activation écrit dans la SURCHARGE de l'asset.
      { params: props.asset ? { asset: props.asset } : undefined, timeout: 30_000 },
    )
    msg.value = r.data?.verifie ? 'Config appliquée et vérifiée par relecture ✓' : 'Réponse inattendue'
    erreur.value = !r.data?.verifie
    await charger()
  } catch (e) {
    msg.value = `Échec : ${(e as Error).message}`
    erreur.value = true
  }
  enCours.value = false
}

onMounted(charger)
watch(() => props.asset, charger)
</script>
