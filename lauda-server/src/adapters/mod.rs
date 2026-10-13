pub mod caldav;
pub mod carddav;
pub mod http;
pub mod in_memory;
pub mod maildir;
pub mod sqlite_calendar;
pub mod sqlite_contacts;

pub use caldav::caldav_routes;
pub use carddav::carddav_routes;
pub use http::{app_router, app_router_with_calendar, app_router_with_contacts};
pub use in_memory::InMemoryMailBackend;
pub use maildir::MaildirBackend;
pub use sqlite_calendar::SqliteCalendarStore;
pub use sqlite_contacts::SqliteContactsStore;

