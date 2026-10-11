use std::time::Duration;

/// Retorna o timestamp UNIX atual em segundos de forma segura em todas as plataformas (incluindo WASM).
pub fn current_unix_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Pausa assíncrona com suporte nativo e WebAssembly (utilizando `setTimeout` via `js_sys::Promise`).
pub async fn async_sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(duration).await;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let millis = duration.as_millis() as i32;
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            if let Some(win) = web_sys::window() {
                let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis);
            }
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }
}
