pub mod ports;
pub mod update;
pub mod view;

pub use ports::{ClientError, MailApiClient};
pub use update::{UpdateManifest, UpdateStatus, check_update};
pub use view::{ViewMode, filter_aliases, parse_markdown_to_html};
