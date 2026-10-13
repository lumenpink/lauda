package in.twolp.lauda

import android.accounts.Account
import android.app.Service
import android.content.AbstractThreadedSyncAdapter
import android.content.ContentProviderClient
import android.content.Context
import android.content.Intent
import android.content.SyncResult
import android.os.Bundle
import android.os.IBinder
import android.util.Log

class LaudaCalendarSyncAdapter(context: Context, autoInitialize: Boolean) :
    AbstractThreadedSyncAdapter(context, autoInitialize) {

    override fun onPerformSync(
        account: Account?,
        extras: Bundle?,
        authority: String?,
        provider: ContentProviderClient?,
        syncResult: SyncResult?
    ) {
        Log.i("LaudaCalendarSync", "Disparando sincronização nativa de calendário para conta: ${account?.name}")
        // A reconciliação real consome o lote gerado pelo módulo Rust (lauda-types::android_calendar_sync)
    }
}

class LaudaCalendarSyncService : Service() {
    companion object {
        private var syncAdapter: LaudaCalendarSyncAdapter? = null
        private val lock = Any()
    }

    override fun onCreate() {
        super.onCreate()
        synchronized(lock) {
            if (syncAdapter == null) {
                syncAdapter = LaudaCalendarSyncAdapter(applicationContext, true)
            }
        }
    }

    override fun onBind(intent: Intent?): IBinder? = syncAdapter?.syncAdapterBinder
}
