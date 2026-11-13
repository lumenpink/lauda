package in.twolp.lauda

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

const val ACTION_MESSAGE = "org.unifiedpush.android.connector.MESSAGE"
const val ACTION_NEW_ENDPOINT = "org.unifiedpush.android.connector.NEW_ENDPOINT"
const val ACTION_UNREGISTERED = "org.unifiedpush.android.connector.UNREGISTERED"
const val ACTION_REGISTRATION_FAILED = "org.unifiedpush.android.connector.REGISTRATION_FAILED"

const val EXTRA_MESSAGE = "message"
const val EXTRA_BYTES_MESSAGE = "bytes"
const val EXTRA_ENDPOINT = "endpoint"
const val EXTRA_INSTANCE = "instance"

class LaudaPushReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "LaudaPushReceiver"
    }

    override fun onReceive(context: Context, intent: Intent) {
        when (intent.action) {
            ACTION_MESSAGE -> {
                val instance = intent.getStringExtra(EXTRA_INSTANCE) ?: ""
                val msgBytes = intent.getByteArrayExtra(EXTRA_BYTES_MESSAGE)
                    ?: intent.getStringExtra(EXTRA_MESSAGE)?.toByteArray(Charsets.UTF_8)
                    ?: byteArrayOf()
                onMessage(context, msgBytes, instance)
            }
            ACTION_NEW_ENDPOINT -> {
                val endpoint = intent.getStringExtra(EXTRA_ENDPOINT) ?: return
                val instance = intent.getStringExtra(EXTRA_INSTANCE) ?: ""
                onNewEndpoint(context, endpoint, instance)
            }
            ACTION_REGISTRATION_FAILED -> {
                val instance = intent.getStringExtra(EXTRA_INSTANCE) ?: ""
                onRegistrationFailed(context, instance)
            }
            ACTION_UNREGISTERED -> {
                val instance = intent.getStringExtra(EXTRA_INSTANCE) ?: ""
                onUnregistered(context, instance)
            }
        }
    }

    fun onMessage(context: Context, message: ByteArray, instance: String) {
        Log.i(TAG, "Notificação push recebida via UnifiedPush (< 50ms dispatch)")
        // Regra de Ouro da Natasha: "Recebe, Enfileira e Morre"
        try {
            val pushWork = OneTimeWorkRequestBuilder<LaudaSyncWorker>()
                .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
                .build()

            WorkManager.getInstance(context.applicationContext).enqueueUniqueWork(
                "LaudaPushSync",
                ExistingWorkPolicy.KEEP,
                pushWork
            )
        } catch (e: Exception) {
            Log.e(TAG, "Erro ao enfileirar WorkManager no push", e)
        }
    }

    fun onNewEndpoint(context: Context, endpoint: String, instance: String) {
        Log.i(TAG, "Novo endpoint UnifiedPush gerado: $endpoint")
        CoroutineScope(Dispatchers.IO).launch {
            try {
                val url = java.net.URL("http://localhost:8080/api/push/register")
                val conn = url.openConnection() as java.net.HttpURLConnection
                conn.requestMethod = "POST"
                conn.setRequestProperty("Content-Type", "application/json")
                conn.doOutput = true
                val payload = """{"endpoint": "$endpoint", "instance": "$instance"}"""
                conn.outputStream.use { os ->
                    os.write(payload.toByteArray(Charsets.UTF_8))
                }
                val code = conn.responseCode
                Log.d(TAG, "Resposta do registro de push no servidor: $code")
            } catch (t: Throwable) {
                Log.w(TAG, "Registro de endpoint push no servidor pendente de conexão: ${t.message}")
            }
        }
    }

    fun onRegistrationFailed(context: Context, instance: String) {
        Log.w(TAG, "Falha no registro do distribuidor UnifiedPush para instância: $instance")
    }

    fun onUnregistered(context: Context, instance: String) {
        Log.i(TAG, "UnifiedPush desregistrado para instância: $instance")
    }
}
