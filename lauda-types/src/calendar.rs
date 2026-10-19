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

impl Weekday {
    pub fn to_ical_code(&self) -> &'static str {
        match self {
            Weekday::Monday => "MO",
            Weekday::Tuesday => "TU",
            Weekday::Wednesday => "WE",
            Weekday::Thursday => "TH",
            Weekday::Friday => "FR",
            Weekday::Saturday => "SA",
            Weekday::Sunday => "SU",
        }
    }

    pub fn from_ical_code(code: &str) -> Option<Self> {
        match code.trim().to_uppercase().as_str() {
            "MO" => Some(Weekday::Monday),
            "TU" => Some(Weekday::Tuesday),
            "WE" => Some(Weekday::Wednesday),
            "TH" => Some(Weekday::Thursday),
            "FR" => Some(Weekday::Friday),
            "SA" => Some(Weekday::Saturday),
            "SU" => Some(Weekday::Sunday),
            _ => None,
        }
    }

    pub fn day_number(&self) -> u32 {
        match self {
            Weekday::Monday => 0,
            Weekday::Tuesday => 1,
            Weekday::Wednesday => 2,
            Weekday::Thursday => 3,
            Weekday::Friday => 4,
            Weekday::Saturday => 5,
            Weekday::Sunday => 6,
        }
    }

    pub fn from_day_number(num: u32) -> Self {
        match num % 7 {
            0 => Weekday::Monday,
            1 => Weekday::Tuesday,
            2 => Weekday::Wednesday,
            3 => Weekday::Thursday,
            4 => Weekday::Friday,
            5 => Weekday::Saturday,
            _ => Weekday::Sunday,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceFreq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceFreq {
    pub fn to_ical_str(&self) -> &'static str {
        match self {
            RecurrenceFreq::Daily => "DAILY",
            RecurrenceFreq::Weekly => "WEEKLY",
            RecurrenceFreq::Monthly => "MONTHLY",
            RecurrenceFreq::Yearly => "YEARLY",
        }
    }

    pub fn from_ical_str(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "DAILY" => Some(RecurrenceFreq::Daily),
            "WEEKLY" => Some(RecurrenceFreq::Weekly),
            "MONTHLY" => Some(RecurrenceFreq::Monthly),
            "YEARLY" => Some(RecurrenceFreq::Yearly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurrenceRule {
    pub freq: RecurrenceFreq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<EventTime>,
    pub by_day: Vec<Weekday>,
}

impl RecurrenceRule {
    pub fn to_rrule_string(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("FREQ={}", self.freq.to_ical_str()));
        if self.interval >= 1 {
            parts.push(format!("INTERVAL={}", self.interval));
        }
        if let Some(count) = self.count {
            parts.push(format!("COUNT={count}"));
        }
        if let Some(until) = &self.until {
            parts.push(format!("UNTIL={}", until.to_ical_string()));
        }
        if !self.by_day.is_empty() {
            let days = self
                .by_day
                .iter()
                .map(|d| d.to_ical_code())
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("BYDAY={days}"));
        }
        parts.join(";")
    }

    pub fn from_rrule_string(s: &str) -> Result<Self, CalendarError> {
        let mut freq = None;
        let mut interval = 1;
        let mut count = None;
        let mut until = None;
        let mut by_day = Vec::new();

        for part in s.split(';') {
            let (key, val) = part.split_once('=').ok_or_else(|| {
                CalendarError::InvalidFormat(format!("Parte RRULE inválida: {part}"))
            })?;
            match key.trim().to_uppercase().as_str() {
                "FREQ" => {
                    freq = Some(RecurrenceFreq::from_ical_str(val).ok_or_else(|| {
                        CalendarError::InvalidFormat(format!("FREQ desconhecida: {val}"))
                    })?);
                }
                "INTERVAL" => {
                    interval = val.trim().parse::<u32>().map_err(|e| {
                        CalendarError::InvalidFormat(format!("INTERVAL inválido: {e}"))
                    })?;
                }
                "COUNT" => {
                    count = Some(val.trim().parse::<u32>().map_err(|e| {
                        CalendarError::InvalidFormat(format!("COUNT inválido: {e}"))
                    })?);
                }
                "UNTIL" => {
                    until = Some(EventTime::from_ical_string(val.trim())?);
                }
                "BYDAY" => {
                    for code in val.split(',') {
                        if let Some(wd) = Weekday::from_ical_code(code) {
                            by_day.push(wd);
                        }
                    }
                }
                _ => {}
            }
        }

        let freq = freq.ok_or_else(|| {
            CalendarError::InvalidFormat("RRULE sem campo FREQ obrigatório".to_string())
        })?;

        Ok(RecurrenceRule {
            freq,
            interval,
            count,
            until,
            by_day,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RsvpStatus {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
}

impl RsvpStatus {
    pub fn to_ical_str(&self) -> &'static str {
        match self {
            RsvpStatus::NeedsAction => "NEEDS-ACTION",
            RsvpStatus::Accepted => "ACCEPTED",
            RsvpStatus::Declined => "DECLINED",
            RsvpStatus::Tentative => "TENTATIVE",
        }
    }

    pub fn from_ical_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "ACCEPTED" => RsvpStatus::Accepted,
            "DECLINED" => RsvpStatus::Declined,
            "TENTATIVE" => RsvpStatus::Tentative,
            _ => RsvpStatus::NeedsAction,
        }
    }
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

impl EventStatus {
    pub fn to_ical_str(&self) -> &'static str {
        match self {
            EventStatus::Confirmed => "CONFIRMED",
            EventStatus::Tentative => "TENTATIVE",
            EventStatus::Cancelled => "CANCELLED",
        }
    }

    pub fn from_ical_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "CANCELLED" => EventStatus::Cancelled,
            "TENTATIVE" => EventStatus::Tentative,
            _ => EventStatus::Confirmed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItipMethod {
    Request,
    Reply,
    Cancel,
}

impl ItipMethod {
    pub fn to_ical_str(&self) -> &'static str {
        match self {
            ItipMethod::Request => "REQUEST",
            ItipMethod::Reply => "REPLY",
            ItipMethod::Cancel => "CANCEL",
        }
    }

    pub fn from_ical_str(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "REQUEST" => Some(ItipMethod::Request),
            "REPLY" => Some(ItipMethod::Reply),
            "CANCEL" => Some(ItipMethod::Cancel),
            _ => None,
        }
    }
}

// Algoritmo civil exato de calendário Gregoriano sem dependências externas
pub fn days_to_civil(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

pub fn civil_to_days(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let m = month as i64;
    let d = day as i64;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + (doy as u32);
    era * 146097 + (doe as i64) - 719468
}

pub fn timestamp_to_utc_parts(ts: i64) -> (i32, u32, u32, u32, u32, u32) {
    let days = ts.div_euclid(86400);
    let rem = ts.rem_euclid(86400);
    let (year, month, day) = days_to_civil(days);
    let hour = (rem / 3600) as u32;
    let minute = ((rem % 3600) / 60) as u32;
    let second = (rem % 60) as u32;
    (year, month, day, hour, minute, second)
}

pub fn utc_parts_to_timestamp(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> i64 {
    let days = civil_to_days(year, month, day);
    days * 86400 + (hour as i64 * 3600) + (minute as i64 * 60) + (second as i64)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum EventTime {
    DateTimeUtc(i64),
    DateOnly { year: i32, month: u32, day: u32 },
}

impl EventTime {
    pub fn to_ical_string(&self) -> String {
        match self {
            EventTime::DateTimeUtc(ts) => {
                let (y, m, d, h, min, sec) = timestamp_to_utc_parts(*ts);
                format!("{y:04}{m:02}{d:02}T{h:02}{min:02}{sec:02}Z")
            }
            EventTime::DateOnly { year, month, day } => {
                format!("{year:04}{month:02}{day:02}")
            }
        }
    }

    pub fn from_ical_string(s: &str) -> Result<Self, CalendarError> {
        let clean = s.trim();
        if clean.contains('T') {
            // Formato esperado: YYYYMMDDTHHMMSSZ ou YYYYMMDDTHHMMSS
            let stripped = clean.strip_suffix('Z').unwrap_or(clean);
            let (d_part, t_part) = stripped
                .split_once('T')
                .ok_or_else(|| CalendarError::InvalidFormat(format!("Horário sem T: {clean}")))?;
            if d_part.len() < 8 || t_part.len() < 4 {
                return Err(CalendarError::InvalidFormat(format!(
                    "Data/hora curta: {clean}"
                )));
            }
            let year = d_part[0..4]
                .parse::<i32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            let month = d_part[4..6]
                .parse::<u32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            let day = d_part[6..8]
                .parse::<u32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            let hour = t_part[0..2]
                .parse::<u32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            let minute = t_part[2..4]
                .parse::<u32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            let second = if t_part.len() >= 6 {
                t_part[4..6].parse::<u32>().unwrap_or(0)
            } else {
                0
            };
            let ts = utc_parts_to_timestamp(year, month, day, hour, minute, second);
            Ok(EventTime::DateTimeUtc(ts))
        } else if clean.len() >= 8 {
            let year = clean[0..4]
                .parse::<i32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            let month = clean[4..6]
                .parse::<u32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            let day = clean[6..8]
                .parse::<u32>()
                .map_err(|e| CalendarError::InvalidFormat(e.to_string()))?;
            Ok(EventTime::DateOnly { year, month, day })
        } else {
            Err(CalendarError::InvalidFormat(format!(
                "Formato temporal desconhecido: {clean}"
            )))
        }
    }

    pub fn to_timestamp_utc(&self) -> i64 {
        match self {
            EventTime::DateTimeUtc(ts) => *ts,
            EventTime::DateOnly { year, month, day } => {
                utc_parts_to_timestamp(*year, *month, *day, 0, 0, 0)
            }
        }
    }
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
        let mut lines = Vec::new();
        lines.push("BEGIN:VCALENDAR".to_string());
        lines.push("VERSION:2.0".to_string());
        lines.push("PRODID:-//Lauda//PT-BR".to_string());
        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:{}", self.uid));

        let dtstamp = self.updated_at.unwrap_or_else(|| {
            self.created_at
                .unwrap_or_else(|| self.start.to_timestamp_utc())
        });
        lines.push(format!(
            "DTSTAMP:{}",
            EventTime::DateTimeUtc(dtstamp).to_ical_string()
        ));

        lines.push(format!("SUMMARY:{}", self.summary));

        if let Some(desc) = &self.description {
            lines.push(format!("DESCRIPTION:{desc}"));
        }
        if let Some(loc) = &self.location {
            lines.push(format!("LOCATION:{loc}"));
        }

        match &self.start {
            EventTime::DateTimeUtc(_) => {
                lines.push(format!("DTSTART:{}", self.start.to_ical_string()))
            }
            EventTime::DateOnly { .. } => lines.push(format!(
                "DTSTART;VALUE=DATE:{}",
                self.start.to_ical_string()
            )),
        }

        match &self.end {
            EventTime::DateTimeUtc(_) => lines.push(format!("DTEND:{}", self.end.to_ical_string())),
            EventTime::DateOnly { .. } => {
                lines.push(format!("DTEND;VALUE=DATE:{}", self.end.to_ical_string()))
            }
        }

        if let Some(rrule) = &self.rrule {
            lines.push(format!("RRULE:{}", rrule.to_rrule_string()));
        }

        lines.push(format!("STATUS:{}", self.status.to_ical_str()));

        if let Some(org) = &self.organizer {
            let cn = org.name.as_deref().unwrap_or(&org.email);
            lines.push(format!("ORGANIZER;CN={cn}:mailto:{}", org.email));
        }

        for att in &self.attendees {
            let cn = att.name.as_deref().unwrap_or(&att.email);
            lines.push(format!(
                "ATTENDEE;PARTSTAT={};CN={cn}:mailto:{}",
                att.status.to_ical_str(),
                att.email
            ));
        }

        lines.push(format!("SEQUENCE:{}", self.sequence));
        lines.push("END:VEVENT".to_string());
        lines.push("END:VCALENDAR".to_string());

        lines.join("\r\n") + "\r\n"
    }

    pub fn from_ical(raw: &str) -> Result<Self, CalendarError> {
        // Desdobramento de linhas RFC 5545 (§3.1)
        let mut unfolded = String::new();
        for line in raw.lines() {
            if line.starts_with(' ') || line.starts_with('\t') {
                unfolded.push_str(&line[1..]);
            } else {
                if !unfolded.is_empty() {
                    unfolded.push('\n');
                }
                unfolded.push_str(line.trim_end_matches('\r'));
            }
        }

        let mut uid = String::new();
        let mut summary = String::new();
        let mut description = None;
        let mut location = None;
        let mut start = None;
        let mut end = None;
        let mut is_all_day = false;
        let mut rrule = None;
        let mut status = EventStatus::Confirmed;
        let mut organizer = None;
        let mut attendees = Vec::new();
        let mut sequence = 0;
        let mut in_vevent = false;

        for line in unfolded.lines() {
            let line = line.trim();
            if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
                in_vevent = true;
                continue;
            }
            if line.eq_ignore_ascii_case("END:VEVENT") {
                break;
            }
            if !in_vevent {
                continue;
            }

            let (full_key, val) = match line.split_once(':') {
                Some(pair) => pair,
                None => continue,
            };

            let prop_name = full_key
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_uppercase();

            match prop_name.as_str() {
                "UID" => uid = val.trim().to_string(),
                "SUMMARY" => summary = val.trim().to_string(),
                "DESCRIPTION" => description = Some(val.trim().to_string()),
                "LOCATION" => location = Some(val.trim().to_string()),
                "DTSTART" => {
                    if full_key.to_uppercase().contains("VALUE=DATE") {
                        is_all_day = true;
                    }
                    start = Some(EventTime::from_ical_string(val)?);
                }
                "DTEND" => {
                    end = Some(EventTime::from_ical_string(val)?);
                }
                "RRULE" => {
                    rrule = Some(RecurrenceRule::from_rrule_string(val)?);
                }
                "STATUS" => {
                    status = EventStatus::from_ical_str(val);
                }
                "SEQUENCE" => {
                    sequence = val.trim().parse::<u32>().unwrap_or(0);
                }
                "ORGANIZER" => {
                    let mut name = None;
                    for param in full_key.split(';') {
                        if let Some((pk, pv)) = param.split_once('=')
                            && pk.trim().eq_ignore_ascii_case("CN")
                        {
                            name = Some(pv.trim().to_string());
                        }
                    }
                    let email = val
                        .trim()
                        .strip_prefix("mailto:")
                        .unwrap_or(val)
                        .trim()
                        .to_string();
                    organizer = Some(EventAttendee {
                        email,
                        name,
                        status: RsvpStatus::Accepted,
                    });
                }
                "ATTENDEE" => {
                    let mut name = None;
                    let mut partstat = RsvpStatus::NeedsAction;
                    for param in full_key.split(';') {
                        if let Some((pk, pv)) = param.split_once('=') {
                            if pk.trim().eq_ignore_ascii_case("CN") {
                                name = Some(pv.trim().to_string());
                            } else if pk.trim().eq_ignore_ascii_case("PARTSTAT") {
                                partstat = RsvpStatus::from_ical_str(pv);
                            }
                        }
                    }
                    let email = val
                        .trim()
                        .strip_prefix("mailto:")
                        .unwrap_or(val)
                        .trim()
                        .to_string();
                    attendees.push(EventAttendee {
                        email,
                        name,
                        status: partstat,
                    });
                }
                _ => {}
            }
        }

        if uid.is_empty() {
            return Err(CalendarError::ParseError("VEVENT sem UID".to_string()));
        }

        let start =
            start.ok_or_else(|| CalendarError::ParseError("VEVENT sem DTSTART".to_string()))?;
        let end = end.unwrap_or_else(|| start.clone());

        Ok(CalendarEvent {
            uid,
            summary,
            description,
            location,
            start,
            end,
            is_all_day,
            rrule,
            status,
            organizer,
            attendees,
            sequence,
            created_at: None,
            updated_at: None,
        })
    }
}

/// Gera o payload iTIP (RFC 5546 / RFC 5545) para convites, respostas ou cancelamento de eventos.
pub fn generate_itip_payload(
    event: &CalendarEvent,
    method: ItipMethod,
    organizer: &EventAttendee,
) -> String {
    let mut lines = Vec::new();
    lines.push("BEGIN:VCALENDAR".to_string());
    lines.push("VERSION:2.0".to_string());
    lines.push("PRODID:-//Lauda//PT-BR".to_string());
    lines.push(format!("METHOD:{}", method.to_ical_str()));
    lines.push("BEGIN:VEVENT".to_string());
    lines.push(format!("UID:{}", event.uid));

    let dtstamp = event.updated_at.unwrap_or_else(|| {
        event
            .created_at
            .unwrap_or_else(|| event.start.to_timestamp_utc())
    });
    lines.push(format!(
        "DTSTAMP:{}",
        EventTime::DateTimeUtc(dtstamp).to_ical_string()
    ));

    lines.push(format!("SUMMARY:{}", event.summary));

    if let Some(desc) = &event.description {
        lines.push(format!("DESCRIPTION:{desc}"));
    }
    if let Some(loc) = &event.location {
        lines.push(format!("LOCATION:{loc}"));
    }

    match &event.start {
        EventTime::DateTimeUtc(_) => {
            lines.push(format!("DTSTART:{}", event.start.to_ical_string()))
        }
        EventTime::DateOnly { .. } => lines.push(format!(
            "DTSTART;VALUE=DATE:{}",
            event.start.to_ical_string()
        )),
    }

    match &event.end {
        EventTime::DateTimeUtc(_) => lines.push(format!("DTEND:{}", event.end.to_ical_string())),
        EventTime::DateOnly { .. } => {
            lines.push(format!("DTEND;VALUE=DATE:{}", event.end.to_ical_string()))
        }
    }

    if let Some(rrule) = &event.rrule {
        lines.push(format!("RRULE:{}", rrule.to_rrule_string()));
    }

    lines.push(format!("STATUS:{}", event.status.to_ical_str()));

    match &organizer.name {
        Some(name) => lines.push(format!(
            "ORGANIZER;CN=\"{name}\":mailto:{}",
            organizer.email
        )),
        None => lines.push(format!("ORGANIZER:mailto:{}", organizer.email)),
    }

    for att in &event.attendees {
        let cn_str = match &att.name {
            Some(name) => format!(";CN=\"{name}\""),
            None => String::new(),
        };
        let rsvp_str = if method == ItipMethod::Request {
            ";RSVP=TRUE"
        } else {
            ""
        };
        lines.push(format!(
            "ATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT={}{rsvp_str}{cn_str}:mailto:{}",
            att.status.to_ical_str(),
            att.email
        ));
    }

    lines.push(format!("SEQUENCE:{}", event.sequence));
    lines.push("END:VEVENT".to_string());
    lines.push("END:VCALENDAR".to_string());

    lines.join("\r\n") + "\r\n"
}

/// Formata a data e horário do evento para exibição amigável em chips na interface visual.
pub fn format_event_datetime_chip(event: &CalendarEvent) -> String {
    match (&event.start, &event.end) {
        (
            EventTime::DateOnly {
                year: _,
                month,
                day,
            },
            EventTime::DateOnly {
                year: _,
                month: end_m,
                day: end_d,
            },
        ) => {
            if month == end_m && day == end_d {
                format!("{day:02}/{month:02} (Dia inteiro)")
            } else {
                format!("{day:02}/{month:02} até {end_d:02}/{end_m:02} (Dia inteiro)")
            }
        }
        (
            EventTime::DateOnly {
                year: _,
                month,
                day,
            },
            _,
        ) => {
            format!("{day:02}/{month:02} (Dia inteiro)")
        }
        (EventTime::DateTimeUtc(start_ts), EventTime::DateTimeUtc(end_ts)) => {
            let (_, m1, d1, h1, min1, _) = timestamp_to_utc_parts(*start_ts);
            let (_, m2, d2, h2, min2, _) = timestamp_to_utc_parts(*end_ts);
            if event.is_all_day {
                format!("{d1:02}/{m1:02} (Dia inteiro)")
            } else if m1 == m2 && d1 == d2 {
                format!("{d1:02}/{m1:02} · {h1:02}:{min1:02} - {h2:02}:{min2:02}")
            } else {
                format!("{d1:02}/{m1:02} {h1:02}:{min1:02} → {d2:02}/{m2:02} {h2:02}:{min2:02}")
            }
        }
        (EventTime::DateTimeUtc(start_ts), _) => {
            let (_, m1, d1, h1, min1, _) = timestamp_to_utc_parts(*start_ts);
            if event.is_all_day {
                format!("{d1:02}/{m1:02} (Dia inteiro)")
            } else {
                format!("{d1:02}/{m1:02} às {h1:02}:{min1:02}")
            }
        }
    }
}

/// Formata a data de calendário processada do intent para exibição em chip (DD/MM/YYYY).
pub fn format_intent_date_chip(intent: &ParsedCalendarIntent) -> String {
    match (&intent.start, &intent.end) {
        (
            EventTime::DateOnly {
                year: y1,
                month: m1,
                day: d1,
            },
            EventTime::DateOnly {
                year: y2,
                month: m2,
                day: d2,
            },
        ) => {
            if y1 == y2 && m1 == m2 && d1 == d2 {
                format!("{d1:02}/{m1:02}/{y1}")
            } else {
                format!("{d1:02}/{m1:02}/{y1} → {d2:02}/{m2:02}/{y2}")
            }
        }
        (EventTime::DateOnly { year, month, day }, _) => {
            format!("{day:02}/{month:02}/{year}")
        }
        (EventTime::DateTimeUtc(start_ts), EventTime::DateTimeUtc(end_ts)) => {
            let (y1, m1, d1, _, _, _) = timestamp_to_utc_parts(*start_ts);
            let (y2, m2, d2, _, _, _) = timestamp_to_utc_parts(*end_ts);
            if y1 == y2 && m1 == m2 && d1 == d2 {
                format!("{d1:02}/{m1:02}/{y1}")
            } else {
                format!("{d1:02}/{m1:02}/{y1} → {d2:02}/{m2:02}/{y2}")
            }
        }
        (EventTime::DateTimeUtc(start_ts), _) => {
            let (y1, m1, d1, _, _, _) = timestamp_to_utc_parts(*start_ts);
            format!("{d1:02}/{m1:02}/{y1}")
        }
    }
}

/// Formata o horário processado do intent para exibição em chip (HH:MM - HH:MM ou Dia inteiro).
pub fn format_intent_time_chip(intent: &ParsedCalendarIntent) -> String {
    if intent.is_all_day {
        return "Dia inteiro".to_string();
    }
    match (&intent.start, &intent.end) {
        (EventTime::DateTimeUtc(start_ts), EventTime::DateTimeUtc(end_ts)) => {
            let (y1, m1, d1, h1, min1, _) = timestamp_to_utc_parts(*start_ts);
            let (y2, m2, d2, h2, min2, _) = timestamp_to_utc_parts(*end_ts);
            if y1 == y2 && m1 == m2 && d1 == d2 {
                format!("{h1:02}:{min1:02} - {h2:02}:{min2:02}")
            } else {
                format!("{h1:02}:{min1:02} → {h2:02}:{min2:02}")
            }
        }
        (EventTime::DateTimeUtc(start_ts), _) => {
            let (_, _, _, h1, min1, _) = timestamp_to_utc_parts(*start_ts);
            format!("{h1:02}:{min1:02}")
        }
        _ => "Dia inteiro".to_string(),
    }
}

/// Formata a data e horário completos do intent para exibição unificada em chips.
pub fn format_intent_datetime_chip(intent: &ParsedCalendarIntent) -> String {
    let date_str = format_intent_date_chip(intent);
    if intent.is_all_day {
        return format!("{date_str} (Dia inteiro)");
    }
    match (&intent.start, &intent.end) {
        (EventTime::DateTimeUtc(start_ts), EventTime::DateTimeUtc(end_ts)) => {
            let (y1, m1, d1, h1, min1, _) = timestamp_to_utc_parts(*start_ts);
            let (y2, m2, d2, h2, min2, _) = timestamp_to_utc_parts(*end_ts);
            if y1 == y2 && m1 == m2 && d1 == d2 {
                format!("{d1:02}/{m1:02}/{y1} · {h1:02}:{min1:02} - {h2:02}:{min2:02}")
            } else {
                format!(
                    "{d1:02}/{m1:02}/{y1} {h1:02}:{min1:02} → {d2:02}/{m2:02}/{y2} {h2:02}:{min2:02}"
                )
            }
        }
        (EventTime::DateTimeUtc(start_ts), _) => {
            let (y1, m1, d1, h1, min1, _) = timestamp_to_utc_parts(*start_ts);
            format!("{d1:02}/{m1:02}/{y1} às {h1:02}:{min1:02}")
        }
        _ => format!("{date_str} (Dia inteiro)"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub attendees: Vec<EventAttendee>,
}

pub struct CalendarIntentParser;

impl CalendarIntentParser {
    pub fn parse(
        input: &str,
        ref_time: &CalendarReferenceTime,
    ) -> Result<ParsedCalendarIntent, CalendarError> {
        let (extracted_input, literals) = extract_quotes(input);
        let trimmed = extracted_input.trim();
        if trimmed.is_empty() {
            return Err(CalendarError::ParseError("Entrada vazia".to_string()));
        }

        let mut location = None;
        let mut rrule = None;
        let mut start_hour = None;
        let mut start_min = 0;
        let mut end_hour = None;
        let mut end_min = 0;
        let mut day_offset: i64 = 0;
        let mut specific_date: Option<(i32, u32, u32)> = None;
        let mut weekday_target: Option<Weekday> = None;
        let mut is_all_day = true;

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let mut consumed_indices = Vec::new();

        let weekdays_pt = [
            ("segunda", Weekday::Monday),
            ("terça", Weekday::Tuesday),
            ("terca", Weekday::Tuesday),
            ("quarta", Weekday::Wednesday),
            ("quinta", Weekday::Thursday),
            ("sexta", Weekday::Friday),
            ("sábado", Weekday::Saturday),
            ("sabado", Weekday::Saturday),
            ("domingo", Weekday::Sunday),
        ];

        // Pass 0: Localização protegida por aspas ou por substantivo de logradouro
        for i in 0..tokens.len() {
            if consumed_indices.contains(&i) {
                continue;
            }
            let t_low = tokens[i].to_lowercase();
            if (t_low == "no" || t_low == "na" || t_low == "em" || t_low == "at")
                && i + 1 < tokens.len()
            {
                let next_low = tokens[i + 1].to_lowercase();
                if next_low.starts_with("__lit_") {
                    location = Some(restore_literals(tokens[i + 1], &literals));
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    continue;
                }
                let venue_keywords = [
                    "clube",
                    "rua",
                    "avenida",
                    "av",
                    "hospital",
                    "restaurante",
                    "bar",
                    "edifício",
                    "ed",
                    "praça",
                    "parque",
                ];
                if venue_keywords.iter().any(|&vk| next_low == vk) {
                    let mut loc_tokens = Vec::new();
                    let mut j = i + 1;
                    while j < tokens.len() && !consumed_indices.contains(&j) {
                        let w_low = tokens[j].to_lowercase();
                        if w_low == "às"
                            || w_low == "as"
                            || w_low == "das"
                            || w_low.starts_with("amanh")
                            || w_low == "hoje"
                            || w_low == "findi"
                        {
                            break;
                        }
                        loc_tokens.push(tokens[j]);
                        consumed_indices.push(j);
                        j += 1;
                    }
                    if !loc_tokens.is_empty() {
                        location = Some(restore_literals(&loc_tokens.join(" "), &literals));
                        consumed_indices.push(i);
                    }
                }
            }
        }

        // Pass 1: Recorrências e padrões temporais
        for i in 0..tokens.len() {
            if consumed_indices.contains(&i) {
                continue;
            }
            let t_low = tokens[i].to_lowercase();

            // "a cada 15 dias"
            if t_low == "a"
                && i + 3 < tokens.len()
                && tokens[i + 1].to_lowercase() == "cada"
                && tokens[i + 2] == "15"
                && tokens[i + 3].to_lowercase().starts_with("dia")
            {
                rrule = Some(RecurrenceRule {
                    freq: RecurrenceFreq::Weekly,
                    interval: 2,
                    count: None,
                    until: None,
                    by_day: Vec::new(),
                });
                consumed_indices.extend_from_slice(&[i, i + 1, i + 2, i + 3]);
                continue;
            }

            // "depois de amanhã" / "depois de amanha"
            if t_low == "depois"
                && i + 2 < tokens.len()
                && tokens[i + 1].to_lowercase() == "de"
                && (tokens[i + 2].to_lowercase().starts_with("amanh")
                    || tokens[i + 2].to_lowercase().starts_with("amanha"))
            {
                day_offset = 2;
                consumed_indices.extend_from_slice(&[i, i + 1, i + 2]);
                continue;
            }

            // "todo dia", "toda semana", "todo mês", "todo mes", "todo ano"
            if (t_low == "todo" || t_low == "toda") && i + 1 < tokens.len() {
                let next_low = tokens[i + 1].to_lowercase();
                if next_low == "dia" {
                    rrule = Some(RecurrenceRule {
                        freq: RecurrenceFreq::Daily,
                        interval: 1,
                        count: None,
                        until: None,
                        by_day: Vec::new(),
                    });
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    continue;
                } else if next_low == "semana" {
                    rrule = Some(RecurrenceRule {
                        freq: RecurrenceFreq::Weekly,
                        interval: 1,
                        count: None,
                        until: None,
                        by_day: Vec::new(),
                    });
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    continue;
                } else if next_low.starts_with("mês") || next_low.starts_with("mes") {
                    rrule = Some(RecurrenceRule {
                        freq: RecurrenceFreq::Monthly,
                        interval: 1,
                        count: None,
                        until: None,
                        by_day: Vec::new(),
                    });
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    continue;
                } else if next_low == "ano" {
                    rrule = Some(RecurrenceRule {
                        freq: RecurrenceFreq::Yearly,
                        interval: 1,
                        count: None,
                        until: None,
                        by_day: Vec::new(),
                    });
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    continue;
                }
            }

            // "toda terça", "todas as quartas", etc.
            for (name, wd) in &weekdays_pt {
                if (t_low == "toda" || t_low == "todo") && i + 1 < tokens.len() {
                    let next_low = tokens[i + 1].to_lowercase();
                    if next_low.starts_with(name) {
                        rrule = Some(RecurrenceRule {
                            freq: RecurrenceFreq::Weekly,
                            interval: 1,
                            count: None,
                            until: None,
                            by_day: vec![*wd],
                        });
                        weekday_target = Some(*wd);
                        consumed_indices.extend_from_slice(&[i, i + 1]);
                        break;
                    }
                } else if t_low == "todas" && i + 2 < tokens.len() {
                    let p1 = tokens[i + 1].to_lowercase();
                    let p2 = tokens[i + 2].to_lowercase();
                    if p1 == "as" && p2.starts_with(name) {
                        rrule = Some(RecurrenceRule {
                            freq: RecurrenceFreq::Weekly,
                            interval: 1,
                            count: None,
                            until: None,
                            by_day: vec![*wd],
                        });
                        weekday_target = Some(*wd);
                        consumed_indices.extend_from_slice(&[i, i + 1, i + 2]);
                        break;
                    }
                }
            }

            // "dia 5", "dia 15"
            if t_low == "dia"
                && i + 1 < tokens.len()
                && let Ok(day_val) = tokens[i + 1].parse::<u32>()
                && (1..=31).contains(&day_val)
            {
                let is_date_of_month = i + 2 < tokens.len()
                    && tokens[i + 2].to_lowercase() == "de"
                    && i + 3 < tokens.len()
                    && parse_month_token(tokens[i + 3]).is_some();
                if !is_date_of_month {
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    continue;
                }
            }

            // Palavras únicas de data/semana
            if !consumed_indices.contains(&i) {
                if t_low.starts_with("amanh") || t_low == "tomorrow" {
                    day_offset = 1;
                    consumed_indices.push(i);
                } else if t_low == "hoje" || t_low == "today" {
                    day_offset = 0;
                    consumed_indices.push(i);
                } else if t_low == "diariamente" {
                    rrule = Some(RecurrenceRule {
                        freq: RecurrenceFreq::Daily,
                        interval: 1,
                        count: None,
                        until: None,
                        by_day: Vec::new(),
                    });
                    consumed_indices.push(i);
                } else if t_low == "semanalmente" {
                    rrule = Some(RecurrenceRule {
                        freq: RecurrenceFreq::Weekly,
                        interval: 1,
                        count: None,
                        until: None,
                        by_day: Vec::new(),
                    });
                    consumed_indices.push(i);
                } else if t_low == "quinzenalmente" {
                    rrule = Some(RecurrenceRule {
                        freq: RecurrenceFreq::Weekly,
                        interval: 2,
                        count: None,
                        until: None,
                        by_day: Vec::new(),
                    });
                    consumed_indices.push(i);
                } else {
                    for (name, wd) in &weekdays_pt {
                        if t_low.starts_with(name) {
                            weekday_target = Some(*wd);
                            consumed_indices.push(i);
                            break;
                        }
                    }
                }
            }
        }

        // Pass 1b: Feriados Nacionais e Coloquiais ("findi", "natal", etc.)
        for i in 0..tokens.len() {
            if consumed_indices.contains(&i) {
                continue;
            }
            let t_low = tokens[i].to_lowercase();

            // "findi", "fim de semana", "final de semana"
            if t_low == "findi" || t_low == "fim-de-semana" {
                let current_wd_num = ref_time.weekday.day_number();
                let sat_num = Weekday::Saturday.day_number();
                let diff = (sat_num + 7 - current_wd_num) % 7;
                day_offset = if diff == 0 { 7 } else { diff as i64 };
                consumed_indices.push(i);
                if i > 0
                    && (tokens[i - 1].to_lowercase() == "no"
                        || tokens[i - 1].to_lowercase() == "neste")
                    && !consumed_indices.contains(&(i - 1))
                {
                    consumed_indices.push(i - 1);
                }
                continue;
            } else if (t_low == "fim" || t_low == "final")
                && i + 2 < tokens.len()
                && tokens[i + 1].to_lowercase() == "de"
                && tokens[i + 2].to_lowercase() == "semana"
            {
                let current_wd_num = ref_time.weekday.day_number();
                let sat_num = Weekday::Saturday.day_number();
                let diff = (sat_num + 7 - current_wd_num) % 7;
                day_offset = if diff == 0 { 7 } else { diff as i64 };
                consumed_indices.extend_from_slice(&[i, i + 1, i + 2]);
                if i > 0
                    && (tokens[i - 1].to_lowercase() == "no"
                        || tokens[i - 1].to_lowercase() == "neste")
                    && !consumed_indices.contains(&(i - 1))
                {
                    consumed_indices.push(i - 1);
                }
                continue;
            }

            // Feriados
            let is_natal = t_low == "natal";
            let is_ano_novo = t_low == "reveillon"
                || t_low == "réveillon"
                || (t_low == "ano"
                    && i + 1 < tokens.len()
                    && tokens[i + 1].to_lowercase() == "novo");
            let is_independencia = t_low == "independência" || t_low == "independencia";
            let is_tiradentes = t_low == "tiradentes";
            let is_trabalhador = t_low == "trabalhador"
                || (t_low == "dia"
                    && i + 1 < tokens.len()
                    && (tokens[i + 1].to_lowercase() == "do"
                        || tokens[i + 1].to_lowercase() == "de")
                    && i + 2 < tokens.len()
                    && tokens[i + 2].to_lowercase().starts_with("trabalh"));

            if is_natal {
                let year = if ref_time.month > 12 || (ref_time.month == 12 && ref_time.day > 25) {
                    ref_time.year + 1
                } else {
                    ref_time.year
                };
                specific_date = Some((year, 12, 25));
                consumed_indices.push(i);
                if i > 0
                    && (tokens[i - 1].to_lowercase() == "de"
                        || tokens[i - 1].to_lowercase() == "no")
                    && !consumed_indices.contains(&(i - 1))
                {
                    consumed_indices.push(i - 1);
                }
                continue;
            } else if is_ano_novo {
                let year = if ref_time.month > 1 || (ref_time.month == 1 && ref_time.day >= 1) {
                    ref_time.year + 1
                } else {
                    ref_time.year
                };
                specific_date = Some((year, 1, 1));
                if t_low == "ano" {
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                } else {
                    consumed_indices.push(i);
                }
                if i > 0
                    && (tokens[i - 1].to_lowercase() == "de"
                        || tokens[i - 1].to_lowercase() == "no")
                    && !consumed_indices.contains(&(i - 1))
                {
                    consumed_indices.push(i - 1);
                }
                continue;
            } else if is_independencia {
                let year = if (ref_time.month, ref_time.day) > (9, 7) {
                    ref_time.year + 1
                } else {
                    ref_time.year
                };
                specific_date = Some((year, 9, 7));
                consumed_indices.push(i);
                if i > 0
                    && (tokens[i - 1].to_lowercase() == "de"
                        || tokens[i - 1].to_lowercase() == "no")
                    && !consumed_indices.contains(&(i - 1))
                {
                    consumed_indices.push(i - 1);
                }
                continue;
            } else if is_tiradentes {
                let year = if (ref_time.month, ref_time.day) > (4, 21) {
                    ref_time.year + 1
                } else {
                    ref_time.year
                };
                specific_date = Some((year, 4, 21));
                consumed_indices.push(i);
                if i > 0
                    && (tokens[i - 1].to_lowercase() == "de"
                        || tokens[i - 1].to_lowercase() == "no")
                    && !consumed_indices.contains(&(i - 1))
                {
                    consumed_indices.push(i - 1);
                }
                continue;
            } else if is_trabalhador {
                let year = if (ref_time.month, ref_time.day) > (5, 1) {
                    ref_time.year + 1
                } else {
                    ref_time.year
                };
                specific_date = Some((year, 5, 1));
                if t_low == "dia" {
                    consumed_indices.extend_from_slice(&[i, i + 1, i + 2]);
                } else {
                    consumed_indices.push(i);
                }
                if i > 0
                    && (tokens[i - 1].to_lowercase() == "de"
                        || tokens[i - 1].to_lowercase() == "no")
                    && !consumed_indices.contains(&(i - 1))
                {
                    consumed_indices.push(i - 1);
                }
                continue;
            }

            // Data com barra: 1/nov, 01/10, 15/10/2026
            if t_low.contains('/') {
                let parts: Vec<&str> = t_low.split('/').collect();
                if (2..=3).contains(&parts.len())
                    && let Ok(day_val) = parts[0].parse::<u32>()
                    && (1..=31).contains(&day_val)
                    && let Some(month_val) = parse_month_token(parts[1])
                {
                    let year_val = if parts.len() == 3 {
                        if let Ok(y) = parts[2].parse::<i32>() {
                            if y < 100 { 2000 + y } else { y }
                        } else {
                            ref_time.year
                        }
                    } else if (month_val, day_val) < (ref_time.month, ref_time.day) {
                        ref_time.year + 1
                    } else {
                        ref_time.year
                    };
                    specific_date = Some((year_val, month_val, day_val));
                    consumed_indices.push(i);
                    continue;
                }
            }

            // "primeiro de <mês>", "1º de <mês>", "1o de <mês>"
            if t_low == "primeiro" || t_low == "1º" || t_low == "1o" {
                if i + 2 < tokens.len()
                    && tokens[i + 1].to_lowercase() == "de"
                    && let Some(m) = parse_month_token(tokens[i + 2])
                {
                    let y = if (m, 1) < (ref_time.month, ref_time.day) {
                        ref_time.year + 1
                    } else {
                        ref_time.year
                    };
                    specific_date = Some((y, m, 1));
                    consumed_indices.extend_from_slice(&[i, i + 1, i + 2]);
                    continue;
                } else if i + 1 < tokens.len()
                    && let Some(m) = parse_month_token(tokens[i + 1])
                {
                    let y = if (m, 1) < (ref_time.month, ref_time.day) {
                        ref_time.year + 1
                    } else {
                        ref_time.year
                    };
                    specific_date = Some((y, m, 1));
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    continue;
                }
            }

            // "<dia> de <mês>"
            let day_cand = if t_low == "dia" && i + 1 < tokens.len() {
                tokens[i + 1]
                    .parse::<u32>()
                    .ok()
                    .or_else(|| parse_word_number(tokens[i + 1]))
            } else {
                tokens[i]
                    .parse::<u32>()
                    .ok()
                    .or_else(|| parse_word_number(tokens[i]))
            };
            if let Some(d) = day_cand
                && (1..=31).contains(&d)
            {
                let offset = if t_low == "dia" { 2 } else { 1 };
                if i + offset < tokens.len() {
                    let has_de = tokens
                        .get(i + offset)
                        .map(|s| s.to_lowercase() == "de")
                        .unwrap_or(false);
                    let m_token_idx = if has_de { i + offset + 1 } else { i + offset };
                    if m_token_idx < tokens.len()
                        && let Some(m) = parse_month_token(tokens[m_token_idx])
                    {
                        let y = if (m, d) < (ref_time.month, ref_time.day) {
                            ref_time.year + 1
                        } else {
                            ref_time.year
                        };
                        specific_date = Some((y, m, d));
                        for k in i..=m_token_idx {
                            consumed_indices.push(k);
                        }
                        continue;
                    }
                }
            }
        }

        if let Some(target_wd) = weekday_target {
            let current_wd_num = ref_time.weekday.day_number();
            let target_wd_num = target_wd.day_number();
            let diff = (target_wd_num + 7 - current_wd_num) % 7;
            day_offset = if diff == 0 { 7 } else { diff as i64 };
        }

        // Pass 2: Minutos Inversos ("15 pras 2", "20 pra meia noite", etc.)
        for i in 0..tokens.len() {
            if consumed_indices.contains(&i) {
                continue;
            }
            let min_cand = tokens[i]
                .parse::<u32>()
                .ok()
                .or_else(|| parse_word_number(tokens[i]));
            if let Some(m) = min_cand
                && (1..=59).contains(&m)
                && i + 1 < tokens.len()
            {
                let p_low = tokens[i + 1].to_lowercase();
                let is_prep = p_low == "pra"
                    || p_low == "pras"
                    || p_low == "pro"
                    || p_low == "pros"
                    || p_low == "para";
                if is_prep {
                    let mut target_idx = i + 2;
                    if p_low == "para"
                        && target_idx < tokens.len()
                        && (tokens[target_idx].to_lowercase() == "as"
                            || tokens[target_idx].to_lowercase() == "os"
                            || tokens[target_idx].to_lowercase() == "a")
                    {
                        target_idx += 1;
                    }
                    if target_idx < tokens.len() {
                        let tgt_low = tokens[target_idx].to_lowercase();
                        let mut target_hour = None;
                        let mut advance = 1;

                        if tgt_low == "meia"
                            && target_idx + 1 < tokens.len()
                            && tokens[target_idx + 1].to_lowercase() == "noite"
                        {
                            target_hour = Some(0);
                            advance = 2;
                        } else if tgt_low == "meia-noite" || tgt_low == "meianoite" {
                            target_hour = Some(0);
                            advance = 1;
                        } else if tgt_low == "meio"
                            && target_idx + 1 < tokens.len()
                            && tokens[target_idx + 1].to_lowercase() == "dia"
                        {
                            target_hour = Some(12);
                            advance = 2;
                        } else if tgt_low == "meio-dia" || tgt_low == "meiodia" {
                            target_hour = Some(12);
                            advance = 1;
                        } else if let Some(th) = tokens[target_idx]
                            .parse::<u32>()
                            .ok()
                            .or_else(|| parse_word_number(tokens[target_idx]))
                        {
                            target_hour = Some(th);
                            advance = 1;
                        }

                        if let Some(mut th) = target_hour {
                            let next_pos = target_idx + advance;
                            let mut extra_consumed = 0;
                            let mut turno = None;

                            if next_pos + 1 < tokens.len()
                                && tokens[next_pos].to_lowercase() == "da"
                            {
                                let t_type = tokens[next_pos + 1].to_lowercase();
                                if t_type == "tarde" || t_type == "noite" {
                                    turno = Some("pm");
                                    extra_consumed = 2;
                                } else if t_type == "manhã"
                                    || t_type == "manha"
                                    || t_type == "madrugada"
                                {
                                    turno = Some("am");
                                    extra_consumed = 2;
                                }
                            }

                            match turno {
                                Some("pm") => {
                                    if th < 12 && th != 0 {
                                        th += 12;
                                    }
                                }
                                Some("am") => {}
                                _ => {
                                    if (1..=6).contains(&th) {
                                        th += 12;
                                    }
                                }
                            }

                            let final_min = 60 - m;
                            let final_hour = if th == 0 { 23 } else { th - 1 };

                            start_hour = Some(final_hour);
                            start_min = final_min;
                            is_all_day = false;

                            for k in i..(next_pos + extra_consumed) {
                                consumed_indices.push(k);
                            }
                            break;
                        }
                    }
                }
            }
        }

        // Pass 3: Termos nominais ("meio-dia", "meia-noite")
        if start_hour.is_none() {
            for i in 0..tokens.len() {
                if consumed_indices.contains(&i) {
                    continue;
                }
                let t_low = tokens[i].to_lowercase();
                if t_low == "meio-dia" || t_low == "meiodia" {
                    start_hour = Some(12);
                    start_min = 0;
                    is_all_day = false;
                    consumed_indices.push(i);
                    if i > 0
                        && (tokens[i - 1].to_lowercase() == "ao"
                            || tokens[i - 1].to_lowercase() == "às"
                            || tokens[i - 1].to_lowercase() == "as")
                        && !consumed_indices.contains(&(i - 1))
                    {
                        consumed_indices.push(i - 1);
                    }
                    break;
                } else if t_low == "meio"
                    && i + 1 < tokens.len()
                    && tokens[i + 1].to_lowercase() == "dia"
                {
                    start_hour = Some(12);
                    start_min = 0;
                    is_all_day = false;
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    if i > 0
                        && (tokens[i - 1].to_lowercase() == "ao"
                            || tokens[i - 1].to_lowercase() == "às"
                            || tokens[i - 1].to_lowercase() == "as")
                        && !consumed_indices.contains(&(i - 1))
                    {
                        consumed_indices.push(i - 1);
                    }
                    break;
                } else if t_low == "meia-noite" || t_low == "meianoite" {
                    start_hour = Some(0);
                    start_min = 0;
                    is_all_day = false;
                    consumed_indices.push(i);
                    if i > 0
                        && (tokens[i - 1].to_lowercase() == "à"
                            || tokens[i - 1].to_lowercase() == "a"
                            || tokens[i - 1].to_lowercase() == "às"
                            || tokens[i - 1].to_lowercase() == "as")
                        && !consumed_indices.contains(&(i - 1))
                    {
                        consumed_indices.push(i - 1);
                    }
                    break;
                } else if t_low == "meia"
                    && i + 1 < tokens.len()
                    && tokens[i + 1].to_lowercase() == "noite"
                {
                    start_hour = Some(0);
                    start_min = 0;
                    is_all_day = false;
                    consumed_indices.extend_from_slice(&[i, i + 1]);
                    if i > 0
                        && (tokens[i - 1].to_lowercase() == "à"
                            || tokens[i - 1].to_lowercase() == "a"
                            || tokens[i - 1].to_lowercase() == "às"
                            || tokens[i - 1].to_lowercase() == "as")
                        && !consumed_indices.contains(&(i - 1))
                    {
                        consumed_indices.push(i - 1);
                    }
                    break;
                }
            }
        }

        // Pass 4: Horários compostos por extenso ("duas e meia da tarde", etc.)
        if start_hour.is_none() {
            for i in 0..tokens.len() {
                if consumed_indices.contains(&i) {
                    continue;
                }
                let t_low = tokens[i].to_lowercase();
                let (hour_token_idx, has_prefix) =
                    if (t_low == "às" || t_low == "as") && i + 1 < tokens.len() {
                        (i + 1, true)
                    } else {
                        (i, false)
                    };

                if let Some(mut h) = tokens[hour_token_idx]
                    .parse::<u32>()
                    .ok()
                    .or_else(|| parse_word_number(tokens[hour_token_idx]))
                    && h <= 24
                {
                    let mut m = 0;
                    let mut advance = if has_prefix { 2 } else { 1 };
                    let mut matched_time = false;

                    // "e meia", "e 30", "e quinze"
                    let cur = i + advance;
                    if cur + 1 < tokens.len() && tokens[cur].to_lowercase() == "e" {
                        let m_tok = tokens[cur + 1].to_lowercase();
                        if m_tok == "meia" {
                            m = 30;
                            advance += 2;
                            matched_time = true;
                        } else if let Some(m_val) = m_tok
                            .parse::<u32>()
                            .ok()
                            .or_else(|| parse_word_number(&m_tok))
                        {
                            m = m_val;
                            advance += 2;
                            matched_time = true;
                        }
                    }

                    // "horas", "hora", "h"
                    let cur_h = i + advance;
                    if cur_h < tokens.len() {
                        let h_tok = tokens[cur_h].to_lowercase();
                        if h_tok == "horas" || h_tok == "hora" || h_tok == "h" {
                            advance += 1;
                            matched_time = true;
                        }
                    }

                    // "da tarde", "da noite", "da manhã"
                    let cur_turno = i + advance;
                    let mut turno = None;
                    if cur_turno + 1 < tokens.len() && tokens[cur_turno].to_lowercase() == "da" {
                        let t_type = tokens[cur_turno + 1].to_lowercase();
                        if t_type == "tarde" || t_type == "noite" {
                            turno = Some("pm");
                            advance += 2;
                            matched_time = true;
                        } else if t_type == "manhã" || t_type == "manha" || t_type == "madrugada" {
                            turno = Some("am");
                            advance += 2;
                            matched_time = true;
                        }
                    }

                    if has_prefix || matched_time {
                        match turno {
                            Some("pm") => {
                                if h < 12 {
                                    h += 12;
                                }
                            }
                            Some("am") => {}
                            _ => {
                                if (1..=6).contains(&h) {
                                    h += 12;
                                }
                            }
                        }
                        start_hour = Some(h);
                        start_min = m;
                        is_all_day = false;
                        for k in i..(i + advance) {
                            consumed_indices.push(k);
                        }
                        break;
                    }
                }
            }
        }

        // Pass 5a: Faixa horária "das ... às ..."
        for i in 0..tokens.len() {
            if consumed_indices.contains(&i) {
                continue;
            }
            let t_lower = tokens[i].to_lowercase();
            if t_lower == "das" && i + 3 < tokens.len() {
                let sep = tokens[i + 2].to_lowercase();
                if (sep == "às" || sep == "as" || sep == "a" || sep == "até" || sep == "ate")
                    && let Some((h1, m1)) = parse_time_token(tokens[i + 1])
                    && let Some((h2, m2)) = parse_time_token(tokens[i + 3])
                {
                    start_hour = Some(h1);
                    start_min = m1;
                    end_hour = Some(h2);
                    end_min = m2;
                    is_all_day = false;
                    consumed_indices.extend_from_slice(&[i, i + 1, i + 2, i + 3]);
                    break;
                }
            }
        }

        // Pass 5b: Horário pontual "às X" ou "as X" ou token direto "Xh"
        if start_hour.is_none() {
            for i in 0..tokens.len() {
                if consumed_indices.contains(&i) {
                    continue;
                }
                let t_lower = tokens[i].to_lowercase();
                if (t_lower == "às" || t_lower == "as") && i + 1 < tokens.len() {
                    if let Some((h, m)) = parse_time_token(tokens[i + 1]) {
                        start_hour = Some(h);
                        start_min = m;
                        is_all_day = false;
                        consumed_indices.extend_from_slice(&[i, i + 1]);
                        break;
                    }
                } else if let Some((h, m)) = parse_time_token(tokens[i]) {
                    start_hour = Some(h);
                    start_min = m;
                    is_all_day = false;
                    consumed_indices.push(i);
                    break;
                }
            }
        }

        // Pass 6: Local "no ...", "na ...", "em ...", "at ..." se ainda não extraído
        if location.is_none() {
            for i in 0..tokens.len() {
                if consumed_indices.contains(&i) {
                    continue;
                }
                let t_lower = tokens[i].to_lowercase();
                if (t_lower == "no" || t_lower == "na" || t_lower == "em" || t_lower == "at")
                    && i + 1 < tokens.len()
                {
                    let mut loc_tokens = Vec::new();
                    let mut j = i + 1;
                    while j < tokens.len() && !consumed_indices.contains(&j) {
                        let next_low = tokens[j].to_lowercase();
                        if next_low == "às"
                            || next_low == "as"
                            || next_low == "das"
                            || next_low.starts_with("amanh")
                        {
                            break;
                        }
                        loc_tokens.push(tokens[j]);
                        consumed_indices.push(j);
                        j += 1;
                    }
                    if !loc_tokens.is_empty() {
                        location = Some(restore_literals(&loc_tokens.join(" "), &literals));
                        consumed_indices.push(i);
                    }
                }
            }
        }

        // Pass 7: Extração de participantes por conectivos de companhia
        let mut attendees = Vec::new();
        for i in 0..tokens.len() {
            if consumed_indices.contains(&i) {
                continue;
            }
            let t_low = tokens[i].to_lowercase();
            let mut connective_len = 0;

            if t_low == "com" {
                connective_len = 1;
            } else if t_low == "junto"
                && i + 1 < tokens.len()
                && (tokens[i + 1].to_lowercase() == "a"
                    || tokens[i + 1].to_lowercase() == "de"
                    || tokens[i + 1].to_lowercase() == "com")
            {
                connective_len = 2;
            } else if (t_low == "em" || t_low == "na")
                && i + 2 < tokens.len()
                && tokens[i + 1].to_lowercase() == "companhia"
                && tokens[i + 2].to_lowercase() == "de"
            {
                connective_len = 3;
            }

            if connective_len > 0 {
                let start_idx = i + connective_len;
                let mut attendee_tokens = Vec::new();
                let mut j = start_idx;

                while j < tokens.len()
                    && !consumed_indices.contains(&j)
                    && !is_intent_delimiter(tokens[j])
                {
                    attendee_tokens.push(tokens[j]);
                    consumed_indices.push(j);
                    j += 1;
                }

                if !attendee_tokens.is_empty() {
                    for k in i..(i + connective_len) {
                        consumed_indices.push(k);
                    }
                    let joined = attendee_tokens.join(" ");
                    let restored = restore_literals(&joined, &literals);
                    let normalized = restored.replace(" e ", ", ");
                    for part in normalized.split(',') {
                        let name_or_email = part.trim();
                        if name_or_email.is_empty() {
                            continue;
                        }
                        if let Some(start) = name_or_email.find('<')
                            && let Some(end) = name_or_email.rfind('>')
                            && start < end
                        {
                            let name_part = name_or_email[..start].trim().trim_matches('"').trim();
                            let email_part = name_or_email[(start + 1)..end].trim();
                            attendees.push(EventAttendee {
                                email: email_part.to_string(),
                                name: if name_part.is_empty() {
                                    None
                                } else {
                                    Some(name_part.to_string())
                                },
                                status: RsvpStatus::NeedsAction,
                            });
                        } else if name_or_email.contains('@') {
                            attendees.push(EventAttendee {
                                email: name_or_email.to_string(),
                                name: None,
                                status: RsvpStatus::NeedsAction,
                            });
                        } else {
                            attendees.push(EventAttendee {
                                email: String::new(),
                                name: Some(name_or_email.to_string()),
                                status: RsvpStatus::NeedsAction,
                            });
                        }
                    }
                }
            }
        }

        // Construção do Summary limpo com todos os tokens não consumidos
        let mut summary_tokens = Vec::new();
        for (idx, token) in tokens.iter().enumerate() {
            if consumed_indices.contains(&idx) {
                continue;
            }
            summary_tokens.push(*token);
        }

        let summary_raw = if summary_tokens.is_empty() {
            "Novo Evento".to_string()
        } else {
            summary_tokens.join(" ")
        };
        let summary = restore_literals(&summary_raw, &literals);
        if let Some(loc) = location.as_mut() {
            *loc = restore_literals(loc, &literals);
        }

        // Cálculo dos timestamps de início e término
        let base_days = if let Some((sy, sm, sd)) = specific_date {
            civil_to_days(sy, sm, sd)
        } else {
            civil_to_days(ref_time.year, ref_time.month, ref_time.day) + day_offset
        };
        let (calc_y, calc_m, calc_d) = days_to_civil(base_days);

        let (start, end) = if is_all_day {
            (
                EventTime::DateOnly {
                    year: calc_y,
                    month: calc_m,
                    day: calc_d,
                },
                EventTime::DateOnly {
                    year: calc_y,
                    month: calc_m,
                    day: calc_d,
                },
            )
        } else {
            let sh = start_hour.unwrap_or(9);
            let sm = start_min;
            let (eh, em, day_span) = if let Some(eh) = end_hour {
                if eh < sh {
                    // Rola para o próximo dia (ex: 19h às 07h)
                    (eh, end_min, 1)
                } else {
                    (eh, end_min, 0)
                }
            } else {
                // Duração padrão: 1 hora
                if sh + 1 >= 24 {
                    (sh + 1 - 24, sm, 1)
                } else {
                    (sh + 1, sm, 0)
                }
            };

            let start_ts = utc_parts_to_timestamp(calc_y, calc_m, calc_d, sh, sm, 0);
            let (end_y, end_m, end_d) = days_to_civil(base_days + day_span);
            let end_ts = utc_parts_to_timestamp(end_y, end_m, end_d, eh, em, 0);

            (
                EventTime::DateTimeUtc(start_ts),
                EventTime::DateTimeUtc(end_ts),
            )
        };

        Ok(ParsedCalendarIntent {
            summary,
            start,
            end,
            is_all_day,
            location,
            rrule,
            attendees,
        })
    }
}

fn is_intent_delimiter(token: &str) -> bool {
    let t = token.trim().to_lowercase();
    let clean = t.trim_matches(|c: char| !c.is_alphanumeric());
    if clean.is_empty() {
        return false;
    }
    matches!(
        clean,
        "às" | "as"
            | "das"
            | "à"
            | "a"
            | "até"
            | "ate"
            | "amanhã"
            | "amanha"
            | "hoje"
            | "depois"
            | "findi"
            | "fim"
            | "final"
            | "no"
            | "na"
            | "em"
            | "at"
            | "segunda"
            | "terça"
            | "terca"
            | "quarta"
            | "quinta"
            | "sexta"
            | "sábado"
            | "sabado"
            | "domingo"
            | "natal"
            | "reveillon"
            | "réveillon"
            | "tiradentes"
            | "trabalhador"
            | "independência"
            | "independencia"
            | "dia"
            | "todo"
            | "toda"
            | "todas"
    ) || parse_time_token(token).is_some()
        || parse_month_token(token).is_some()
        || clean.contains('/')
}

fn extract_quotes(input: &str) -> (String, Vec<String>) {
    let mut literals = Vec::new();
    let mut out = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '"' || c == '\'' {
            let quote = c;
            let mut lit = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == quote {
                    closed = true;
                    break;
                }
                lit.push(inner);
            }
            if closed && !lit.is_empty() {
                let placeholder = format!("__LIT_{}__", literals.len());
                literals.push(lit);
                out.push_str(&placeholder);
            } else {
                out.push(quote);
                out.push_str(&lit);
            }
        } else {
            out.push(c);
        }
    }
    (out, literals)
}

fn restore_literals(text: &str, literals: &[String]) -> String {
    let mut res = text.to_string();
    for (i, lit) in literals.iter().enumerate() {
        let placeholder = format!("__LIT_{i}__");
        res = res.replace(&placeholder, lit);
    }
    res
}

fn parse_word_number(word: &str) -> Option<u32> {
    let w = word.trim().to_lowercase();
    let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
    match clean {
        "um" | "uma" | "1" => Some(1),
        "dois" | "duas" | "2" => Some(2),
        "três" | "tres" | "3" => Some(3),
        "quatro" | "4" => Some(4),
        "cinco" | "5" => Some(5),
        "seis" | "6" => Some(6),
        "sete" | "7" => Some(7),
        "oito" | "8" => Some(8),
        "nove" | "9" => Some(9),
        "dez" | "10" => Some(10),
        "onze" | "11" => Some(11),
        "doze" | "12" => Some(12),
        "treze" | "13" => Some(13),
        "quatorze" | "catorze" | "14" => Some(14),
        "quinze" | "15" => Some(15),
        "dezesseis" | "dezeseis" | "16" => Some(16),
        "dezessete" | "dezesete" | "17" => Some(17),
        "dezoito" | "18" => Some(18),
        "dezenove" | "19" => Some(19),
        "vinte" | "20" => Some(20),
        "trinta" | "30" => Some(30),
        "quarenta" | "40" => Some(40),
        "cinquenta" | "50" => Some(50),
        _ => None,
    }
}

fn parse_month_token(token: &str) -> Option<u32> {
    let clean = token.trim().to_lowercase();
    let clean = clean.trim_matches(|c: char| !c.is_alphanumeric());
    match clean {
        "janeiro" | "jan" | "1" | "01" => Some(1),
        "fevereiro" | "fev" | "2" | "02" => Some(2),
        "março" | "marco" | "mar" | "3" | "03" => Some(3),
        "abril" | "abr" | "4" | "04" => Some(4),
        "maio" | "mai" | "5" | "05" => Some(5),
        "junho" | "jun" | "6" | "06" => Some(6),
        "julho" | "jul" | "7" | "07" => Some(7),
        "agosto" | "ago" | "8" | "08" => Some(8),
        "setembro" | "set" | "9" | "09" => Some(9),
        "outubro" | "out" | "10" => Some(10),
        "novembro" | "nov" | "11" => Some(11),
        "dezembro" | "dez" | "12" => Some(12),
        _ => None,
    }
}

fn parse_time_token(token: &str) -> Option<(u32, u32)> {
    let mut clean = token.trim().to_lowercase();
    clean = clean
        .trim_end_matches("minutos")
        .trim_end_matches("minuto")
        .trim_end_matches("mins")
        .trim_end_matches("min")
        .trim_end_matches('m')
        .to_string();
    clean = clean
        .trim_end_matches("horas")
        .trim_end_matches("hora")
        .trim_end_matches("hrs")
        .trim_end_matches("hr")
        .trim_end_matches('h')
        .to_string();

    if clean.contains(':') {
        let (h_str, m_str) = clean.split_once(':')?;
        let h = h_str.parse::<u32>().ok()?;
        let m = m_str.parse::<u32>().ok()?;
        if h < 24 && m < 60 {
            return Some((h, m));
        }
    } else if clean.contains('h') {
        let (h_str, m_str) = clean.split_once('h')?;
        let h = h_str.parse::<u32>().ok()?;
        let m = if m_str.is_empty() {
            0
        } else {
            m_str.parse::<u32>().ok()?
        };
        if h < 24 && m < 60 {
            return Some((h, m));
        }
    } else if let Ok(h) = clean.parse::<u32>()
        && h < 24
    {
        let tok_low = token.to_lowercase();
        if tok_low.ends_with('h')
            || tok_low.ends_with("hr")
            || tok_low.ends_with("hrs")
            || tok_low.ends_with("hora")
            || tok_low.ends_with("horas")
        {
            return Some((h, 0));
        }
    }
    None
}
