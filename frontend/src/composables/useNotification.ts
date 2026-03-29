/**
 * useNotification — Notifications OS natives + alertes sonores.
 *
 * Utilise les commandes Tauri `notifier` (notify-send Linux) et
 * `jouer_son_signal` (paplay OGG). Sans effet si Tauri n'est pas disponible
 * (dev browser fallback silencieux).
 *
 * Usage :
 * ```ts
 * const { notifier, signalerSignal } = useNotification()
 * await signalerSignal('BTC/USDT', 'M15', 'Long', 0.82)
 * ```
 */

import { invoke } from '@tauri-apps/api/core'

// Tauri n'est pas disponible dans un vrai navigateur — garde-fou silencieux
function tauriDisponible(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export interface OptsNotification {
  urgence?: 'low' | 'normal' | 'critical'
  son?: boolean
}

export function useNotification() {
  /**
   * Envoie une notification OS avec titre + corps.
   */
  async function notifier(
    titre: string,
    corps: string,
    opts: OptsNotification = {}
  ): Promise<void> {
    if (!tauriDisponible()) return
    const { urgence = 'normal', son = false } = opts

    try {
      await invoke('notifier', { titre, corps, urgence })
      if (son) await jouerSon()
    } catch {
      // Silencieux — notify-send peut manquer en environnement minimal
    }
  }

  /**
   * Joue le son d'alerte signal.ogg.
   * Priorité : Audio HTML5 (fiable partout) → invoke Tauri/paplay en fallback.
   */
  async function jouerSon(): Promise<void> {
    try {
      const audio = new Audio('/signal.ogg')
      audio.volume = 0.8
      await audio.play()
    } catch {
      // Fallback Tauri/paplay si Audio API bloquée
      if (!tauriDisponible()) return
      try {
        await invoke('jouer_son_signal')
      } catch {
        // Silencieux
      }
    }
  }

  /**
   * Raccourci pour notifier un nouveau signal de trading.
   * Urgence "normal" + son activé.
   */
  async function signalerSignal(
    asset: string,
    timeframe: string,
    direction: string,
    confiance: number
  ): Promise<void> {
    const pct = Math.round(confiance * 100)
    await notifier(
      `🎯 Signal ${direction.toUpperCase()} — ${asset}`,
      `${timeframe} · Confiance ${pct}% · ${new Date().toLocaleTimeString('fr-FR')}`,
      { urgence: 'normal', son: true }
    )
  }

  /**
   * Notification d'alerte critique (drawdown, erreur majeure).
   */
  async function alerterCritique(message: string): Promise<void> {
    await notifier('⚠️ Native Trading AI', message, { urgence: 'critical', son: false })
  }

  return { notifier, jouerSon, signalerSignal, alerterCritique }
}
