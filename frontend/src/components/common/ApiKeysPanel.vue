<template>
  <div class="space-y-4">

    <!-- IG Markets -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Données de marché — IG Markets</h2>
          <p class="text-xs text-gray-500 mt-0.5">Forex, métaux, indices — compte démo ou live (AMF réglementé)</p>
        </div>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1.5 bg-emerald-700 hover:bg-emerald-600 rounded text-xs font-medium transition-colors" @click="sauvegarderIG">Enregistrer</button>
          <span v-if="igSauvegarde" class="text-emerald-400 text-xs">✓</span>
          <span v-if="igErreur" class="text-red-400 text-xs">⚠️ Erreur</span>
        </div>
      </div>
      <div class="grid grid-cols-1 gap-3">
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block mb-1 text-xs text-gray-400">API Key</label>
            <div class="flex gap-2 items-center">
              <input v-model="igApiKey" :type="afficherIgKey ? 'text' : 'password'" placeholder="••••••••••••••••"
                autocomplete="off"
                class="bg-gray-700 text-white rounded px-2 py-1.5 w-56 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-emerald-500" />
              <button class="text-xs text-gray-400 hover:text-white transition-colors" @click="afficherIgKey = !afficherIgKey">
                {{ afficherIgKey ? '🙈' : '👁️' }}
              </button>
            </div>
          </div>
          <div>
            <label class="block mb-1 text-xs text-gray-400">Identifiant (login)</label>
            <div class="flex gap-2 items-center">
              <input v-model="igUsername" :type="afficherIgUser ? 'text' : 'password'" placeholder="••••••••"
                autocomplete="off"
                class="bg-gray-700 text-white rounded px-2 py-1.5 w-40 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-emerald-500" />
              <button class="text-xs text-gray-400 hover:text-white transition-colors" @click="afficherIgUser = !afficherIgUser">
                {{ afficherIgUser ? '🙈' : '👁️' }}
              </button>
            </div>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label class="block mb-1 text-xs text-gray-400">Mot de passe</label>
            <div class="flex gap-2 items-center">
              <input v-model="igPassword" :type="afficherIgPass ? 'text' : 'password'" placeholder="••••••••"
                autocomplete="off"
                class="bg-gray-700 text-white rounded px-2 py-1.5 w-40 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-emerald-500" />
              <button class="text-xs text-gray-400 hover:text-white transition-colors" @click="afficherIgPass = !afficherIgPass">
                {{ afficherIgPass ? '🙈' : '👁️' }}
              </button>
            </div>
          </div>
          <div>
            <label class="block mb-1 text-xs text-gray-400">Environnement</label>
            <div class="flex gap-2">
              <button :class="igEnv === 'demo' ? 'bg-emerald-700 text-emerald-200' : 'bg-gray-600 text-gray-300'"
                class="px-3 py-1.5 rounded text-xs font-medium transition-colors"
                @click="igEnv = 'demo'">Démo</button>
              <button :class="igEnv === 'live' ? 'bg-red-700 text-red-200' : 'bg-gray-600 text-gray-300'"
                class="px-3 py-1.5 rounded text-xs font-medium transition-colors"
                @click="igEnv = 'live'">Live</button>
            </div>
          </div>
        </div>
      </div>
      <p class="text-xs text-gray-500 mt-2">Mon Compte IG → Mon Profil → API → Générer une clé</p>
    </div>

    <!-- IA Vision (Anthropic) -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">IA Vision — Anthropic</h2>
          <p class="text-xs text-gray-500 mt-0.5">Utilisée pour l'analyse de charts (Chart Import)</p>
        </div>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1.5 bg-violet-600 hover:bg-violet-500 rounded text-xs font-medium transition-colors" @click="sauvegarderAnthropic">Enregistrer</button>
          <span v-if="anthropicSauvegarde" class="text-emerald-400 text-xs">✓</span>
          <span v-if="anthropicErreur" class="text-red-400 text-xs">⚠️ Clé invalide</span>
        </div>
      </div>
      <div class="flex gap-3 items-center">
        <input v-model="anthropicKey" :type="afficherAnthropic ? 'text' : 'password'" placeholder="sk-ant-..."
          autocomplete="off"
          class="bg-gray-700 text-white rounded px-2 py-1.5 w-80 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-violet-500"
          @keyup.enter="sauvegarderAnthropic" />
        <button class="text-xs text-gray-400 hover:text-white transition-colors" @click="afficherAnthropic = !afficherAnthropic">
          {{ afficherAnthropic ? '🙈 Masquer' : '👁️ Afficher' }}
        </button>
        <span class="text-xs text-gray-500">Obtenir une clé : console.anthropic.com</span>
      </div>
    </div>

    <!-- Telegram -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Notifications — Telegram</h2>
          <p class="text-xs text-gray-500 mt-0.5">Envoi d'un message à chaque nouveau signal validé</p>
        </div>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1.5 bg-sky-600 hover:bg-sky-500 rounded text-xs font-medium transition-colors" @click="sauvegarderTelegram">Enregistrer</button>
          <span v-if="telegramSauvegarde" class="text-emerald-400 text-xs">✓</span>
          <span v-if="telegramErreur" class="text-red-400 text-xs">⚠️ Erreur</span>
        </div>
      </div>
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="block mb-1 text-xs text-gray-400">Bot Token</label>
          <div class="flex gap-2 items-center">
            <input v-model="telegramToken" :type="afficherTelegram ? 'text' : 'password'"
              placeholder="123456:ABCDEF..."
              autocomplete="off"
              class="bg-gray-700 text-white rounded px-2 py-1.5 w-64 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-sky-500" />
            <button class="text-xs text-gray-400 hover:text-white transition-colors" @click="afficherTelegram = !afficherTelegram">
              {{ afficherTelegram ? '🙈 Masquer' : '👁️ Afficher' }}
            </button>
          </div>
        </div>
        <div>
          <label class="block mb-1 text-xs text-gray-400">Chat ID</label>
          <input v-model="telegramChatId" type="text" placeholder="-100123456789"
            class="bg-gray-700 text-white rounded px-2 py-1.5 w-48 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-sky-500"
            @keyup.enter="sauvegarderTelegram" />
        </div>
      </div>
      <p class="text-xs text-gray-500 mt-2">Étape 1 : créez un bot via @BotFather — Étape 2 : récupérez votre Chat ID via @getidsbot</p>
    </div>

    <!-- Twelve Data -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h2 class="text-sm font-semibold text-gray-400 uppercase tracking-wider">Données — Twelve Data</h2>
          <p class="text-xs text-gray-500 mt-0.5">Clé API pour les données forex, métaux et indices (Phase 7)</p>
        </div>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1.5 bg-amber-600 hover:bg-amber-500 rounded text-xs font-medium transition-colors" @click="sauvegarderTwelveData">Enregistrer</button>
          <span v-if="twelveDataSauvegarde" class="text-emerald-400 text-xs">✓</span>
        </div>
      </div>
      <div class="flex gap-3 items-center">
        <input v-model="twelveDataKey" :type="afficherTwelveData ? 'text' : 'password'"
          placeholder="1f192cc..."
          autocomplete="off"
          class="bg-gray-700 text-white rounded px-2 py-1.5 w-80 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-amber-500"
          @keyup.enter="sauvegarderTwelveData" />
        <button class="text-xs text-gray-400 hover:text-white transition-colors" @click="afficherTwelveData = !afficherTwelveData">
          {{ afficherTwelveData ? '🙈 Masquer' : '👁️ Afficher' }}
        </button>
        <span class="text-xs text-gray-500">Obtenir une clé : twelvedata.com</span>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'

// ── IG Markets ────────────────────────────────────────────────────────────────
const igApiKey = ref('')
const igUsername = ref('')
const igPassword = ref('')
const igEnv = ref<'demo' | 'live'>('demo')
const igSauvegarde = ref(false)
const igErreur = ref(false)
const afficherIgKey = ref(false)
const afficherIgUser = ref(false)
const afficherIgPass = ref(false)

// ── Anthropic ─────────────────────────────────────────────────────────────────
const anthropicKey = ref('')
const anthropicSauvegarde = ref(false)
const anthropicErreur = ref(false)
const afficherAnthropic = ref(false)

const telegramToken = ref('')
const telegramChatId = ref('')
const telegramSauvegarde = ref(false)
const telegramErreur = ref(false)
const afficherTelegram = ref(false)

const twelveDataKey = ref('')
const twelveDataSauvegarde = ref(false)
const afficherTwelveData = ref(false)

const timers: ReturnType<typeof setTimeout>[] = []
onUnmounted(() => timers.forEach(clearTimeout))

onMounted(async () => {
  try {
    const [igKey, igUser, igPass, igEnvVal, key, tok, chatId, tdKey] = await Promise.all([
      apiService.obtenirConfig('ig_api_key'),
      apiService.obtenirConfig('ig_username'),
      apiService.obtenirConfig('ig_password'),
      apiService.obtenirConfig('ig_env'),
      apiService.obtenirConfig('anthropic_api_key'),
      apiService.obtenirConfig('telegram_bot_token'),
      apiService.obtenirConfig('telegram_chat_id'),
      apiService.obtenirConfig('twelvedata_api_key'),
    ])
    if (igKey?.valeur) igApiKey.value = igKey.valeur
    if (igUser?.valeur) igUsername.value = igUser.valeur
    if (igPass?.valeur) igPassword.value = igPass.valeur
    if (igEnvVal?.valeur) igEnv.value = igEnvVal.valeur as 'demo' | 'live'
    if (key?.valeur) anthropicKey.value = key.valeur
    if (tok?.valeur) telegramToken.value = tok.valeur
    if (chatId?.valeur) telegramChatId.value = chatId.valeur
    if (tdKey?.valeur) twelveDataKey.value = tdKey.valeur
  } catch {
    // Backend non disponible — valeurs par défaut
  }
})

async function sauvegarderIG() {
  try {
    await Promise.all([
      apiService.sauvegarderConfig('ig_api_key', igApiKey.value.trim()),
      apiService.sauvegarderConfig('ig_username', igUsername.value.trim()),
      apiService.sauvegarderConfig('ig_password', igPassword.value.trim()),
      apiService.sauvegarderConfig('ig_env', igEnv.value),
    ])
    igSauvegarde.value = true
    igErreur.value = false
    timers.push(setTimeout(() => { igSauvegarde.value = false }, 2000))
  } catch {
    igErreur.value = true
    timers.push(setTimeout(() => { igErreur.value = false }, 3000))
  }
}

async function sauvegarderAnthropic() {
  const cle = anthropicKey.value.trim()
  if (!cle.startsWith('sk-ant-') && cle !== '') {
    anthropicErreur.value = true
    timers.push(setTimeout(() => { anthropicErreur.value = false }, 3000))
    return
  }
  try {
    await apiService.sauvegarderConfig('anthropic_api_key', cle)
    anthropicSauvegarde.value = true
    anthropicErreur.value = false
    timers.push(setTimeout(() => { anthropicSauvegarde.value = false }, 2000))
  } catch {
    anthropicErreur.value = true
    timers.push(setTimeout(() => { anthropicErreur.value = false }, 3000))
  }
}

async function sauvegarderTelegram() {
  try {
    await Promise.all([
      apiService.sauvegarderConfig('telegram_bot_token', telegramToken.value.trim()),
      apiService.sauvegarderConfig('telegram_chat_id', telegramChatId.value.trim()),
    ])
    telegramSauvegarde.value = true
    telegramErreur.value = false
    timers.push(setTimeout(() => { telegramSauvegarde.value = false }, 2000))
  } catch {
    telegramErreur.value = true
    timers.push(setTimeout(() => { telegramErreur.value = false }, 3000))
  }
}

async function sauvegarderTwelveData() {
  try {
    await apiService.sauvegarderConfig('twelvedata_api_key', twelveDataKey.value.trim())
    twelveDataSauvegarde.value = true
    timers.push(setTimeout(() => { twelveDataSauvegarde.value = false }, 2000))
  } catch {
    // Erreur silencieuse
  }
}
</script>

<style scoped>
.glass-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 0.75rem;
}
</style>
