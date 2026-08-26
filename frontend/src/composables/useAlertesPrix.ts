/**
 * useAlertesPrix — alertes de prix sur le graphique.
 *
 * - Lignes horizontales (priceLines) sur la série pour chaque alerte de
 *   l'asset courant : ambre en pointillés + 🔔 et le prix sur l'échelle,
 *   grisée une fois déclenchée.
 * - Mode pose : activé, le prochain CLIC sur le chart crée l'alerte au prix
 *   du curseur (sens auto : au-dessus si le clic est au-dessus du prix live).
 * - Poll 10 s : détection des déclenchées → notification OS + son.
 */
import type { IChartApi, ISeriesApi, IPriceLine } from 'lightweight-charts'
import { LineStyle } from 'lightweight-charts'
import { ref, computed } from 'vue'
import { alertesApi, type AlertePrix } from '@/services/api.alertes'
import { useNotification } from '@/composables/useNotification'

export function useAlertesPrix() {
  const { notifier } = useNotification()

  const alertes = ref<AlertePrix[]>([])
  /** Mode pose d'alerte : le prochain clic sur le chart la crée. */
  const modePose = ref(false)

  let chart: IChartApi | null = null
  let serie: ISeriesApi<'Candlestick'> | null = null
  let asset = ''
  let lignes: Map<number, IPriceLine> = new Map()
  let poll: ReturnType<typeof setInterval> | null = null
  /// Derniers ids déjà notifiés (évite le re-son au rechargement).
  let dejaNotifiees = new Set<number>()

  const alertesAsset = computed(() => alertes.value.filter(a => a.asset === asset))
  const nbActives = computed(() => alertesAsset.value.filter(a => a.active).length)

  // ── Lignes sur le chart ─────────────────────────────────────────────────────
  function synchroniserLignes() {
    if (!serie) return
    // Retirer les lignes disparues / modifier les existantes.
    const ids = new Set(alertesAsset.value.map(a => a.id))
    for (const [id, ligne] of lignes) {
      if (!ids.has(id)) {
        try { serie.removePriceLine(ligne) } catch { /* série détruite */ }
        lignes.delete(id)
      }
    }
    for (const a of alertesAsset.value) {
      // Pas de prix dans le libellé : l'étiquette de l'axe l'affiche déjà
      // (sinon doublon côte à côte).
      const titre = a.active
        ? `🔔 ${a.sens === 'au_dessus' ? '↑' : '↓'}`
        : '✓'
      const options = {
        price: a.prix,
        color: a.active ? '#f59e0b' : '#64748b',
        lineWidth: 1 as 1 | 2 | 3 | 4,
        lineStyle: LineStyle.Dashed,
        axisLabelVisible: true,
        title: titre,
      }
      const existante = lignes.get(a.id)
      if (existante) {
        try { existante.applyOptions(options) } catch { /* ignoré */ }
      } else {
        try { lignes.set(a.id, serie.createPriceLine(options)) } catch { /* ignoré */ }
      }
    }
  }

  // ── Poll + notifications ────────────────────────────────────────────────────
  async function recharger() {
    try {
      alertes.value = await alertesApi.lister()
    } catch { return }
    // Nouvelles déclenchées → notification + son.
    for (const a of alertes.value) {
      if (!a.active && a.declenchee_le && !dejaNotifiees.has(a.id)) {
        dejaNotifiees.add(a.id)
        if (a.asset === asset) {
          const sens = a.sens === 'au_dessus' ? 'est monté à' : 'est descendu à'
          void notifier(`🔔 Alerte prix — ${a.asset}`, `${a.asset} ${sens} ${a.prix.toFixed(2)}`, { son: true, urgence: 'normal' })
        }
      } else if (a.active) {
        dejaNotifiees.delete(a.id) // réarmée → re-notifiable
      }
    }
    synchroniserLignes()
  }

  // ── Pose au clic ────────────────────────────────────────────────────────────
  function surClic(param: Parameters<Parameters<IChartApi['subscribeClick']>[0]>[0]) {
    if (!modePose.value || !serie || !param.point) return
    const prix = serie.coordinateToPrice(param.point.y)
    if (prix === null || !isFinite(prix)) return
    // Sens auto : franchissement vers le niveau cliqué depuis le prix actuel.
    const dernier = (param.seriesData.get(serie) as { close?: number } | undefined)?.close
    const sens: 'au_dessus' | 'en_dessous' =
      dernier !== undefined && prix < (dernier as number) ? 'en_dessous' : 'au_dessus'
    modePose.value = false
    void alertesApi.creer(asset, prix as number, sens).then(() => recharger())
  }

  function basculerModePose() {
    modePose.value = !modePose.value
  }

  async function supprimer(id: number) {
    await alertesApi.supprimer(id)
    await recharger()
  }

  async function rearmer(id: number) {
    await alertesApi.rearmer(id)
    await recharger()
  }

  // ── Cycle de vie ────────────────────────────────────────────────────────────
  function initialiser(c: IChartApi, s: ISeriesApi<'Candlestick'>, assetCourant: string) {
    detruire()
    chart = c
    serie = s
    asset = assetCourant
    dejaNotifiees = new Set()
    chart.subscribeClick(surClic)
    void recharger()
    poll = setInterval(() => void recharger(), 10_000)
  }

  function definirAsset(a: string) {
    if (a === asset) return
    asset = a
    // Les lignes de l'ancien asset partent, celles du nouveau arrivent.
    if (serie) { for (const l of lignes.values()) { try { serie.removePriceLine(l) } catch { /* */ } } }
    lignes.clear()
    dejaNotifiees = new Set()
    void recharger()
  }

  function detruire() {
    if (poll !== null) { clearInterval(poll); poll = null }
    if (chart) { try { chart.unsubscribeClick(surClic) } catch { /* */ } }
    if (serie) { for (const l of lignes.values()) { try { serie.removePriceLine(l) } catch { /* */ } } }
    lignes.clear()
    chart = null
    serie = null
  }

  return {
    alertes, alertesAsset, nbActives, modePose,
    initialiser, definirAsset, detruire, basculerModePose, supprimer, rearmer, recharger,
  }
}
