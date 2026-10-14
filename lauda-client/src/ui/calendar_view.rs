use dioxus::prelude::*;
use lauda_types::calendar::{
    CalendarEvent, CalendarIntentParser, CalendarReferenceTime, EventStatus, EventTime,
    ParsedCalendarIntent, Weekday, timestamp_to_utc_parts,
};

use crate::domain::calendar_cache::LocalCalendarCache;
use crate::domain::session::SessionConfig;
use crate::domain::time::current_unix_timestamp;
use crate::ui::state::Screen;

fn create_event_action(
    mut quick_input: Signal<String>,
    mut error_msg: Signal<Option<String>>,
    mut events: Signal<Vec<CalendarEvent>>,
    ref_time: &CalendarReferenceTime,
) {
    let text = quick_input.read().trim().to_string();
    if text.is_empty() {
        return;
    }

    let intent = match CalendarIntentParser::parse(&text, ref_time) {
        Ok(res) => res,
        Err(e) => {
            error_msg.set(Some(format!("Não foi possível compreender o evento: {e}")));
            return;
        }
    };

    let now = current_unix_timestamp() as i64;
    let event_uid = format!("event-{}", now);
    let event = CalendarEvent {
        uid: event_uid,
        summary: intent.summary,
        description: None,
        location: intent.location,
        start: intent.start,
        end: intent.end,
        is_all_day: intent.is_all_day,
        rrule: intent.rrule,
        status: EventStatus::Confirmed,
        organizer: None,
        attendees: Vec::new(),
        sequence: 1,
        created_at: Some(now),
        updated_at: Some(now),
    };

    // 1. Atualização OTIMISTA imediata na UI (0ms latency!)
    let mut current_list = events.read().clone();
    if let Some(pos) = current_list.iter().position(|e| e.uid == event.uid) {
        current_list[pos] = event.clone();
    } else {
        current_list.push(event.clone());
    }
    current_list.sort_by_key(|e| e.start.to_timestamp_utc());
    events.set(current_list);

    // 2. Persistência em cache (SQLite em Native ou IndexedDB na Web)
    let cache_dir = crate::domain::cache::get_default_cache_dir();
    if let Ok(cache) = LocalCalendarCache::open(&cache_dir) {
        let _ = cache.upsert_event(&event);
    }

    quick_input.set(String::new());
    error_msg.set(None);
}

fn delete_event_action(mut events: Signal<Vec<CalendarEvent>>, del_uid: &str) {
    let mut current_list = events.read().clone();
    current_list.retain(|e| e.uid != del_uid);
    events.set(current_list);

    let cache_dir = crate::domain::cache::get_default_cache_dir();
    if let Ok(cache) = LocalCalendarCache::open(&cache_dir) {
        let _ = cache.delete_event(del_uid);
    }
}

#[component]
pub fn CalendarView(mut screen: Signal<Screen>, session: Signal<SessionConfig>) -> Element {
    let mut events = use_signal(Vec::<CalendarEvent>::new);
    let mut is_loading = use_signal(|| true);
    let quick_input = use_signal(String::new);
    let mut error_msg = use_signal(|| None::<String>);

    // Carregamento imediato a partir do cache local SQLite (Native) ou IndexedDB (Web)
    use_effect(move || {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let cache_dir = crate::domain::cache::get_default_cache_dir();
            if let Ok(cache) = LocalCalendarCache::open(&cache_dir)
                && let Ok(local_list) = cache.load_events()
            {
                events.set(local_list);
                is_loading.set(false);
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            spawn(async move {
                if let Ok(list) = LocalCalendarCache::load_events_idb().await {
                    events.set(list);
                    is_loading.set(false);
                }
            });
        }
    });

    let now_ts = current_unix_timestamp() as i64;
    let (y, m, d, h, min, _s) = timestamp_to_utc_parts(now_ts);
    // 1970-01-01 foi Quinta-feira (4)
    let days_since_epoch = now_ts.div_euclid(86400);
    let weekday_num = ((days_since_epoch + 3).rem_euclid(7)) as u32;
    let ref_time = CalendarReferenceTime {
        year: y,
        month: m,
        day: d,
        hour: h,
        minute: min,
        weekday: Weekday::from_day_number(weekday_num),
        timestamp_utc: now_ts,
    };

    // Parser reativo da linha única de Quick-Add
    let current_input = quick_input.read().clone();
    let parsed_intent: Option<ParsedCalendarIntent> = if !current_input.trim().is_empty() {
        CalendarIntentParser::parse(&current_input, &ref_time).ok()
    } else {
        None
    };

    // Agrupamento temporal para a Timeline Stream
    let events_list = events.read().clone();
    let today_start_ts = lauda_types::calendar::utc_parts_to_timestamp(y, m, d, 0, 0, 0);
    let today_end_ts = today_start_ts + 86400;
    let tomorrow_end_ts = today_end_ts + 86400;

    let mut today_events = Vec::new();
    let mut tomorrow_events = Vec::new();
    let mut upcoming_events = Vec::new();

    for ev in &events_list {
        let st = ev.start.to_timestamp_utc();
        if st < today_end_ts && st >= today_start_ts {
            today_events.push(ev.clone());
        } else if st >= today_end_ts && st < tomorrow_end_ts {
            tomorrow_events.push(ev.clone());
        } else if st >= tomorrow_end_ts {
            upcoming_events.push(ev.clone());
        } else {
            // Eventos que já passaram hoje ou antes
            today_events.push(ev.clone());
        }
    }

    let next_event = events_list
        .iter()
        .find(|ev| ev.start.to_timestamp_utc() >= now_ts);

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 1rem; width: 100%; box-sizing: border-box;",

            // Header Principal da Agenda (Estética Trash Polka)
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; justify-content: space-between; align-items: center; gap: 0.8rem; flex-wrap: wrap;",

                div {
                    style: "display: flex; align-items: center; gap: 0.6rem;",
                    span {
                        style: "background: #dc143c; color: #fff; font-weight: bold; font-size: 0.85rem; padding: 0.3rem 0.6rem; letter-spacing: 1px;",
                        "AGENDA SOBERANA"
                    }
                    span {
                        style: "font-weight: bold; font-size: 0.85rem; color: #555;",
                        "// FLUXO CONTÍNUO"
                    }
                }

                button {
                    style: "background: #0a0a0a; color: #fff; border: 2px solid #0a0a0a; padding: 0.4rem 0.8rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #dc143c;",
                    onclick: move |_| screen.set(Screen::Inbox),
                    "← VOLTAR AO CORREIO"
                }
            }

            if let Some(ref err) = *error_msg.read() {
                div {
                    style: "border: 2px solid #dc143c; background: #ffebee; color: #b71c1c; padding: 0.8rem 1rem; font-weight: bold; font-size: 0.9rem; display: flex; justify-content: space-between; align-items: center;",
                    span { "⚠ {err}" }
                    button {
                        style: "background: none; border: none; font-weight: bold; cursor: pointer; color: #b71c1c;",
                        onclick: move |_| error_msg.set(None),
                        "✕"
                    }
                }
            }

            // Quick-Add Bar (Entrada em linha única sem modais)
            div {
                style: "border: 2px solid #0a0a0a; background: #fff; padding: 1rem; box-shadow: 4px 4px 0 #0a0a0a; display: flex; flex-direction: column; gap: 0.6rem;",

                div {
                    style: "font-size: 0.75rem; font-weight: bold; color: #dc143c; text-transform: uppercase; letter-spacing: 1px;",
                    "QUICK-ADD (DIGITE NATURALMENTE E TECLE ENTER)"
                }

                div {
                    style: "display: flex; gap: 0.5rem; width: 100%;",

                    input {
                        style: "flex: 1; border: 2px solid #0a0a0a; padding: 0.6rem 0.8rem; font-size: 0.95rem; outline: none; background: #fdfdfd;",
                        placeholder: "Ex: Almoço com Marcelo amanhã 12h30 no Fasano",
                        value: "{quick_input.read()}",
                        oninput: {
                            let mut quick_input = quick_input;
                            move |evt: FormEvent| quick_input.set(evt.value())
                        },
                        onkeydown: move |evt: KeyboardEvent| {
                            if evt.key() == Key::Enter {
                                create_event_action(quick_input, error_msg, events, &ref_time);
                            }
                        },
                    }

                    button {
                        style: "background: #dc143c; color: #fff; border: 2px solid #0a0a0a; padding: 0.6rem 1.2rem; font-weight: bold; cursor: pointer; box-shadow: 2px 2px 0 #0a0a0a; white-space: nowrap;",
                        onclick: move |_| {
                            create_event_action(quick_input, error_msg, events, &ref_time);
                        },
                        "+ CRIAR"
                    }
                }

                // Chip de prévia reativo instantâneo
                if let Some(ref intent) = parsed_intent {
                    div {
                        style: "background: #f7f7f7; border: 1px dashed #0a0a0a; padding: 0.5rem 0.8rem; display: flex; align-items: center; gap: 0.8rem; flex-wrap: wrap; font-size: 0.85rem;",

                        span {
                            style: "background: #0a0a0a; color: #fff; font-weight: bold; padding: 0.2rem 0.5rem; border-radius: 2px;",
                            "PRÉVIA"
                        }
                        span {
                            style: "font-weight: bold; color: #0a0a0a;",
                            "{intent.summary}"
                        }
                        span {
                            style: "color: #dc143c; font-weight: bold;",
                            if intent.is_all_day { "📅 Dia Inteiro" } else { "📅 Com horário definido" }
                        }
                        if let Some(ref loc) = intent.location {
                            span {
                                style: "background: #fff; border: 1px solid #0a0a0a; padding: 0.1rem 0.4rem;",
                                "📍 {loc}"
                            }
                        }
                        if let Some(ref rr) = intent.rrule {
                            span {
                                style: "background: #dc143c; color: #fff; padding: 0.1rem 0.4rem; font-size: 0.75rem;",
                                "🔁 {rr.freq.to_ical_str()}"
                            }
                        }
                    }
                }
            }

            // Banner de Próximo Compromisso (se houver)
            if let Some(next_ev) = next_event {
                div {
                    style: "border: 2px solid #0a0a0a; background: #0a0a0a; color: #fff; padding: 0.8rem 1rem; box-shadow: 3px 3px 0 #dc143c; display: flex; align-items: center; justify-content: space-between;",
                    div {
                        style: "display: flex; align-items: center; gap: 0.8rem;",
                        span {
                            style: "background: #dc143c; color: #fff; font-weight: bold; font-size: 0.75rem; padding: 0.2rem 0.5rem;",
                            "PRÓXIMO COMPROMISSO"
                        }
                        span {
                            style: "font-weight: bold; font-size: 0.95rem;",
                            "{next_ev.summary}"
                        }
                    }
                    span {
                        style: "font-size: 0.8rem; color: #ccc;",
                        match &next_ev.start {
                            EventTime::DateTimeUtc(ts) => {
                                let diff = (ts - now_ts) / 60;
                                if diff > 0 {
                                    format!("em {diff} min")
                                } else {
                                    "agora".to_string()
                                }
                            }
                            EventTime::DateOnly { .. } => "Hoje".to_string(),
                        }
                    }
                }
            }

            // Timeline Stream: Hoje, Amanhã e Próximos Dias
            div {
                style: "display: flex; flex-direction: column; gap: 1.2rem;",

                // Seção: Hoje
                div {
                    style: "display: flex; flex-direction: column; gap: 0.6rem;",
                    div {
                        style: "font-size: 0.85rem; font-weight: bold; color: #0a0a0a; border-bottom: 2px solid #0a0a0a; padding-bottom: 0.3rem; display: flex; justify-content: space-between;",
                        span { "HOJE" }
                        span { style: "color: #555;", "{today_events.len()} compromissos" }
                    }
                    if today_events.is_empty() {
                        div {
                            style: "padding: 1rem; background: #fff; border: 1px dashed #ccc; color: #666; font-size: 0.85rem;",
                            "Nenhum compromisso para hoje. Aproveite seu tempo!"
                        }
                    } else {
                        for ev in today_events {
                            {
                                let del_uid = ev.uid.clone();
                                rsx! {
                                    EventCard {
                                        event: ev,
                                        on_delete: move |_| {
                                            delete_event_action(events, &del_uid);
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                // Seção: Amanhã
                div {
                    style: "display: flex; flex-direction: column; gap: 0.6rem;",
                    div {
                        style: "font-size: 0.85rem; font-weight: bold; color: #0a0a0a; border-bottom: 2px solid #0a0a0a; padding-bottom: 0.3rem; display: flex; justify-content: space-between;",
                        span { "AMANHÃ" }
                        span { style: "color: #555;", "{tomorrow_events.len()} compromissos" }
                    }
                    if tomorrow_events.is_empty() {
                        div {
                            style: "padding: 1rem; background: #fff; border: 1px dashed #ccc; color: #666; font-size: 0.85rem;",
                            "Nenhum compromisso agendado para amanhã."
                        }
                    } else {
                        for ev in tomorrow_events {
                            {
                                let del_uid = ev.uid.clone();
                                rsx! {
                                    EventCard {
                                        event: ev,
                                        on_delete: move |_| {
                                            delete_event_action(events, &del_uid);
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                // Seção: Próximos Dias
                if !upcoming_events.is_empty() {
                    div {
                        style: "display: flex; flex-direction: column; gap: 0.6rem;",
                        div {
                            style: "font-size: 0.85rem; font-weight: bold; color: #0a0a0a; border-bottom: 2px solid #0a0a0a; padding-bottom: 0.3rem;",
                            "ESTA SEMANA & PRÓXIMOS DIAS"
                        }
                        for ev in upcoming_events {
                            {
                                let del_uid = ev.uid.clone();
                                rsx! {
                                    EventCard {
                                        event: ev,
                                        on_delete: move |_| {
                                            delete_event_action(events, &del_uid);
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EventCard(event: CalendarEvent, on_delete: EventHandler<()>) -> Element {
    let time_badge = match &event.start {
        EventTime::DateTimeUtc(ts) => {
            let (_, _, _, h, m, _) = timestamp_to_utc_parts(*ts);
            format!("{h:02}:{m:02}")
        }
        EventTime::DateOnly { .. } => "DIA INTEIRO".to_string(),
    };

    rsx! {
        div {
            style: "border: 2px solid #0a0a0a; background: #fff; padding: 0.8rem 1rem; box-shadow: 3px 3px 0 #0a0a0a; display: flex; justify-content: space-between; align-items: center; gap: 0.8rem;",

            div {
                style: "display: flex; align-items: center; gap: 0.8rem; flex-wrap: wrap;",

                span {
                    style: if event.is_all_day {
                        "background: #dc143c; color: #fff; font-weight: bold; font-size: 0.75rem; padding: 0.3rem 0.5rem; letter-spacing: 0.5px;"
                    } else {
                        "background: #0a0a0a; color: #fff; font-weight: bold; font-size: 0.8rem; padding: 0.3rem 0.6rem;"
                    },
                    "{time_badge}"
                }

                div {
                    style: "display: flex; flex-direction: column; gap: 0.2rem;",
                    span {
                        style: "font-weight: bold; font-size: 0.95rem; color: #0a0a0a;",
                        "{event.summary}"
                    }
                    div {
                        style: "display: flex; gap: 0.5rem; align-items: center; font-size: 0.8rem; color: #555;",
                        if let Some(ref loc) = event.location {
                            span { "📍 {loc}" }
                        }
                        if let Some(ref rr) = event.rrule {
                            span {
                                style: "background: #eee; border: 1px solid #ccc; padding: 0.1rem 0.3rem; font-size: 0.7rem;",
                                "🔁 {rr.freq.to_ical_str()}"
                            }
                        }
                    }
                }
            }

            button {
                style: "background: #fff; border: 1px solid #0a0a0a; padding: 0.3rem 0.6rem; font-size: 0.75rem; font-weight: bold; cursor: pointer; color: #dc143c;",
                onclick: move |_| on_delete.call(()),
                "✕ EXCLUIR"
            }
        }
    }
}
