<template>
  <!-- Modale d'ajout d'un asset — extraite de la vue Données (limite 600 l.) -->
    <!-- ══ MODALE — Ajout d'un asset ═══════════════════════════════════════ -->
    <div
      v-if="modaleAsset"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/30"
      @click.self="fermerModaleAsset()"
    >
      <div class="w-full max-w-md p-6 space-y-4 rounded-2xl border border-white/10 bg-[#16181d] shadow-2xl">
        <div class="flex items-center justify-between">
          <h3 class="font-bold text-lg">Ajouter un asset</h3>
          <button class="text-white hover:text-white transition" @click="fermerModaleAsset()">✕</button>
        </div>

        <div class="space-y-3">
          <div>
            <label class="text-xs text-white">Ticker</label>
            <input
              v-model="nouvelAsset.ticker"
              placeholder="ex : TON, GBPAUD, HK50"
              class="w-full mt-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white uppercase focus:border-emerald-500/50 outline-none"
              @input="nouvelAsset.ticker = nouvelAsset.ticker.toUpperCase(); majWorkerEtSymboles()"
            />
          </div>
          <div>
            <label class="text-xs text-white">Nom</label>
            <input
              v-model="nouvelAsset.nom"
              placeholder="ex : Toncoin"
              class="w-full mt-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:border-emerald-500/50 outline-none"
            />
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="text-xs text-white">Classe</label>
              <select
                v-model="nouvelAsset.classe"
                class="w-full mt-1 bg-white border border-white/20 rounded-lg px-3 py-2 text-sm text-black"
                @change="majWorkerEtSymboles()"
              >
                <option value="crypto">🪙 Crypto</option>
                <option value="metal">🥇 Métal</option>
                <option value="forex">💱 Forex</option>
                <option value="indice">📈 Indice</option>
              </select>
            </div>
            <div>
              <label class="text-xs text-white">Source</label>
              <div class="mt-1 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm text-violet-300">
                MT5 / Axi (broker)
              </div>
            </div>
          </div>
          <div>
            <label class="text-xs text-white">Symbole MT5 (broker Axi) — requis</label>
            <input
              v-model="nouvelAsset.symbolMt5"
              placeholder="ex : UK100, dax40.fs, XAUUSD, BTCUSD, SOL-USD"
              class="w-full mt-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm font-mono text-white focus:border-emerald-500/50 outline-none"
              @input="nouvelAsset.symbolMt5 = nouvelAsset.symbolMt5.toUpperCase()"
            />
            <p class="text-[11px] text-white mt-1">Résolu insensible à la casse dans le Market Watch de ton MT5 — l'EA collecte dès qu'il tourne. Crypto : NE PAS deviner (SOL-USD, LNKUSD, DOGUSD…) — recopie le Market Watch.</p>
          </div>
        </div>

        <p v-if="erreurModaleAsset" class="text-sm text-red-400">{{ erreurModaleAsset }}</p>
        <p v-if="succesModaleAsset" class="text-sm text-emerald-400">{{ succesModaleAsset }}</p>

        <p class="text-[11px] text-white">
          L'asset est ajouté <b>actif</b> sur MT5/Axi, la seule source du pipeline (bascule 24/10) :
          l'EA collecte ses bougies dès qu'il tourne. Disponible partout, ARMÉ NULLE PART (doctrine 15/10).
        </p>

        <div class="flex justify-end gap-2 pt-1">
          <button
            class="px-4 py-2 rounded-lg bg-white/5 text-white text-sm hover:bg-white/10 transition"
            @click="fermerModaleAsset()"
          >
            Annuler
          </button>
          <button
            class="px-4 py-2 rounded-lg bg-emerald-500/20 text-emerald-400 text-sm font-semibold hover:bg-emerald-500/30 transition disabled:opacity-40"
            :disabled="enAjoutAsset"
            @click="creerAsset()"
          >
            {{ enAjoutAsset ? '⏳ Ajout…' : "Créer l'asset" }}
          </button>
        </div>
      </div>
    </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { apiService } from '@/services/api.service'
import type { AssetInfo } from '@/services/api.service'
import { useAssetsStore } from '@/stores/assets.store'

const emit = defineEmits<{ (e: 'cree'): void }>()
const assetsStore = useAssetsStore()

// ── Modale d'ajout d'asset ─────────────────────────────────────────────────────
const modaleAsset = ref(false)
const enAjoutAsset = ref(false)
const erreurModaleAsset = ref('')
const succesModaleAsset = ref('')
const nouvelAsset = ref({
  ticker: '',
  nom: '',
  classe: 'crypto' as 'crypto' | 'metal' | 'forex' | 'indice',
  symbolMt5: '',
})

/// Auto-proposition du symbole MT5 quand la classe ou le ticker change :
/// métaux/forex/indices → ticker tel quel (résolu insensible à la casse) ;
/// crypto → JAMAIS deviné (SOL-USD, LNKUSD, DOGUSD… pièges réels — doctrine
/// « table en dur, jamais d'appariement automatique ») : champ laissé vide.
function majWorkerEtSymboles() {
  const t = nouvelAsset.value.ticker.trim()
  nouvelAsset.value.symbolMt5 = nouvelAsset.value.classe === 'crypto' ? '' : t
}

function ouvrirModaleAsset() {
  nouvelAsset.value = { ticker: '', nom: '', classe: 'crypto', symbolMt5: '' }
  erreurModaleAsset.value = ''
  succesModaleAsset.value = ''
  modaleAsset.value = true
}

function fermerModaleAsset() {
  modaleAsset.value = false
}

async function creerAsset() {
  const a = nouvelAsset.value
  erreurModaleAsset.value = ''
  if (a.ticker.trim().length < 2) {
    erreurModaleAsset.value = 'Le ticker doit faire au moins 2 caractères.'
    return
  }
  if (!a.nom.trim()) {
    erreurModaleAsset.value = 'Le nom est requis.'
    return
  }
  if (!a.symbolMt5.trim()) {
    erreurModaleAsset.value = 'Le symbole MT5/Axi est requis (recopie le Market Watch).'
    return
  }
  enAjoutAsset.value = true
  try {
    await apiService.ajouterAsset(
      a.ticker.trim(),
      a.nom.trim(),
      a.classe,
      'mt5',
      undefined,
      a.symbolMt5.trim(),
    )
    succesModaleAsset.value = `✅ ${a.ticker} ajouté — l'EA le prend en charge dès son prochain cycle (Market Watch + push M1).`
    await assetsStore.chargerAssets()
    setTimeout(() => { modaleAsset.value = false }, 1200)
  } catch (e: unknown) {
    erreurModaleAsset.value = (e as Error).message ?? 'Erreur inconnue'
  } finally {
    enAjoutAsset.value = false
  }
}


defineExpose({ ouvrirModaleAsset })
</script>
<style scoped>
</style>
