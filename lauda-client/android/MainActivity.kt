package dev.dioxus.main

import android.os.Bundle
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback

typealias BuildConfig = com.example.LaudaClient.BuildConfig

class MainActivity : WryActivity() {
    private var webViewRef: WebView? = null

    override fun onWebViewCreate(webView: WebView) {
        super.onWebViewCreate(webView)
        webViewRef = webView
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

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
