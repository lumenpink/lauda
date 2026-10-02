use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use lauda_types::{ComposeForm, MailboxInfo, Message, MessageEnvelope};

use crate::domain::{BackendError, MailBackend};

/// Fake em memória hermético para testes automatizados rápidos sem dependência do Dovecot/Postfix.
#[derive(Debug, Default)]
pub struct InMemoryMailBackend {
    users: RwLock<HashMap<String, String>>, // username -> password
    mailboxes: RwLock<HashMap<String, Vec<MailboxInfo>>>, // username -> list of boxes
    messages: RwLock<HashMap<String, HashMap<String, Vec<Message>>>>, // username -> box -> msgs
    sent: RwLock<Vec<(String, ComposeForm)>>, // sent messages
}

impl InMemoryMailBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_user(&self, username: impl Into<String>, password: impl Into<String>) {
        self.users
            .write()
            .unwrap()
            .insert(username.into(), password.into());
    }

    pub fn add_mailbox(&self, username: &str, info: MailboxInfo) {
        self.mailboxes
            .write()
            .unwrap()
            .entry(username.to_string())
            .or_default()
            .push(info);
    }

    pub fn add_message(&self, username: &str, mailbox: &str, message: Message) {
        self.messages
            .write()
            .unwrap()
            .entry(username.to_string())
            .or_default()
            .entry(mailbox.to_string())
            .or_default()
            .push(message);
    }

    pub fn sent_count(&self) -> usize {
        self.sent.read().unwrap().len()
    }
}

#[async_trait]
impl MailBackend for InMemoryMailBackend {
    async fn authenticate(&self, username: &str, password: &str) -> Result<(), BackendError> {
        let users = self.users.read().unwrap();
        match users.get(username) {
            Some(expected) if expected == password => Ok(()),
            _ => Err(BackendError::AuthFailed),
        }
    }

    async fn list_mailboxes(&self, username: &str) -> Result<Vec<MailboxInfo>, BackendError> {
        let boxes = self.mailboxes.read().unwrap();
        Ok(boxes.get(username).cloned().unwrap_or_default())
    }

    async fn list_messages(
        &self,
        username: &str,
        mailbox: &str,
    ) -> Result<Vec<MessageEnvelope>, BackendError> {
        let msgs = self.messages.read().unwrap();
        if let Some(user_boxes) = msgs.get(username) {
            if let Some(box_msgs) = user_boxes.get(mailbox) {
                return Ok(box_msgs.iter().map(|m| m.envelope.clone()).collect());
            }
        }
        Ok(Vec::new())
    }

    async fn get_message(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<Message, BackendError> {
        let msgs = self.messages.read().unwrap();
        if let Some(user_boxes) = msgs.get(username) {
            if let Some(box_msgs) = user_boxes.get(mailbox) {
                if let Some(msg) = box_msgs.iter().find(|m| m.envelope.uid == uid) {
                    return Ok(msg.clone());
                }
            }
        }
        Err(BackendError::NotFound(format!(
            "Mensagem UID {uid} não encontrada"
        )))
    }

    async fn send_message(&self, username: &str, form: &ComposeForm) -> Result<(), BackendError> {
        self.sent
            .write()
            .unwrap()
            .push((username.to_string(), form.clone()));
        Ok(())
    }
}
