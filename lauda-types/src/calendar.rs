use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarError {
    ParseError(String),
    InvalidFormat(String),
}

impl std::fmt::Display for CalendarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalendarError::ParseError(msg) => write!(f, "Erro de parse: {msg}"),
            CalendarError::InvalidFormat(msg) => write!(f, "Formato inválido: {msg}"),
        }
    }
}

impl std::error::Error for CalendarError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceFreq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceRule {
    pub freq: RecurrenceFreq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<EventTime>,
    pub by_day: Vec<Weekday>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RsvpStatus {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventAttendee {
    pub email: String,
    pub name: Option<String>,
    pub status: RsvpStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum EventTime {
    DateTimeUtc(i64),
    DateOnly { year: i32, month: u32, day: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: EventTime,
    pub end: EventTime,
    pub is_all_day: bool,
    pub rrule: Option<RecurrenceRule>,
    pub status: EventStatus,
    pub organizer: Option<EventAttendee>,
    pub attendees: Vec<EventAttendee>,
    pub sequence: u32,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

impl CalendarEvent {
    pub fn to_ical(&self) -> String {
        unimplemented!("Fase 4 Red Test")
    }

    pub fn from_ical(_raw: &str) -> Result<Self, CalendarError> {
        unimplemented!("Fase 4 Red Test")
    }
}

#[derive(Debug, Clone)]
pub struct CalendarReferenceTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub weekday: Weekday,
    pub timestamp_utc: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCalendarIntent {
    pub summary: String,
    pub start: EventTime,
    pub end: EventTime,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub rrule: Option<RecurrenceRule>,
}

pub struct CalendarIntentParser;

impl CalendarIntentParser {
    pub fn parse(
        _input: &str,
        _ref_time: &CalendarReferenceTime,
    ) -> Result<ParsedCalendarIntent, CalendarError> {
        unimplemented!("Fase 4 Red Test")
    }
}
