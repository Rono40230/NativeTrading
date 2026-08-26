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
          <button class="text-gray-400 hover:text-white transition" @click="fermerModaleAsset()">✕</button>
        </div>

        <div class="space-y-3">
          <div>
            <label class="text-xs text-gray-400">Ticker</label>
            <input
              v-model="nouvelAsset.ticker"
              placeholder="ex : TON, GBPAUD, HK50"
              class="w-full mt-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white uppercase focus:border-emerald-500/50 outline-none"
              @input="nouvelAsset.ticker = nouvelAsset.ticker.toUpperCase(); majWorkerEtSymboles()"
            />
          </div>
          <div>
            <label class="text-xs text-gray-400">Nom</label>
            <input
              v-model="nouvelAsset.nom"
              placeholder="ex : Toncoin"
              class="w-full mt-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:border-emerald-500/50 outline-none"
            />
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="text-xs text-gray-400">Classe</label>
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
              <label class="text-xs text-gray-400">Worker</label>
              <div class="mt-1 px-3 py-2 rounded-lg bg-white/5 border border-white/10 text-sm"
                   :class="sourceWorker === 'binance' ? 'text-yellow-300' : 'text-violet-300'">
                {{ sourceWorker === 'binance' ? 'Bybit (temps réel)' : 'MT5 / Axi (broker)' }}
              </div>
            </div>
          </div>
          <div v-if="sourceWorker === 'binance'">
            <label class="text-xs text-gray-400">Symbole Bybit</label>
            <input
              v-model="nouvelAsset.symbolBybit"
              placeholder="TONUSDT"
              class="w-full mt-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm font-mono text-white focus:border-emerald-500/50 outline-none"
              @input="nouvelAsset.symbolBybit = nouvelAsset.symbolBybit.toUpperCase()"
            />
          </div>
          <div v-else>
            <label class="text-xs text-gray-400">Symbole MT5 (broker Axi)</label>
            <input
              v-model="nouvelAsset.symbolMt5"
              placeholder="ex : UK100, dax40.fs, XAUUSD"
              class="w-full mt-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm font-mono text-white focus:border-emerald-500/50 outline-none"
              @input="nouvelAsset.symbolMt5 = nouvelAsset.symbolMt5.toUpperCase()"
            />
            <p class="text-[11px] text-gray-500 mt-1">Résolu insensible à la casse dans le Market Watch de ton MT5 — l'EA collecte dès qu'il tourne.</p>
          </div>
        </div>

        <p v-if="erreurModaleAsset" class="text-sm text-red-400">{{ erreurModaleAsset }}</p>
        <p v-if="succesModaleAsset" class="text-sm text-emerald-400">{{ succesModaleAsset }}</p>

        <p class="text-[11px] text-gray-500">
          L'asset est ajouté <b>actif</b> : Bybit le prend en charge en ≤ 60 s (souscription + moteur v12 armé) ;
          MT5 dès que ton EA tourne ( Market Watch + push M1).
        </p>

        <div class="flex justify-end gap-2 pt-1">
          <button
            class="px-4 py-2 rounded-lg bg-white/5 text-gray-300 text-sm hover:bg-white/10 transition"
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
  symbolBybit: '',
})

/// Règle famille → worker (actée 26/08) : crypto = Bybit temps réel ; métaux,
/// forex et indices = MT5 / Axi (broker). Dukascopy n'est plus utilisé.
const sourceWorker = computed(() =>
  nouvelAsset.value.classe === 'crypto' ? 'binance' : 'mt5',
)

/// Auto-proposition des symboles quand la classe ou le ticker change :
/// crypto → TICKERUSDT (Bybit) ; autres familles → ticker tel quel (symbole
/// MT5, résolu insensible à la casse par l'EA — formes .fs ajustables).
function majWorkerEtSymboles() {
  const t = nouvelAsset.value.ticker.trim()
  if (sourceWorker.value === 'binance') {
    const base = t.endsWith('USD') && t.length > 3 ? t.slice(0, -3) : t
    nouvelAsset.value.symbolBybit = base ? `${base}USDT` : ''
  } else {
    nouvelAsset.value.symbolMt5 = t
  }
}

function ouvrirModaleAsset() {
  nouvelAsset.value = { ticker: '', nom: '', classe: 'crypto', symbolBybit: '', symbolMt5: '' }
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
  enAjoutAsset.value = true
  try {
    await apiService.ajouterAsset(
      a.ticker.trim(),
      a.nom.trim(),
      a.classe,
      sourceWorker.value as 'binance' | 'mt5',
      sourceWorker.value === 'binance' ? a.symbolBybit.trim() : undefined,
      undefined,
      sourceWorker.value === 'mt5' ? a.symbolMt5.trim() : undefined,
    )
    succesModaleAsset.value = `✅ ${a.ticker} ajouté — prise en charge par le pipeline en ≤ 60 s.`
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
