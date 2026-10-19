pub mod android_calendar_sync;
pub mod android_sync;
pub mod auth;
pub mod calendar;
pub mod compose;
pub mod contact;
pub mod mailbox;
pub mod message;
pub mod phone;
pub mod sieve;

pub use android_calendar_sync::{
    AndroidCalendarBatch, AndroidCalendarOp, LAUDA_CALENDAR_AUTHORITY, STATUS_CANCELED,
    STATUS_CONFIRMED, STATUS_TENTATIVE, build_calendar_sync_batch, event_status_to_android,
    map_event_to_insert_op,
};
pub use android_sync::{
    AndroidContactBatch, AndroidMimeType, AndroidRawContactOp, LAUDA_ACCOUNT_TYPE,
};
pub use auth::{LoginRequest, TokenResponse};
pub use calendar::{
    CalendarError, CalendarEvent, CalendarIntentParser, CalendarReferenceTime, EventAttendee,
    EventStatus, EventTime, ItipMethod, ParsedCalendarIntent, RecurrenceFreq, RecurrenceRule,
    RsvpStatus, Weekday, format_event_datetime_chip, format_intent_date_chip,
    format_intent_datetime_chip, format_intent_time_chip, generate_itip_payload,
};
pub use compose::{AttachmentUpload, ComposeForm};
pub use contact::{ContactEntry, ContactInfo, ContactSyncRequest, ContactSyncResponse};
pub use mailbox::MailboxInfo;
pub use message::{AttachmentMeta, Message, MessageEnvelope, ViewMode};
pub use phone::{NormalizedPhone, PhoneError, PhoneKind, PhoneNormalizer};
pub use sieve::{
    ConditionField, MatchType, SieveAction, SieveCondition, SieveRule, SieveScriptBundle,
    generate_sieve_script, parse_sieve_script, predict_rule_from_envelope,
};
