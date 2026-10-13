use serde::{Deserialize, Serialize};

use crate::android_sync::LAUDA_ACCOUNT_TYPE;
use crate::calendar::{CalendarEvent, EventStatus, EventTime};

/// Autoridade oficial do provedor nativo de calendário do Android.
pub const LAUDA_CALENDAR_AUTHORITY: &str = "com.android.calendar";

/// Constantes oficiais de status do CalendarContract.Events do Android.
pub const STATUS_TENTATIVE: i32 = 0;
pub const STATUS_CONFIRMED: i32 = 1;
pub const STATUS_CANCELED: i32 = 2;

/// Mapeia o status de um evento RFC 5545 para o inteiro canônico do Android CalendarContract.
pub fn event_status_to_android(status: EventStatus) -> i32 {
    match status {
        EventStatus::Tentative => STATUS_TENTATIVE,
        EventStatus::Confirmed => STATUS_CONFIRMED,
        EventStatus::Cancelled => STATUS_CANCELED,
    }
}

/// Operações atômicas contra o CalendarProvider do Android.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidCalendarOp {
    /// Inserção de evento vinculado à conta gerenciada do Lauda.
    InsertEvent {
        calendar_id: i64,
        account_name: String,
        account_type: String,
        sync_id: String,
        title: String,
        description: Option<String>,
        location: Option<String>,
        dtstart_ms: i64,
        dtend_ms: i64,
        all_day: bool,
        rrule: Option<String>,
        status: i32,
    },
    /// Atualização de evento existente pelo identificador de sincronização (_SYNC_ID).
    UpdateEvent {
        sync_id: String,
        title: String,
        description: Option<String>,
        location: Option<String>,
        dtstart_ms: i64,
        dtend_ms: i64,
        all_day: bool,
        rrule: Option<String>,
        status: i32,
    },
    /// Remoção de evento (Tombstone reconciliado pelo _SYNC_ID).
    DeleteEvent { sync_id: String },
}

/// Lote de sincronização pronto para ContentResolver.applyBatch(CalendarContract.AUTHORITY, ops).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AndroidCalendarBatch {
    pub account_name: String,
    pub operations: Vec<AndroidCalendarOp>,
}

/// Converte um `CalendarEvent` em uma operação de inserção nativa `AndroidCalendarOp::InsertEvent`.
pub fn map_event_to_insert_op(
    event: &CalendarEvent,
    account_name: &str,
    calendar_id: i64,
) -> AndroidCalendarOp {
    let dtstart_ms = match &event.start {
        EventTime::DateTimeUtc(ts) => ts * 1000,
        EventTime::DateOnly { year, month, day } => {
            let ts = crate::calendar::utc_parts_to_timestamp(*year, *month, *day, 0, 0, 0);
            ts * 1000
        }
    };

    let dtend_ms = match &event.end {
        EventTime::DateTimeUtc(ts) => ts * 1000,
        EventTime::DateOnly { year, month, day } => {
            let ts = crate::calendar::utc_parts_to_timestamp(*year, *month, *day, 23, 59, 59);
            ts * 1000
        }
    };

    let rrule = event.rrule.as_ref().map(|r| r.to_rrule_string());
    let status = event_status_to_android(event.status);

    AndroidCalendarOp::InsertEvent {
        calendar_id,
        account_name: account_name.to_string(),
        account_type: LAUDA_ACCOUNT_TYPE.to_string(),
        sync_id: event.uid.clone(),
        title: event.summary.clone(),
        description: event.description.clone(),
        location: event.location.clone(),
        dtstart_ms,
        dtend_ms,
        all_day: event.is_all_day,
        rrule,
        status,
    }
}

/// Constrói um lote atômico completo de sincronização a partir de eventos ativos e IDs deletados.
pub fn build_calendar_sync_batch(
    account_name: &str,
    calendar_id: i64,
    events: &[CalendarEvent],
    deleted_uids: &[String],
) -> AndroidCalendarBatch {
    let mut operations = Vec::with_capacity(events.len() + deleted_uids.len());

    for ev in events {
        operations.push(map_event_to_insert_op(ev, account_name, calendar_id));
    }

    for del_id in deleted_uids {
        operations.push(AndroidCalendarOp::DeleteEvent {
            sync_id: del_id.clone(),
        });
    }

    AndroidCalendarBatch {
        account_name: account_name.to_string(),
        operations,
    }
}
