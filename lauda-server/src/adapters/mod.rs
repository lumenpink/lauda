pub mod http;
pub mod in_memory;
pub mod maildir;

pub use http::app_router;
pub use in_memory::InMemoryMailBackend;
pub use maildir::MaildirBackend;
