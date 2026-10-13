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

class LaudaSyncAdapter(context: Context, autoInitialize: Boolean) :
    AbstractThreadedSyncAdapter(context, autoInitialize) {

    override fun onPerformSync(
        account: Account?,
        extras: Bundle?,
        authority: String?,
        provider: ContentProviderClient?,
        syncResult: SyncResult?
    ) {
        Log.i("LaudaSyncAdapter", "Disparando sincronização nativa de contatos para conta: ${account?.name}")
        // A reconciliação real consome o lote gerado pelo módulo Rust (lauda-types::android_sync)
    }
}

class LaudaSyncService : Service() {
    companion object {
        private var syncAdapter: LaudaSyncAdapter? = null
        private val lock = Any()
    }

    override fun onCreate() {
        super.onCreate()
        synchronized(lock) {
            if (syncAdapter == null) {
                syncAdapter = LaudaSyncAdapter(applicationContext, true)
            }
        }
    }

    override fun onBind(intent: Intent?): IBinder? = syncAdapter?.syncAdapterBinder
}
