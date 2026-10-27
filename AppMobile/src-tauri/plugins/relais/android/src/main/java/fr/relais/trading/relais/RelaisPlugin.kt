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

    // ── Deep-link notif → détail (26/10) ────────────────────────────────
    // La notification porte « relais_alerte_id » dans son intent. App
    // déjà ouverte → onNewIntent ; démarrage à froid → l'intent du
    // démarrage. L'app web demande « derniereNotif » au montage et à
    // chaque retour au premier plan, puis ouvre DetailAlerte(id).

    private var notifId: Long = -1L

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        val id = intent.getLongExtra("relais_alerte_id", -1L)
        if (id > 0) notifId = id
    }

    /** Consomme l'id de la notification qui a ouvert l'app (-1 si aucun). */
    @Command
    fun derniereNotif(invoke: Invoke) {
        val duLancement = activity.intent?.getLongExtra("relais_alerte_id", -1L) ?: -1L
        val id = if (notifId > 0) notifId else duLancement
        notifId = -1L
        activity.intent?.removeExtra("relais_alerte_id")
        val r = JSObject()
        r.put("id", if (id > 0) id else -1L)
        invoke.resolve(r)
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
