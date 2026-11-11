pub mod adapters;
pub mod auth;
pub mod domain;

pub use adapters::{
    app_router, app_router_with_calendar, app_router_with_contacts, app_router_with_events_tx,
};
