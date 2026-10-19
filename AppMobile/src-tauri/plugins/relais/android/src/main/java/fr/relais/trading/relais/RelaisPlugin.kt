package fr.relais.trading.relais

import android.Manifest
import android.app.Activity
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Pont JS ↔ service : les 3 commandes Rust du plugin aboutissent ici.
 * Le serveur/token passés par les Réglages de l'app web sont confiés au
 * service (persistés en SharedPreferences pour survivre aux redémarrages).
 */
@TauriPlugin
class RelaisPlugin(private val activity: Activity) : Plugin(activity) {

    @Command
    fun demarrer(invoke: Invoke) {
        val args = invoke.getArgs()
        val serveur = args.optString("serveur", "").trimEnd('/')
        val token = args.optString("token", "")
        if (serveur.isEmpty()) {
            invoke.reject("serveur requis")
            return
        }
        demanderDroitNotifs()
        val i = Intent(activity, RelaisService::class.java)
            .putExtra("serveur", serveur)
            .putExtra("token", token)
        if (RelaisService.demarrer(activity, i)) invoke.resolve()
        else invoke.reject("démarrage du service impossible")
    }

    @Command
    fun arreter(invoke: Invoke) {
        activity.stopService(Intent(activity, RelaisService::class.java))
        invoke.resolve()
    }

    @Command
    fun etat(invoke: Invoke) {
        val prefs = activity.getSharedPreferences(RelaisService.PREFS, Activity.MODE_PRIVATE)
        val etat = JSObject()
        etat.put("actif", RelaisService.actif || prefs.getBoolean("actif", false))
        etat.put("serveur", prefs.getString("serveur", ""))
        etat.put("android", true)
        invoke.resolve(etat)
    }

    /** Android 13+ : la permission POST_NOTIFICATIONS est runtime. */
    private fun demanderDroitNotifs() {
        if (Build.VERSION.SDK_INT >= 33 &&
            activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 100)
        }
    }
}
