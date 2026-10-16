use dioxus::prelude::*;
use lauda_types::calendar::{
    CalendarEvent, CalendarIntentParser, CalendarReferenceTime, EventStatus, EventTime,
    ParsedCalendarIntent, Weekday, format_event_datetime_chip, timestamp_to_utc_parts,
};

use crate::domain::calendar_cache::LocalCalendarCache;
use crate::domain::session::SessionConfig;
use crate::domain::time::current_unix_timestamp;
use crate::ui::state::Screen;
use lauda_types::contact::ContactEntry;

/// Texto de instrução e sugestão (placeholder) genérico e neutro para o campo Quick-Add.
pub const QUICK_ADD_DEFAULT_PLACEHOLDER: &str = "Ex: Reunião de equipe amanhã às 14h na Sala 2";

/// Seleciona até `limit` eventos cronológicos futuros ordenados pelo início.
pub fn select_upcoming_events(
    events: &[CalendarEvent],
    now_ts: i64,
    limit: usize,
) -> Vec<CalendarEvent> {
    let mut upcoming: Vec<CalendarEvent> = events
        .iter()
        .filter(|ev| ev.start.to_timestamp_utc() >= now_ts)
        .cloned()
        .collect();

    upcoming.sort_by_key(|ev| ev.start.to_timestamp_utc());
    upcoming.truncate(limit);
    upcoming
}

/// Formata o tempo restante até um evento de forma amigável e legível para humanos.
pub fn format_relative_event_time(event_time: &EventTime, now_ts: i64) -> String {
    match event_time {
        EventTime::DateTimeUtc(ts) => {
            let diff_sec = ts - now_ts;
            if diff_sec < 60 {
                return "agora".to_string();
            }
            if diff_sec < 3600 {
                let mins = diff_sec / 60;
                return format!("em {mins} min");
            }
            if diff_sec < 86400 {
                let hours = diff_sec / 3600;
                let mins = (diff_sec % 3600) / 60;
                if mins == 0 {
                    return format!("em {hours}h");
                } else {
                    return format!("em {hours}h {mins}min");
                }
            }

            let days = diff_sec / 86400;
            let rem_sec = diff_sec % 86400;
            let hours = rem_sec / 3600;

            if days == 1 {
                if hours > 0 {
                    format!("em 1 dia e {hours}h")
                } else {
                    "em 1 dia".to_string()
                }
            } else if hours > 0 {
                format!("em {days} dias e {hours}h")
            } else {
                format!("em {days} dias")
            }
        }
        EventTime::DateOnly { year, month, day } => {
            let (now_y, now_m, now_d, _, _, _) = timestamp_to_utc_parts(now_ts);
            let today_ts =
                lauda_types::calendar::utc_parts_to_timestamp(now_y, now_m, now_d, 0, 0, 0);
            let ev_ts = lauda_types::calendar::utc_parts_to_timestamp(*year, *month, *day, 0, 0, 0);
            let diff_days = (ev_ts - today_ts) / 86400;

            if diff_days <= 0 {
                "Hoje".to_string()
            } else if diff_days == 1 {
                "Amanhã".to_string()
            } else {
                format!("em {diff_days} dias")
            }
        }
    }
}

fn apply_contact_to_input(
    contact: &ContactEntry,
    quick_input: &mut Signal<String>,
    attendee_to_link: &Option<String>,
) {
    let current = quick_input.read().clone();
    let name = contact
        .display_name
        .as_deref()
        .unwrap_or(&contact.primary_email);
    let email = &contact.primary_email;
    let replacement = format!("\"{name}\" <{email}>");

    if let Some(target) = attendee_to_link
        && let Some(idx) = current.find(target)
    {
        let mut new_val = current[..idx].to_string();
        new_val.push_str(&replacement);
        new_val.push_str(&current[(idx + target.len())..]);
        quick_input.set(new_val);
        return;
    }

    let mut new_val = current;
    new_val.push_str(&format!(" com {replacement}"));
    quick_input.set(new_val);
}

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
        attendees: intent.attendees,
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
    let contact_suggestions = use_signal(Vec::<ContactEntry>::new);
    let active_suggestion_index = use_signal(|| None::<usize>);
    let dismissed_query = use_signal(String::new);

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

    let mut current_now = use_signal(|| current_unix_timestamp() as i64);

    // Tique temporal reativo contínuo (atualiza a cada 30 segundos sem bloqueio)
    use_effect(move || {
        spawn(async move {
            loop {
                crate::domain::time::async_sleep(std::time::Duration::from_secs(30)).await;
                current_now.set(current_unix_timestamp() as i64);
            }
        });
    });

    let now_ts = *current_now.read();
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

    let attendee_to_link = parsed_intent.as_ref().and_then(|intent| {
        intent
            .attendees
            .iter()
            .find(|att| att.email.is_empty())
            .and_then(|att| att.name.clone())
    });

    // Agrupamento temporal para a Timeline Stream
    let events_read = events.read();
    let today_start_ts = lauda_types::calendar::utc_parts_to_timestamp(y, m, d, 0, 0, 0);
    let today_end_ts = today_start_ts + 86400;
    let tomorrow_end_ts = today_end_ts + 86400;

    let mut today_events = Vec::new();
    let mut tomorrow_events = Vec::new();
    let mut upcoming_events = Vec::new();

    for ev in events_read.iter() {
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

    let upcoming_highlights = select_upcoming_events(&events_read, now_ts, 3);

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
                        placeholder: "{QUICK_ADD_DEFAULT_PLACEHOLDER}",
                        value: "{quick_input.read()}",
                        oninput: {
                            let mut quick_input = quick_input;
                            let mut contact_suggestions = contact_suggestions;
                            let mut active_suggestion_index = active_suggestion_index;
                            move |evt: FormEvent| {
                                let val = evt.value();
                                quick_input.set(val.clone());

                                if val.ends_with(' ') {
                                    contact_suggestions.set(Vec::new());
                                    active_suggestion_index.set(None);
                                    return;
                                }

                                if let Ok(intent) = CalendarIntentParser::parse(&val, &ref_time)
                                    && let Some(att) = intent.attendees.iter().find(|a| a.email.is_empty())
                                    && let Some(ref name) = att.name
                                    && dismissed_query.read().as_str() != name.as_str()
                                {
                                    let name_clone = name.clone();
                                    spawn(async move {
                                        let cache_dir = crate::domain::cache::get_default_cache_dir();
                                        let matches = crate::domain::view::search_contacts_fuzzy_async(cache_dir, name_clone, 5).await;
                                        contact_suggestions.set(matches);
                                        active_suggestion_index.set(None);
                                    });
                                    return;
                                }
                                contact_suggestions.set(Vec::new());
                                active_suggestion_index.set(None);
                            }
                        },
                        onkeydown: {
                            let mut quick_input = quick_input;
                            let mut contact_suggestions = contact_suggestions;
                            let mut active_suggestion_index = active_suggestion_index;
                            let mut dismissed_query = dismissed_query;
                            let attendee_to_link = attendee_to_link.clone();
                            move |evt: KeyboardEvent| {
                                let suggestions = contact_suggestions.read().clone();
                                let count = suggestions.len();
                                if count > 0 {
                                    match evt.key() {
                                        Key::ArrowDown => {
                                            evt.prevent_default();
                                            let current_idx = *active_suggestion_index.read();
                                            let next = match current_idx {
                                                Some(idx) => (idx + 1) % count,
                                                None => 0,
                                            };
                                            active_suggestion_index.set(Some(next));
                                            return;
                                        }
                                        Key::ArrowUp => {
                                            evt.prevent_default();
                                            let current_idx = *active_suggestion_index.read();
                                            let prev = match current_idx {
                                                Some(idx) => (idx + count - 1) % count,
                                                None => count.saturating_sub(1),
                                            };
                                            active_suggestion_index.set(Some(prev));
                                            return;
                                        }
                                        Key::Enter => {
                                            let current_idx = *active_suggestion_index.read();
                                            if let Some(idx) = current_idx
                                                && let Some(selected) = suggestions.get(idx)
                                            {
                                                evt.prevent_default();
                                                apply_contact_to_input(selected, &mut quick_input, &attendee_to_link);
                                                contact_suggestions.set(Vec::new());
                                                active_suggestion_index.set(None);
                                                return;
                                            }
                                        }
                                        Key::Character(ref ch) if ch == " " => {
                                            if let Some(ref att_name) = attendee_to_link {
                                                dismissed_query.set(att_name.clone());
                                            }
                                            contact_suggestions.set(Vec::new());
                                            active_suggestion_index.set(None);
                                            return;
                                        }
                                        Key::Escape => {
                                            evt.prevent_default();
                                            if let Some(ref att_name) = attendee_to_link {
                                                dismissed_query.set(att_name.clone());
                                            }
                                            contact_suggestions.set(Vec::new());
                                            active_suggestion_index.set(None);
                                            return;
                                        }
                                        _ => {}
                                    }
                                }

                                if evt.key() == Key::Enter {
                                    create_event_action(quick_input, error_msg, events, &ref_time);
                                    contact_suggestions.set(Vec::new());
                                    active_suggestion_index.set(None);
                                }
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

                // Dropdown de sugestões de contatos (abaixo do campo de entrada)
                if !contact_suggestions.read().is_empty() {
                    div {
                        style: "border: 2px solid #0a0a0a; background: #fff; box-shadow: 4px 4px 0 #0a0a0a; max-height: 180px; overflow-y: auto; touch-action: pan-y; -webkit-overflow-scrolling: touch; display: flex; flex-direction: column; margin-top: -0.2rem; margin-bottom: 0.4rem; z-index: 50;",
                        for (i, c) in contact_suggestions.read().clone().into_iter().enumerate() {
                            {
                                let is_active = *active_suggestion_index.read() == Some(i);
                                let c_clone = c.clone();
                                let att_clone = attendee_to_link.clone();
                                let display_name = c.display_name.clone().unwrap_or_else(|| c.primary_email.clone());
                                let primary_email = c.primary_email.clone();
                                let has_auto = c.has_autocrypt;

                                rsx! {
                                    div {
                                        key: "{c.id}",
                                        style: if is_active {
                                            "background: #0a0a0a; color: #fff; padding: 0.5rem 0.8rem; cursor: pointer; display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #333;"
                                        } else {
                                            "background: #fff; color: #0a0a0a; padding: 0.5rem 0.8rem; cursor: pointer; display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #eee;"
                                        },
                                        onclick: {
                                            let mut quick_input = quick_input;
                                            let mut contact_suggestions = contact_suggestions;
                                            let mut active_suggestion_index = active_suggestion_index;
                                            move |_| {
                                                apply_contact_to_input(&c_clone, &mut quick_input, &att_clone);
                                                contact_suggestions.set(Vec::new());
                                                active_suggestion_index.set(None);
                                            }
                                        },
                                        div {
                                            style: "display: flex; flex-direction: column; gap: 0.1rem;",
                                            span {
                                                style: "font-weight: bold; font-size: 0.85rem;",
                                                "{display_name}"
                                            }
                                            span {
                                                style: if is_active { "font-size: 0.75rem; color: #ccc;" } else { "font-size: 0.75rem; color: #666;" },
                                                "{primary_email}"
                                            }
                                        }
                                        if has_auto {
                                            span {
                                                style: "background: #2e7d32; color: #fff; font-size: 0.65rem; font-weight: bold; padding: 0.1rem 0.4rem; border-radius: 2px;",
                                                "AUTOCRYPT"
                                            }
                                        }
                                    }
                                }
                            }
                        }
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
                        for att in &intent.attendees {
                            {
                                let is_linked = !att.email.is_empty() && att.name.as_deref() != Some(&att.email);
                                let label = if is_linked {
                                    if let Some(ref n) = att.name {
                                        format!("{n} <{}>", att.email)
                                    } else {
                                        att.email.clone()
                                    }
                                } else {
                                    att.name.as_deref().unwrap_or(&att.email).to_string()
                                };
                                rsx! {
                                    span {
                                        style: if is_linked {
                                            "background: #e8f5e9; border: 1px solid #2e7d32; color: #1b5e20; padding: 0.1rem 0.4rem; font-size: 0.75rem; font-weight: bold;"
                                        } else {
                                            "background: #fff; border: 1px solid #0a0a0a; padding: 0.1rem 0.4rem; font-size: 0.75rem;"
                                        },
                                        "👤 {label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Banner de Próximos Compromissos (se houver, até 3 eventos)
            if !upcoming_highlights.is_empty() {
                div {
                    style: "border: 2px solid #0a0a0a; background: #0a0a0a; color: #fff; padding: 0.8rem 1rem; box-shadow: 3px 3px 0 #dc143c; display: flex; flex-direction: column; gap: 0.6rem;",
                    div {
                        style: "display: flex; align-items: center; justify-content: space-between;",
                        span {
                            style: "background: #dc143c; color: #fff; font-weight: bold; font-size: 0.75rem; padding: 0.2rem 0.5rem; letter-spacing: 0.5px;",
                            "PRÓXIMOS COMPROMISSOS"
                        }
                        span {
                            style: "font-size: 0.75rem; color: #aaa;",
                            "{upcoming_highlights.len()} DESTAQUE(S)"
                        }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 0.4rem;",
                        for ev in &upcoming_highlights {
                            {
                                let loc_label = ev.location.as_deref().map(|l| format!("📍 {l}")).unwrap_or_default();
                                rsx! {
                                    div {
                                        key: "{ev.uid}",
                                        style: "display: flex; align-items: center; justify-content: space-between; border-top: 1px solid #222; padding-top: 0.4rem; flex-wrap: wrap; gap: 0.4rem;",
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.6rem;",
                                            span {
                                                style: "font-weight: bold; font-size: 0.9rem;",
                                                "{ev.summary}"
                                            }
                                            if !loc_label.is_empty() {
                                                span {
                                                    style: "font-size: 0.75rem; color: #bbb;",
                                                    "{loc_label}"
                                                }
                                            }
                                        }
                                        span {
                                            style: "font-size: 0.8rem; color: #ccc;",
                                            "{format_relative_event_time(&ev.start, now_ts)}"
                                        }
                                    }
                                }
                            }
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
                        style: "display: flex; gap: 0.5rem; align-items: center; font-size: 0.8rem; color: #555; flex-wrap: wrap;",
                        span {
                            style: "background: #eee; border: 1px solid #ccc; padding: 0.1rem 0.3rem; font-size: 0.7rem;",
                            "📅 {format_event_datetime_chip(&event)}"
                        }
                        if let Some(ref loc) = event.location {
                            span {
                                style: "background: #eee; border: 1px solid #ccc; padding: 0.1rem 0.3rem; font-size: 0.7rem;",
                                "📍 {loc}"
                            }
                        }
                        if let Some(ref rr) = event.rrule {
                            span {
                                style: "background: #eee; border: 1px solid #ccc; padding: 0.1rem 0.3rem; font-size: 0.7rem;",
                                "🔁 {rr.freq.to_ical_str()}"
                            }
                        }
                        for att in &event.attendees {
                            {
                                let is_linked = !att.email.is_empty() && att.name.as_deref() != Some(&att.email);
                                let label = if is_linked {
                                    if let Some(ref n) = att.name {
                                        format!("{n} <{}>", att.email)
                                    } else {
                                        att.email.clone()
                                    }
                                } else {
                                    att.name.as_deref().unwrap_or(&att.email).to_string()
                                };
                                rsx! {
                                    span {
                                        style: if is_linked {
                                            "background: #e8f5e9; border: 1px solid #2e7d32; color: #1b5e20; padding: 0.1rem 0.3rem; font-size: 0.7rem; font-weight: bold;"
                                        } else {
                                            "background: #eee; border: 1px solid #ccc; padding: 0.1rem 0.3rem; font-size: 0.7rem;"
                                        },
                                        "👤 {label}"
                                    }
                                }
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
