pub mod http;
pub mod in_memory;
pub mod maildir;
pub mod sqlite_contacts;

pub use http::app_router;
pub use in_memory::InMemoryMailBackend;
pub use maildir::MaildirBackend;
pub use sqlite_contacts::SqliteContactsStore;
