pub mod ports;
pub mod update;
pub mod view;

pub use ports::{ClientError, MailApiClient};
pub use update::{check_update, UpdateManifest, UpdateStatus};
pub use view::{filter_aliases, parse_markdown_to_html, ViewMode};
