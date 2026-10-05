pub mod ports;
pub mod session;
pub mod update;
pub mod view;

pub use ports::{ClientError, MailApiClient};
pub use session::{
    SessionConfig, clear_session, get_default_session_path, load_session, save_session,
};
pub use update::{UpdateManifest, UpdateStatus, check_update};
pub use view::{ViewMode, filter_aliases, parse_markdown_to_html};
