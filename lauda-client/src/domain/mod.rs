pub mod cache;
pub mod contacts_cache;
pub mod outbox;
pub mod ports;
pub mod session;
pub mod sync;
pub mod time;
pub mod update;
pub mod view;

pub use cache::{LocalMessageCache, get_default_cache_dir};
pub use contacts_cache::LocalContactsCache;
pub use outbox::{OutboxItem, OutboxQueue, OutboxStatus};
pub use ports::{ClientError, MailApiClient};
pub use session::{
    SessionConfig, clear_session, get_default_session_path, load_session, save_session,
};
pub use sync::{ActionType, ActionsQueue, QueuedAction};
pub use time::{async_sleep, current_unix_timestamp};
pub use update::{UpdateManifest, UpdateStatus, check_update, fetch_remote_manifest};
pub use view::{ViewMode, filter_aliases, parse_markdown_to_html};
