/**
 * useAlertesPrix — alertes de prix sur le graphique.
 *
 * - Lignes horizontales (priceLines) sur la série pour chaque alerte ACTIVE
 *   de l'asset courant : ambre en pointillés + 🔔 et le prix sur l'échelle.
 * - Mode pose : activé, le prochain CLIC sur le chart crée l'alerte au prix
 *   du curseur (sens auto : au-dessus si le clic est au-dessus du prix live).
 * - Poll 10 s : détection des déclenchées → notification OS + son, puis
 *   SUPPRESSION — une alerte atteinte n'apparaît plus nulle part.
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
    // Seules les ACTIVES sont dessinées — une alerte déclenchée n'apparaît
    // plus nulle part (décision propriétaire) : pas de ligne grise.
    const actives = alertesAsset.value.filter(a => a.active)
    const ids = new Set(actives.map(a => a.id))
    for (const [id, ligne] of lignes) {
      if (!ids.has(id)) {
        try { serie.removePriceLine(ligne) } catch { /* série détruite */ }
        lignes.delete(id)
      }
    }
    for (const a of actives) {
      // Pas de prix dans le libellé : l'étiquette de l'axe l'affiche déjà
      // (sinon doublon côte à côte).
      const options = {
        price: a.prix,
        color: '#f59e0b',
        lineWidth: 1 as 1 | 2 | 3 | 4,
        lineStyle: LineStyle.Dashed,
        axisLabelVisible: true,
        title: `🔔 ${a.sens === 'au_dessus' ? '↑' : '↓'}`,
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
    let liste: AlertePrix[]
    try {
      liste = await alertesApi.lister()
    } catch { return }
    alertes.value = liste
    // Déclenchée détectée → notification (son + OS, si fraîche et sur l'asset
    // du chart) PUIS SUPPRESSION : une alerte atteinte disparaît de partout
    // (graphique, bloc dashboard, base). Fraîche < 60 s : pas de son en
    // retard à l'ouverture d'un chart sur une vieille alerte.
    const aSupprimer: number[] = []
    for (const a of liste) {
      if (!a.active && a.declenchee_le && !dejaNotifiees.has(a.id)) {
        dejaNotifiees.add(a.id)
        aSupprimer.push(a.id)
        if (a.asset === asset && Date.now() / 1000 - a.declenchee_le < 60) {
          const sens = a.sens === 'au_dessus' ? 'est monté à' : 'est descendu à'
          void notifier(`🔔 Alerte prix — ${a.asset}`, `${a.asset} ${sens} ${a.prix.toFixed(2)}`, { son: true, urgence: 'normal' })
        }
      }
    }
    if (aSupprimer.length) {
      await Promise.all(aSupprimer.map(id => alertesApi.supprimer(id).catch(() => null)))
      alertes.value = alertes.value.filter(a => !aSupprimer.includes(a.id))
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
    initialiser, definirAsset, detruire, basculerModePose, supprimer, recharger,
  }
}
