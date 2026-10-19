package fr.relais.trading.relais

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import java.io.BufferedReader
import java.net.HttpURLConnection
import java.net.URL
import java.util.Calendar
import java.util.TimeZone
import kotlin.concurrent.thread
import org.json.JSONObject

/**
 * Service de premier plan « relais actif » (MOB ③).
 *
 * Poll le backend maison toutes les 10 min (INTERVALLE_MS) :
 * GET {serveur}/api/mobile/journal?limite=10 (+ Bearer token).
 * Chaque alerte d'id > dernierId est notifiée en système HORS muette
 * 21h→8h Paris ; pendant la muette le curseur avance silencieusement
 * (le Briefing de 8h30 résume la nuit — option b de l'owner).
 * Au premier démarrage, le curseur saute au present (pas de spam
 * d'anciennes alertes).
 */
class RelaisService : Service() {

    companion object {
        const val CANAL_ALERTES = "relais_alertes"
        const val CANAL_PERMANENTE = "relais_permanente"
        const val NOTIF_PERMANENTE_ID = 1
        const val INTERVALLE_MS = 10 * 60 * 1000L
        const val PREFS = "relais"

        @Volatile var actif = false
            private set

        /** Démarrage idempotent depuis le plugin ou le boot receiver.
         *  @return true si le service a bien été lancé */
        fun demarrer(context: Context, intent: Intent): Boolean {
            return try {
                ContextCompat.startForegroundService(context, intent)
                true
            } catch (e: Exception) {
                false
            }
        }
    }

    private var serveur = ""
    private var token = ""
    private var boucle: Thread? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        creerCanaux()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        serveur = (intent?.getStringExtra("serveur") ?: prefs().getString("serveur", "") ?: "")
            .trimEnd('/')
        token = intent?.getStringExtra("token") ?: prefs().getString("token", "") ?: ""
        prefs().edit()
            .putString("serveur", serveur)
            .putString("token", token)
            .putBoolean("actif", true)
            .apply()

        if (serveur.isEmpty()) {
            stopSelf()
            return START_NOT_STICKY
        }
        // Premier lancement : curseur au présent (aucune notification
        // rétroactive), ensuite chaque nouvel id sera notifié.
        if (prefs().getLong("dernier_id", -1L) == -1L) {
            prefs().edit().putLong("dernier_id", System.currentTimeMillis() / 1000 * 100_000).apply()
        }

        actif = true
        startForeground(NOTIF_PERMANENTE_ID, notifPermanente("Démarrage…"))
        if (boucle?.isAlive != true) {
            boucle = thread(name = "relais-poll") {
                while (actif) {
                    cycle()
                    try {
                        Thread.sleep(INTERVALLE_MS)
                    } catch (_: InterruptedException) {
                        break
                    }
                }
            }
        }
        return START_STICKY
    }

    override fun onDestroy() {
        actif = false
        boucle?.interrupt()
        boucle = null
        prefs().edit().putBoolean("actif", false).apply()
        super.onDestroy()
    }

    // ── Un cycle : journal → nouvelles alertes → notifications ──────────

    private fun cycle() {
        try {
            val corps = requete("$serveur/api/mobile/journal?limite=10")
            val dernierId = prefs().getLong("dernier_id", 0L)
            val alertes = JSONObject(corps).getJSONArray("alertes")
            var nouvelles = 0
            // Parcours ascendant : du plus ancien au plus récent.
            val ids = mutableListOf<Pair<Long, JSONObject>>()
            for (i in 0 until alertes.length()) {
                val a = alertes.getJSONObject(i)
                ids.add(a.getLong("id") to a)
            }
            ids.sortBy { it.first }
            for ((id, a) in ids) {
                if (id <= dernierId) continue
                nouvelles++
                prefs().edit().putLong("dernier_id", id).apply()
                if (!muette()) notifier(id, a)
            }
            majPermanente(nouvelles)
        } catch (e: Exception) {
            majPermanenteErreur(e.message ?: "erreur réseau")
        }
    }

    /** GET avec Bearer éventuel ; null → serveur injoignable. */
    private fun requete(url: String): String {
        val conn = URL(url).openConnection() as HttpURLConnection
        conn.connectTimeout = 10_000
        conn.readTimeout = 15_000
        if (token.isNotEmpty()) conn.setRequestProperty("Authorization", "Bearer $token")
        try {
            if (conn.responseCode != 200) throw IllegalStateException("HTTP ${conn.responseCode}")
            return conn.inputStream.bufferedReader().use(BufferedReader::readText)
        } finally {
            conn.disconnect()
        }
    }

    private fun muette(): Boolean {
        val h = Calendar.getInstance(TimeZone.getTimeZone("Europe/Paris")).get(Calendar.HOUR_OF_DAY)
        return h >= 21 || h < 8
    }

    // ── Notifications ────────────────────────────────────────────────────

    private fun creerCanaux() {
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        nm.createNotificationChannel(
            NotificationChannel(CANAL_PERMANENTE, "Relais actif", NotificationManager.IMPORTANCE_MIN)
                .apply { description = "Notification permanente du service relais" })
        nm.createNotificationChannel(
            NotificationChannel(CANAL_ALERTES, "Alertes", NotificationManager.IMPORTANCE_HIGH)
                .apply { description = "Opportunités relayées par l'app maison" })
    }

    private fun ouvrirApp(): PendingIntent {
        val i = packageManager.getLaunchIntentForPackage(packageName)
            ?: Intent(Intent.ACTION_VIEW, Uri.parse("https://localhost"))
        return PendingIntent.getActivity(
            this, 0, i,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    private fun notifPermanente(texte: String): Notification =
        NotificationCompat.Builder(this, CANAL_PERMANENTE)
            .setSmallIcon(android.R.drawable.stat_notify_sync_noanim)
            .setContentTitle("📡 Relais actif")
            .setContentText(texte)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setContentIntent(ouvrirApp())
            .build()

    private fun majPermanente(nouvelles: Int) {
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        val h = Calendar.getInstance(TimeZone.getTimeZone("Europe/Paris"))
        val hh = String.format("%02d:%02d", h.get(Calendar.HOUR_OF_DAY), h.get(Calendar.MINUTE))
        val resume = when {
            nouvelles == 0 && muette() -> "Vérif $hh · muette jusqu'à 8h"
            nouvelles == 0 -> "Vérif $hh · rien de neuf"
            else -> "Vérif $hh · $nouvelles alerte(s) relayée(s)"
        }
        nm.notify(NOTIF_PERMANENTE_ID, notifPermanente(resume))
    }

    private fun majPermanenteErreur(message: String) {
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(NOTIF_PERMANENTE_ID, notifPermanente("⚠ Serveur injoignable ($message)"))
    }

    private fun notifier(id: Long, a: JSONObject) {
        val asset = if (a.isNull("asset")) "" else a.optString("asset", "")
        val tf = if (a.isNull("tf")) "" else a.optString("tf", "")
        val contexte = listOf(asset, tf).filter { it.isNotEmpty() }.joinToString(" · ")
        val n = NotificationCompat.Builder(this, CANAL_ALERTES)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle(a.optString("titre", "Alerte"))
            .setContentText(contexte)
            .setAutoCancel(true)
            .setContentIntent(ouvrirApp())
            .build()
        val nm = getSystemService(NOTIFICATION_SERVICE) as NotificationManager
        nm.notify(id.toInt(), n)
    }

    private fun prefs() = getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
