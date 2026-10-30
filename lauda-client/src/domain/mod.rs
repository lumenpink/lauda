pub mod cache;
pub mod calendar_cache;
pub mod contacts_cache;
pub mod folders;
pub mod fuzzy;
pub mod outbox;
pub mod ports;
pub mod session;
pub mod sync;
pub mod time;
pub mod update;
pub mod view;

pub use cache::{LocalMessageCache, get_default_cache_dir};
pub use calendar_cache::{LocalCalendarCache, detect_ics_invite};
pub use contacts_cache::{LocalContactsCache, create_local_contact, format_contact_recipient};

pub use folders::{FolderNode, FolderRole, build_folder_tree};
pub use outbox::{OutboxItem, OutboxQueue, OutboxStatus};
pub use ports::{ClientError, MailApiClient};
pub use session::{
    SessionConfig, clear_session, get_default_session_path, load_session, save_session,
};
pub use sync::{ActionType, ActionsQueue, QueuedAction, prefetch_missing_bodies};
pub use time::{async_sleep, current_unix_timestamp};
pub use update::{UpdateManifest, UpdateStatus, check_update, fetch_remote_manifest};
pub use view::{
    MIN_SEARCH_QUERY_LEN, ViewMode, fast_filter_envelopes, filter_aliases, format_search_hint,
    is_search_active, parse_markdown_to_html, should_show_search_hint,
};
