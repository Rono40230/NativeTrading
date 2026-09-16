/// R latent TOTAL des trades en cours (15/09) — colonnes « R Latent » et
/// « P/L Latent » du tableau des signaux. Le backend sert les ingrédients
/// (part fixe : paliers SMC vendus au toucher, jambe opposée du straddle ;
/// coefficient du prix) ; ce composable compose au prix WS vivant :
/// r = r_fixe + coef_prix × prix. Rafraîchi au cycle 60 s.

import { onMounted, onUnmounted, ref } from 'vue'
import { http } from '@/services/http.client'
import { usePrixStore } from '@/stores/prix.store'

interface LatentApi {
  id: string
  r_fixe: number
  coef_prix: number
  prix_entree: number
}

export function useLatentsSignaux() {
  const prixStore = usePrixStore()
  const latents = ref<Map<string, LatentApi>>(new Map())

  async function charger() {
    try {
      const res = await http.get<{ latents: LatentApi[] }>('/api/signaux/latents')
      latents.value = new Map(res.data.latents.map(l => [l.id, l]))
    } catch { /* silencieux — la map garde les derniers ingrédients */ }
  }

  /// R latent total : r = r_fixe + coef × ÉCART au prix d'entrée (15/09,
  /// correctif : la première version multipliait le coef par le prix
  /// absolu — R délirant sur les actifs à prix élevé).
  function rLatent(id: string, asset: string): number | null {
    const l = latents.value.get(id)
    const prix = prixStore.getPrix(asset)
    if (!l || prix === null) return null
    return l.r_fixe + l.coef_prix * (prix - l.prix_entree)
  }

  let minuteur: ReturnType<typeof setInterval> | null = null
  onMounted(() => {
    void charger()
    minuteur = setInterval(charger, 60_000)
  })
  onUnmounted(() => { if (minuteur !== null) clearInterval(minuteur) })

  return { rLatent }
}
