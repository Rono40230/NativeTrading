/**
 * useAlertesPrix — alertes de prix sur le graphique.
 *
 * - Lignes horizontales (priceLines) sur la série pour chaque alerte ACTIVE
 *   de l'asset courant : ambre en pointillés + 🔔 et le prix sur l'échelle.
 * - Mode pose : activé, le prochain CLIC sur le chart crée l'alerte au prix
 *   du curseur (sens auto : au-dessus si le clic est au-dessus du prix live).
 * - GLISSER-DÉPOSER (owner 20/10) : attraper une ligne (±5 px) et la
 *   déplacer — la ligne suit en direct, le relâchement enregistre le nouveau
 *   niveau (PUT) avec le sens recalculé face au prix live.
 * - Poll 10 s : détection des déclenchées → SUPPRESSION — une alerte
 *   atteinte n'apparaît plus nulle part (notification = serveur, journal
 *   universel + Telegram matricé).
 */
import type { IChartApi, ISeriesApi, IPriceLine } from 'lightweight-charts'
import { LineStyle } from 'lightweight-charts'
import { ref, computed } from 'vue'
import { storeToRefs } from 'pinia'
import type { AlertePrix } from '@/services/api.alertes'
import { useAlertesPrixStore } from '@/stores/alertesPrix.store'

export function useAlertesPrix() {
  const store = useAlertesPrixStore()
  const { alertes } = storeToRefs(store)

  const modePose = ref(false)

  let chart: IChartApi | null = null
  let serie: ISeriesApi<'Candlestick'> | null = null
  let conteneur: HTMLElement | null = null
  let asset = ''
  let lignes: Map<number, IPriceLine> = new Map()
  let poll: ReturnType<typeof setInterval> | null = null
  /// Derniers ids déjà notifiés (évite le re-son au rechargement).
  let dejaNotifiees = new Set<number>()
  /// Glisser en cours : id de l'alerte attrapée (null = aucun).
  let glisseId: number | null = null
  /// Date du dernier relâchement — le clic généré par le drop s'ignore.
  let dernierDrop = 0

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
      // Ligne en cours de glissement : le drag est maître (le poll 10 s
      // ne doit pas ramener l'ancien niveau sous le curseur).
      if (a.id === glisseId) continue
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

  // ── Poll + nettoyage ────────────────────────────────────────────────────────
  async function recharger() {
    await store.recharger() // liste partagée (store) + rattrapage >2 min
    // Déclenchée détectée → SUPPRESSION : une alerte atteinte disparaît de
    // partout (graphique, bloc dashboard, base). La NOTIFICATION vit côté
    // serveur (watcher runtime → journal universel + Telegram matricé) —
    // plus de son/OS direct ici (doublon retiré 26/10).
    const aSupprimer: number[] = []
    for (const a of alertes.value) {
      if (!a.active && a.declenchee_le && !dejaNotifiees.has(a.id)) {
        dejaNotifiees.add(a.id)
        aSupprimer.push(a.id)
      }
    }
    if (aSupprimer.length) await store.supprimerPlusieurs(aSupprimer)
    synchroniserLignes()
  }

  // ── Pose au clic ────────────────────────────────────────────────────────────
  function surClic(param: Parameters<Parameters<IChartApi['subscribeClick']>[0]>[0]) {
    if (Date.now() - dernierDrop < 300) return // clic du relâchement → ignoré
    if (!modePose.value || !serie || !param.point) return
    const prix = serie.coordinateToPrice(param.point.y)
    if (prix === null || !isFinite(prix)) return
    // Sens auto : franchissement vers le niveau cliqué depuis le prix actuel.
    const dernier = (param.seriesData.get(serie) as { close?: number } | undefined)?.close
    const sens: 'au_dessus' | 'en_dessous' =
      dernier !== undefined && prix < (dernier as number) ? 'en_dessous' : 'au_dessus'
    modePose.value = false
    void store.creer(asset, prix as number, sens).then(() => { synchroniserLignes() })
  }

  // ── Glisser-déposer (owner 20/10) ───────────────────────────────────────────
  /// Alerte active dont la ligne est à ±5 px verticaux du curseur.
  function alerteSousCurseur(y: number): AlertePrix | null {
    if (!serie) return null
    for (const a of alertesAsset.value) {
      if (!a.active) continue
      const cy = serie.priceToCoordinate(a.prix)
      if (cy !== null && Math.abs(cy - y) <= 5) return a
    }
    return null
  }

  function surAppui(e: MouseEvent) {
    if (!conteneur || !serie) return
    const r = conteneur.getBoundingClientRect()
    const a = alerteSousCurseur(e.clientY - r.top)
    if (!a) return
    glisseId = a.id
    modePose.value = false // attraper une ligne prime sur la pose
    conteneur.style.cursor = 'ns-resize'
    e.preventDefault()
  }

  function surMouvement(e: MouseEvent) {
    if (!conteneur || !serie) return
    const r = conteneur.getBoundingClientRect()
    const y = e.clientY - r.top
    if (glisseId !== null) {
      const prix = serie.coordinateToPrice(y)
      if (prix !== null && isFinite(prix)) {
        lignes.get(glisseId)?.applyOptions({ price: prix as number })
      }
      return
    }
    // Survol : curseur d'ajustement quand on approche une ligne.
    conteneur.style.cursor = alerteSousCurseur(y) ? 'ns-resize' : ''
  }

  function surRelachement() {
    if (glisseId === null || !serie) return
    const id = glisseId
    glisseId = null
    dernierDrop = Date.now()
    if (conteneur) conteneur.style.cursor = ''
    const ligne = lignes.get(id)
    const options = ligne?.options() as { price?: number } | undefined
    const alerte = alertes.value.find(a => a.id === id)
    if (!options?.price || !alerte) { void recharger(); return }
    // Sens recalculé face au prix live (même règle que la pose au clic).
    const barres = serie.data()
    const dernier = barres.length ? (barres[barres.length - 1] as { close?: number }).close : undefined
    const sens: 'au_dessus' | 'en_dessous' =
      dernier !== undefined && options.price < dernier ? 'en_dessous' : 'au_dessus'
    void store.maj(id, options.price, sens).then(() => { synchroniserLignes() })
  }

  function basculerModePose() {
    modePose.value = !modePose.value
  }

  async function supprimer(id: number) {
    await store.supprimer(id)
    synchroniserLignes()
  }

  // ── Cycle de vie ────────────────────────────────────────────────────────────
  function initialiser(
    c: IChartApi,
    s: ISeriesApi<'Candlestick'>,
    assetCourant: string,
    element?: HTMLElement,
  ) {
    detruire()
    chart = c
    serie = s
    asset = assetCourant
    conteneur = element ?? null
    dejaNotifiees = new Set()
    chart.subscribeClick(surClic)
    if (conteneur) {
      conteneur.addEventListener('mousedown', surAppui)
      conteneur.addEventListener('mousemove', surMouvement)
      window.addEventListener('mouseup', surRelachement)
    }
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
    if (conteneur) {
      conteneur.removeEventListener('mousedown', surAppui)
      conteneur.removeEventListener('mousemove', surMouvement)
      window.removeEventListener('mouseup', surRelachement)
      conteneur.style.cursor = ''
    }
    glisseId = null
    if (serie) { for (const l of lignes.values()) { try { serie.removePriceLine(l) } catch { /* */ } } }
    lignes.clear()
    chart = null
    serie = null
    conteneur = null
  }

  return {
    alertes, alertesAsset, nbActives, modePose,
    initialiser, definirAsset, detruire, basculerModePose, supprimer, recharger,
  }
}
