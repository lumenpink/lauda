package in.twolp.lauda

import android.content.Context
import android.util.Log
import androidx.work.ExistingWorkPolicy
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import org.unifiedpush.android.connector.MessagingReceiver

class LaudaPushReceiver : MessagingReceiver() {

    companion object {
        private const val TAG = "LaudaPushReceiver"
    }

    override fun onMessage(context: Context, message: ByteArray, instance: String) {
        Log.i(TAG, "Notificação push recebida via UnifiedPush (< 50ms dispatch)")
        // Regra de Ouro da Natasha: "Recebe, Enfileira e Morre"
        // Passa o bastão imediatamente para o WorkManager com política KEEP e setExpedited
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

    override fun onNewEndpoint(context: Context, endpoint: String, instance: String) {
        Log.i(TAG, "Novo endpoint UnifiedPush gerado: $endpoint")
        // Despacha o endpoint ao servidor em corrotina assíncrona desacoplada
        CoroutineScope(Dispatchers.IO).launch {
            try {
                // Registrar endpoint assincronamente no lauda-server
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

    override fun onRegistrationFailed(context: Context, instance: String) {
        Log.w(TAG, "Falha no registro do distribuidor UnifiedPush para instância: $instance")
    }

    override fun onUnregistered(context: Context, instance: String) {
        Log.i(TAG, "UnifiedPush desregistrado para instância: $instance")
    }
}
