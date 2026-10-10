/// État d'une stratégie (Officielle / Observation) — source unique : le
/// registre /api/strategies. Écart 11 (owner 10/10) : les pages SMC et
/// straddle le HARCODAIENT, la page rockets affichait « Observation » à
/// tort — les cartes dashboard et les pages se contredisaient. Un seul
/// fetch partagé (cache module), repli « Observation » si indisponible.
import { computed, ref } from 'vue'
import { http } from '@/services/http.client'

const etats = ref<Record<string, string>>({})
let lance = false

function charger() {
  if (lance) return
  lance = true
  void http
    .get<{ id: string; etat: string }[]>('/api/strategies')
    .then(r => {
      const carte: Record<string, string> = {}
      for (const s of r.data) carte[s.id] = s.etat
      etats.value = carte
    })
    .catch(() => { /* registre indisponible — repli Observation */ })
}

export function useEtatStrategie(id: string) {
  charger()
  const etat = computed(() => etats.value[id] ?? 'Observation')
  return { etat }
}
