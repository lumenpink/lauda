pub mod android_sync;
pub mod auth;
pub mod calendar;
pub mod compose;
pub mod contact;
pub mod mailbox;
pub mod message;
pub mod phone;
pub mod sieve;

pub use android_sync::{
    AndroidContactBatch, AndroidMimeType, AndroidRawContactOp, LAUDA_ACCOUNT_TYPE,
};
pub use auth::{LoginRequest, TokenResponse};
pub use calendar::{
    CalendarError, CalendarEvent, CalendarIntentParser, CalendarReferenceTime, EventAttendee,
    EventStatus, EventTime, ParsedCalendarIntent, RecurrenceFreq, RecurrenceRule, RsvpStatus,
    Weekday,
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

