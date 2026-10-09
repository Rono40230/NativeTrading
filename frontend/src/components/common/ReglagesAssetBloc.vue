<template>
  <!-- Réglages PAR ASSET (spec 0121) : surcharge optionnelle d'un asset,
       champ vide = repli sur le défaut global (affiché en placeholder grisé).
       Hot-reload ≤ 60 s sur les seuls couples de l'asset. -->
  <section class="reglages-asset">
    <header class="flex items-center gap-3 flex-wrap mb-3">
      <h4 class="font-bold text-sm uppercase tracking-wider text-white/70">🎚️ Réglages par asset</h4>
      <select v-model="assetChoisi" class="sel">
        <option v-for="a in assets" :key="a" :value="a">{{ a }}</option>
      </select>
      <span v-if="surchargeActive" class="badge-actif">surchargé</span>
      <span class="text-xs text-white/40">vide = défaut global · hot-reload ≤ 60 s (cet asset seul)</span>
    </header>

    <p v-if="chargement" class="text-xs text-white/40">Chargement…</p>
    <p v-else-if="erreur" class="erreur">{{ erreur }}</p>

    <template v-else>
      <div class="grille-champs">
        <label v-for="c in champs" :key="c.cle" class="champ">
          <span class="libelle">{{ c.libelle }}</span>
          <select
            v-if="c.type === 'mode'"
            v-model="form[c.cle]"
            class="input"
          >
            <option :value="null">— défaut ({{ defautTxt(c.cle) }}) —</option>
            <option value="lointaine">lointaine (liquidités)</option>
            <option value="fixe">R fixe</option>
          </select>
          <input
            v-else
            v-model="form[c.cle]"
            class="input"
            :type="c.entier ? 'number' : 'text'"
            :placeholder="defautTxt(c.cle)"
            inputmode="decimal"
            @input="c.entier && form[c.cle] !== null && (form[c.cle] = form[c.cle]?.toString().replace(/[^\d-]/g, '') ?? null)"
          />
          <span v-if="c.aide" class="aide">{{ c.aide }}</span>
        </label>
      </div>

      <!-- Somme des fractions en direct (SMC uniquement, décision ③). -->
      <p v-if="strategie === 'SMC'" class="somme" :class="{ ko: sommeFractions !== null && Math.abs(sommeFractions - 1) > 0.001 }">
        Somme des fractions :
        <b v-if="sommeFractions === null">(défaut global)</b>
        <b v-else>{{ sommeFractions.toFixed(3) }}</b>
        <span v-if="sommeFractions !== null && Math.abs(sommeFractions - 1) > 0.001" class="ko">— doit valoir 1,000 (les 3 champs ensemble)</span>
      </p>

      <footer class="flex gap-3 mt-3 flex-wrap">
        <button class="btn primaire" :disabled="!peutEnregistrer" @click="enregistrer">💾 Enregistrer la surcharge</button>
        <button class="btn" :disabled="!surchargeActive" @click="reinitialiser">↩️ Réinitialiser sur défaut</button>
        <span v-if="message" class="text-xs self-center" :class="messageKo ? 'ko' : 'ok'">{{ message }}</span>
      </footer>
    </template>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { apiService } from '@/services/api.service'

const props = defineProps<{ strategie: 'SMC' | 'straddle' | 'kdj_halftrend' }>()

interface Champ {
  cle: string
  libelle: string
  type?: 'mode'
  entier?: boolean
  aide?: string
}

const CHAMPS: Record<string, Champ[]> = {
  SMC: [
    { cle: 'sl_max', libelle: 'SL max (× ATR)', aide: 'multiple d\'ATR — remplace la classe de l\'asset' },
    { cle: 'trailing_r', libelle: 'Trailing k (R)' },
    { cle: 'tp1', libelle: 'TP1 (R)' },
    { cle: 'tp2', libelle: 'TP2 (R)' },
    { cle: 'tp3_mode', libelle: 'TP3 mode', type: 'mode' },
    { cle: 'tp3_rfixe', libelle: 'TP3 R fixe' },
    { cle: 'frac_tp1', libelle: 'Fraction TP1' },
    { cle: 'frac_tp2', libelle: 'Fraction TP2' },
    { cle: 'frac_tp3', libelle: 'Fraction TP3' },
  ],
  straddle: [
    { cle: 'sl_mult', libelle: 'SL (× ATR H1)' },
    { cle: 'trailing_r', libelle: 'Trailing (R)' },
    { cle: 'placement_sec', libelle: 'Placement T− (s)', entier: true },
  ],
  kdj_halftrend: [
    { cle: 'period', libelle: 'Period', entier: true },
    { cle: 'signal', libelle: 'Signal', entier: true },
    { cle: 'amplitude', libelle: 'Amplitude', entier: true },
    { cle: 'ratio_risk', libelle: 'Ratio risque' },
    { cle: 'adx_min', libelle: 'ADX min (< 0 = off)' },
  ],
}

const assets = ref<string[]>([])
const assetChoisi = ref('')
const chargement = ref(true)
const erreur = ref('')
const surchargeActive = ref(false)
const message = ref('')
const messageKo = ref(false)
const defauts = ref<Record<string, unknown>>({})
// form : null = vide (repli défaut) ; sinon chaîne (saisie libre, convertie).
const form = reactive<Record<string, string | null>>({})

const champs = computed(() => CHAMPS[props.strategie] ?? [])

function viderForm() {
  for (const c of champs.value) form[c.cle] = null
}

function defautTxt(cle: string): string {
  const d = defauts.value
  if (cle === 'tp3_mode') return d.tp3_mode == null ? '—' : String(d.tp3_mode)
  const v = (d as Record<string, unknown>)[cle]
  return v === null || v === undefined ? '—' : String(v)
}

const sommeFractions = computed<number | null>(() => {
  const v = ['frac_tp1', 'frac_tp2', 'frac_tp3'].map((c) => form[c])
  if (v.every((x) => x === null || x === '')) return null
  const n = v.map((x) => parseFloat(x ?? ''))
  return n.some((x) => Number.isNaN(x)) ? NaN : n[0] + n[1] + n[2]
})

const peutEnregistrer = computed(() => {
  if (props.strategie === 'SMC' && sommeFractions.value !== null) {
    if (Number.isNaN(sommeFractions.value) || Math.abs(sommeFractions.value - 1) > 0.001) return false
  }
  return true
})

async function charger() {
  chargement.value = true
  erreur.value = ''
  message.value = ''
  try {
    if (!assets.value.length) {
      const tous = await apiService.obtenirAssets()
      assets.value = tous
        .filter((a: { actif?: boolean }) => a.actif !== false)
        .map((a: { id: string }) => a.id)
        .sort()
      if (!assetChoisi.value && assets.value.length) assetChoisi.value = assets.value[0]
    }
    const res = await fetch(
      `/api/strategies/${props.strategie}/reglages-asset/${encodeURIComponent(assetChoisi.value)}`,
    )
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const data = await res.json()
    defauts.value = data.defaut ?? {}
    // fractions SMC : le défaut global arrive en objet {tp1,tp2,tp3} → aplatir.
    if (props.strategie === 'SMC' && defauts.value.frac && typeof defauts.value.frac === 'object') {
      const f = defauts.value.frac as Record<string, number>
      defauts.value.frac_tp1 = f.tp1
      defauts.value.frac_tp2 = f.tp2
      defauts.value.frac_tp3 = f.tp3
    }
    viderForm()
    const s = data.surcharge ?? {}
    surchargeActive.value = Object.values(s).some((v) => v !== null)
    for (const c of champs.value) {
      const v = s[c.cle]
      form[c.cle] = v === null || v === undefined ? null : String(v)
    }
  } catch (e) {
    erreur.value = `Chargement impossible : ${e}`
  } finally {
    chargement.value = false
  }
}

function corpsPut(): Record<string, unknown> {
  const corps: Record<string, unknown> = {}
  for (const c of champs.value) {
    const brut = form[c.cle]
    if (brut === null || brut === '') {
      corps[c.cle] = null
      continue
    }
    if (c.type === 'mode') {
      corps[c.cle] = brut
      continue
    }
    const n = c.entier ? parseInt(brut, 10) : parseFloat(brut.replace(',', '.'))
    corps[c.cle] = Number.isNaN(n) ? null : n
  }
  return corps
}

async function enregistrer() {
  message.value = ''
  try {
    const res = await fetch(
      `/api/strategies/${props.strategie}/reglages-asset/${encodeURIComponent(assetChoisi.value)}`,
      {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(corpsPut()),
      },
    )
    const data = await res.json().catch(() => ({}))
    if (!res.ok) throw new Error(data.error ?? `HTTP ${res.status}`)
    messageKo.value = false
    message.value = `Surchargé — hot-reload ≤ 60 s (${assetChoisi.value} seul)`
    surchargeActive.value = true
  } catch (e) {
    messageKo.value = true
    message.value = `Échec : ${e}`
  }
}

async function reinitialiser() {
  message.value = ''
  try {
    const res = await fetch(
      `/api/strategies/${props.strategie}/reglages-asset/${encodeURIComponent(assetChoisi.value)}`,
      { method: 'DELETE' },
    )
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    viderForm()
    surchargeActive.value = false
    messageKo.value = false
    message.value = `${assetChoisi.value} repassé au défaut global`
  } catch (e) {
    messageKo.value = true
    message.value = `Échec : ${e}`
  }
}

onMounted(charger)
watch(assetChoisi, charger)
watch(() => props.strategie, charger)
</script>

<style scoped>
.reglages-asset { border-top: 1px solid rgba(255,255,255,.1); padding-top: 12px; margin-top: 12px; }
.sel, .input {
  background: rgba(0,0,0,.35); border: 1px solid rgba(255,255,255,.15); color: #e6edf3;
  border-radius: 8px; padding: 6px 10px; font-size: .85rem;
}
.sel:focus, .input:focus { outline: none; border-color: #60a5fa; }
.badge-actif {
  font-size: .65rem; font-weight: 700; padding: 2px 8px; border-radius: 999px;
  background: rgba(251,191,36,.15); color: #fbbf24; border: 1px solid rgba(251,191,36,.3);
}
.grille-champs { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 10px; }
.champ { display: flex; flex-direction: column; gap: 4px; }
.libelle { font-size: .72rem; text-transform: uppercase; letter-spacing: .05em; color: #8b949e; }
.aide { font-size: .65rem; color: #6e7681; }
.input::placeholder { color: rgba(139,148,158,.55); }
.somme { font-size: .78rem; color: #8b949e; margin-top: 10px; }
.ko { color: #f87171; }
.ok { color: #34d399; }
.btn {
  font-size: .8rem; font-weight: 600; padding: 7px 14px; border-radius: 8px;
  border: 1px solid rgba(255,255,255,.2); background: rgba(255,255,255,.06); color: #e6edf3; cursor: pointer;
}
.btn:disabled { opacity: .4; cursor: not-allowed; }
.btn.primaire { background: rgba(63,133,255,.2); border-color: rgba(63,133,255,.4); }
.erreur { color: #f87171; font-size: .8rem; }
</style>
