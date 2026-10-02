pub mod ports;
pub mod view;

pub use ports::{ClientError, MailApiClient};
pub use view::{filter_aliases, parse_markdown_to_html, ViewMode};
