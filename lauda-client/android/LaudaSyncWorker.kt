package `in`.twolp.lauda

import android.content.Context
import android.util.Log
import androidx.work.CoroutineWorker
import androidx.work.WorkerParameters
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class LaudaSyncWorker(appContext: Context, params: WorkerParameters) :
    CoroutineWorker(appContext, params) {

    companion object {
        private const val TAG = "LaudaSyncWorker"

        init {
            try {
                System.loadLibrary("main")
            } catch (e: UnsatisfiedLinkError) {
                Log.w(TAG, "libmain.so ainda não carregada ou mock em teste: ${e.message}")
            }
        }
    }

    external fun nativeSyncInbox(dataDir: String): Boolean

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        Log.i(TAG, "Iniciando ciclo periódico de sincronização de e-mails em segundo plano (anti-entropia)")
        val dataDir = applicationContext.filesDir.absolutePath
        val success = try {
            nativeSyncInbox(dataDir)
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Erro ao invocar nativeSyncInbox via JNI", e)
            false
        } catch (t: Throwable) {
            Log.e(TAG, "Exceção inesperada durante sync em background", t)
            false
        }

        if (success) {
            Log.i(TAG, "Sincronização periódica concluída com sucesso")
            Result.success()
        } else {
            Log.w(TAG, "Sincronização em background falhou; agendando retentativa")
            Result.retry()
        }
    }
}
