<template>
  <ModaleCadre titre="🔔 Gérer les alertes" tres-large @fermer="$emit('fermer')">
    <div class="text-sm text-white/80">
      <table class="w-full">
        <thead>
          <tr class="text-xs uppercase text-white/50">
            <th class="text-left pb-2">Famille</th>
            <th class="pb-2" title="Notification OS + son sur le bureau">Bureau</th>
            <th class="pb-2">Telegram</th>
            <th class="pb-2" title="Notifications du téléphone (muet 21h→8h)">Mobile</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="s in sections" :key="s.titre">
          <tr>
            <td :colspan="4" class="pt-4 pb-1 text-xs font-semibold tracking-wider text-amber-400/90 uppercase">{{ s.titre }}</td>
          </tr>
          <tr v-for="l in s.lignes" :key="l.famille" class="border-t border-white/5">
            <td class="py-2 pr-3">
              <span class="mr-1.5">{{ l.emoji }}</span>{{ l.libelle }}
            </td>
            <td v-for="c in (l.canaux as string[])" :key="c" class="text-center">
              <button
                class="px-2 py-1 rounded text-xs font-semibold transition-colors"
                :class="etat(l.famille, c) ? 'bg-emerald-600/80 text-white' : 'bg-white/10 text-white/50 hover:bg-white/20'"
                @click="basculer(l.famille, c)"
              >{{ etat(l.famille, c) ? '✓' : '—' }}</button>
            </td>
            <td v-for="c in colonnesManquantes(l)" :key="'vide-' + c"></td>
          </tr>
          </template>
        </tbody>
      </table>
    </div>
  </ModaleCadre>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import ModaleCadre from './ModaleCadre.vue'
import { http } from '@/services/http.client'

interface Canal { famille: string; app: boolean; telegram: boolean; mobile: boolean }

const sections = [
  {
    titre: 'Opportunités',
    lignes: [
      { famille: 'evenement_t10', emoji: '📅', libelle: 'Rappel T-10 min avant annonce High', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'presse_100', emoji: '📰', libelle: 'Gros titre presse marquant', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'smc_approche', emoji: '📍', libelle: 'SMC — approche de zone armée', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'smc_contact', emoji: '🎯', libelle: 'SMC — contact de zone', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'obia_contact', emoji: '🧿', libelle: 'OBIA — contact de zone (espion, H4/D)', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'alerte_prix', emoji: '🔔', libelle: 'Alerte prix déclenchée', canaux: ['app', 'telegram', 'mobile'] },
    ],
  },
  {
    titre: 'Informations',
    lignes: [
      { famille: 'briefing', emoji: '🌅', libelle: 'Briefing du matin (8h30)', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'sante', emoji: '🩸', libelle: 'Santé fournisseurs', canaux: ['app', 'telegram', 'mobile'] },
    ],
  },
  {
    titre: 'Signaux pris (Telegram uniquement)',
    lignes: [
      { famille: 'signaux_smc', emoji: '📐', libelle: 'Signaux SMC pris', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'signaux_straddle', emoji: '⚡', libelle: 'Passes Straddle', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'signaux_rockets', emoji: '🚀', libelle: 'Signaux Rockets', canaux: ['app', 'telegram', 'mobile'] },
      { famille: 'signaux_kdj', emoji: '📊', libelle: 'Signaux KDJ/Halftrend', canaux: ['app', 'telegram', 'mobile'] },
    ],
  },
]

const etatMap = ref<Record<string, Canal>>({})

function etat(famille: string, canal: string): boolean {
  const c = etatMap.value[famille]
  if (!c) return false
  return canal === 'app' ? c.app : canal === 'telegram' ? c.telegram : c.mobile
}

async function basculer(famille: string, canal: string) {
  const c = etatMap.value[famille]
  if (!c) return
  const nouveau = !etat(famille, canal)
  // optimiste + PUT
  ;(c as unknown as Record<string, boolean>)[canal] = nouveau
  try {
    await http.put('/api/alertes-canaux', { famille, canal, actif: nouveau })
  } catch {
    ;(c as unknown as Record<string, boolean>)[canal] = !nouveau
  }
}

function colonnesManquantes(l: { canaux: string[] }) {
  return ['app', 'telegram', 'mobile'].filter(c => !(l.canaux as string[]).includes(c))
}

onMounted(async () => {
  try {
    const r = await http.get('/api/alertes-canaux')
    const m: Record<string, Canal> = {}
    for (const c of r.data as Canal[]) m[c.famille] = c
    etatMap.value = m
  } catch { /* hors ligne */ }
})
</script>
