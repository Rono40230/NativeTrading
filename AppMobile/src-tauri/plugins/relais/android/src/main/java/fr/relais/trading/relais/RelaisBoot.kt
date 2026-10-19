package fr.relais.trading.relais

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

/** Redémarre le relais au boot si l'owner l'avait activé (préférence
 *  « actif » posée par le service, jamais par défaut). */
class RelaisBoot : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
        val prefs = context.getSharedPreferences(RelaisService.PREFS, Context.MODE_PRIVATE)
        if (!prefs.getBoolean("actif", false)) return
        val i = Intent(context, RelaisService::class.java)
        RelaisService.demarrer(context, i)
    }
}
