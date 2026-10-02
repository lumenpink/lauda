pub mod http;
pub mod in_memory;

pub use http::app_router;
pub use in_memory::InMemoryMailBackend;
