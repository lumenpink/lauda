pub mod auth;
pub mod compose;
pub mod contact;
pub mod mailbox;
pub mod message;

pub use auth::{LoginRequest, TokenResponse};
pub use compose::{AttachmentUpload, ComposeForm};
pub use contact::ContactInfo;
pub use mailbox::MailboxInfo;
pub use message::{AttachmentMeta, Message, MessageEnvelope, ViewMode};
