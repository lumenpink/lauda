package dev.dioxus.main

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.work.Constraints
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import in.twolp.lauda.LaudaSyncWorker
import java.util.concurrent.TimeUnit

typealias BuildConfig = com.example.LaudaClient.BuildConfig

class MainActivity : WryActivity() {
    private var webViewRef: WebView? = null

    override fun onWebViewCreate(webView: WebView) {
        super.onWebViewCreate(webView)
        webViewRef = webView
    }

    override fun onResume() {
        super.onResume()
        // Fast Resume Sync: ao retornar ao primeiro plano, avisa para disparar sincronização imediata
        webViewRef?.evaluateJavascript(
            "(function() { if (window.__lauda_trigger_sync) window.__lauda_trigger_sync(); })()"
        ) { _ -> }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Configurar sincronização periódica de 15 minutos em segundo plano via WorkManager
        try {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()

            val syncRequest = PeriodicWorkRequestBuilder<LaudaSyncWorker>(15, TimeUnit.MINUTES)
                .setConstraints(constraints)
                .build()

            WorkManager.getInstance(applicationContext).enqueueUniquePeriodicWork(
                "LaudaPeriodicSync",
                ExistingPeriodicWorkPolicy.KEEP,
                syncRequest
            )
        } catch (e: Exception) {
            android.util.Log.e("MainActivity", "Erro ao agendar WorkManager: ${e.message}")
        }

        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                val wv = webViewRef
                if (wv != null && wv.canGoBack()) {
                    wv.goBack()
                } else if (wv != null) {
                    wv.evaluateJavascript(
                        "(function() { return (window.__lauda_handle_back && window.__lauda_handle_back()) ? 'true' : 'false'; })()"
                    ) { result ->
                        if (result != "\"true\"" && result != "true") {
                            isEnabled = false
                            onBackPressedDispatcher.onBackPressed()
                            isEnabled = true
                        }
                    }
                } else {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                    isEnabled = true
                }
            }
        })
    }
}
