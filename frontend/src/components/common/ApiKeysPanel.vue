<template>
  <div class="space-y-4">

    <!-- IA Vision (Anthropic) -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h2 class="text-xs uppercase font-bold text-white">IA Vision — Anthropic</h2>
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
          <h2 class="text-xs uppercase font-bold text-white">Notifications — Telegram</h2>
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

    <!-- Veille Actions — Tiingo (scanner Rockets actions US) -->
    <div class="glass-card p-4">
      <div class="flex items-center justify-between mb-3">
        <div>
          <h2 class="text-xs uppercase font-bold text-white">Veille Actions — Tiingo</h2>
          <p class="text-xs text-gray-500 mt-0.5">Prix quotidiens des actions US (volume réel) pour le scanner Rockets — 1 000 requêtes/jour offertes</p>
        </div>
        <div class="flex items-center gap-2">
          <button class="px-3 py-1.5 bg-amber-600 hover:bg-amber-500 rounded text-xs font-medium transition-colors" @click="sauvegarderTiingo">Enregistrer</button>
          <span v-if="tiingoSauvegarde" class="text-emerald-400 text-xs">✓</span>
          <span v-if="tiingoErreur" class="text-red-400 text-xs">⚠️ Erreur</span>
        </div>
      </div>
      <div class="flex gap-3 items-center">
        <input v-model="tiingoKey" :type="afficherTiingo ? 'text' : 'password'" placeholder="Clé Tiingo..."
          autocomplete="off"
          class="bg-gray-700 text-white rounded px-2 py-1.5 w-80 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-amber-500"
          @keyup.enter="sauvegarderTiingo" />
        <button class="text-xs text-gray-400 hover:text-white transition-colors" @click="afficherTiingo = !afficherTiingo">
          {{ afficherTiingo ? '🙈 Masquer' : '👁️ Afficher' }}
        </button>
        <span class="text-xs text-gray-500">Obtenir une clé gratuite : tiingo.com</span>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { apiService } from '@/services/api.service'

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

// ── Tiingo (veille actions Rockets) ─────────────────────────────────────────
const tiingoKey = ref('')
const tiingoSauvegarde = ref(false)
const tiingoErreur = ref(false)
const afficherTiingo = ref(false)

const timers: ReturnType<typeof setTimeout>[] = []
onUnmounted(() => timers.forEach(clearTimeout))

onMounted(async () => {
  try {
    const [key, tok, chatId, tiingo] = await Promise.all([
      apiService.obtenirConfig('anthropic_api_key'),
      apiService.obtenirConfig('telegram_bot_token'),
      apiService.obtenirConfig('telegram_chat_id'),
      apiService.obtenirConfig('tiingo_api_key'),
    ])
    if (key?.valeur) anthropicKey.value = key.valeur
    if (tok?.valeur) telegramToken.value = tok.valeur
    if (chatId?.valeur) telegramChatId.value = chatId.valeur
    if (tiingo?.valeur) tiingoKey.value = tiingo.valeur
  } catch {
    // Backend non disponible — valeurs par défaut
  }
})

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

async function sauvegarderTiingo() {
  const cle = tiingoKey.value.trim()
  if (cle.length < 10 && cle !== '') {
    tiingoErreur.value = true
    timers.push(setTimeout(() => { tiingoErreur.value = false }, 3000))
    return
  }
  try {
    await apiService.sauvegarderConfig('tiingo_api_key', cle)
    tiingoSauvegarde.value = true
    tiingoErreur.value = false
    timers.push(setTimeout(() => { tiingoSauvegarde.value = false }, 2000))
  } catch {
    tiingoErreur.value = true
    timers.push(setTimeout(() => { tiingoErreur.value = false }, 3000))
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
