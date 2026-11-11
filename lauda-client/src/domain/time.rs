use chrono::{DateTime, Datelike, Local, Timelike};
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

/// Converte uma string de data (RFC 3339 / RFC 2822 / ISO 8601) para a representação no fuso horário local do dispositivo.
/// - Hoje: "HH:MM" (ex.: "14:35")
/// - Outro dia deste ano: "DD mmm HH:MM" (ex.: "10 nov 14:35")
/// - Anos anteriores: "DD/MM/YYYY" (ex.: "20/05/2024")
pub fn format_local_date_display(date_str: &str) -> String {
    let trimmed = date_str.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Tenta parsear via RFC 3339 primeiro
    let parsed_dt = DateTime::parse_from_rfc3339(trimmed)
        .map(|dt| dt.with_timezone(&Local))
        .or_else(|_| DateTime::parse_from_rfc2822(trimmed).map(|dt| dt.with_timezone(&Local)));

    let local_dt = match parsed_dt {
        Ok(dt) => dt,
        Err(_) => {
            // Fallback via timestamp unix caso o parser customizado do cache tenha sucesso
            if let Some(unix) =
                crate::domain::cache::LocalMessageCache::parse_iso_date_to_unix(trimmed)
            {
                if let Some(utc) = DateTime::from_timestamp(unix as i64, 0) {
                    utc.with_timezone(&Local)
                } else {
                    return trimmed.to_string();
                }
            } else {
                return trimmed.to_string();
            }
        }
    };

    let now = Local::now();
    let current_year = now.year();
    let msg_year = local_dt.year();

    if current_year != msg_year {
        return local_dt.format("%d/%m/%Y").to_string();
    }

    if now.month() == local_dt.month() && now.day() == local_dt.day() {
        return local_dt.format("%H:%M").to_string();
    }

    // Mês abreviado em português
    let month_str = match local_dt.month() {
        1 => "jan",
        2 => "fev",
        3 => "mar",
        4 => "abr",
        5 => "mai",
        6 => "jun",
        7 => "jul",
        8 => "ago",
        9 => "set",
        10 => "out",
        11 => "nov",
        12 => "dez",
        _ => "",
    };

    format!(
        "{:02} {} {:02}:{:02}",
        local_dt.day(),
        month_str,
        local_dt.hour(),
        local_dt.minute()
    )
}
