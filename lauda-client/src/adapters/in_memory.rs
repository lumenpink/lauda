use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use lauda_types::{
    ComposeForm, LoginRequest, MailboxInfo, Message, MessageEnvelope, TokenResponse,
};

use crate::domain::{ClientError, MailApiClient};

/// Fake em memória para testes do cliente sem necessidade de servidor HTTP rodando.
#[derive(Debug, Default)]
pub struct InMemoryApiClient {
    mailboxes: RwLock<Vec<MailboxInfo>>,
    messages: RwLock<HashMap<String, Vec<Message>>>, // mailbox -> msgs
    sent: RwLock<Vec<ComposeForm>>,
}

impl InMemoryApiClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_mailbox(&self, info: MailboxInfo) {
        self.mailboxes.write().unwrap().push(info);
    }

    pub fn add_message(&self, mailbox: &str, message: Message) {
        self.messages
            .write()
            .unwrap()
            .entry(mailbox.to_string())
            .or_default()
            .push(message);
    }

    pub fn sent_count(&self) -> usize {
        self.sent.read().unwrap().len()
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl MailApiClient for InMemoryApiClient {
    async fn login(&self, req: &LoginRequest) -> Result<TokenResponse, ClientError> {
        if req.username == "lumen" && req.password == "secret" {
            Ok(TokenResponse {
                token: "in_memory_fake_jwt".to_string(),
                expires_at: 9999999999,
                username: req.username.clone(),
            })
        } else {
            Err(ClientError::AuthFailed)
        }
    }

    async fn list_mailboxes(&self, _token: &str) -> Result<Vec<MailboxInfo>, ClientError> {
        Ok(self.mailboxes.read().unwrap().clone())
    }

    async fn list_messages(
        &self,
        _token: &str,
        mailbox: &str,
    ) -> Result<Vec<MessageEnvelope>, ClientError> {
        let msgs = self.messages.read().unwrap();
        if let Some(box_msgs) = msgs.get(mailbox) {
            return Ok(box_msgs.iter().map(|m| m.envelope.clone()).collect());
        }
        Ok(Vec::new())
    }

    async fn get_message(
        &self,
        _token: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<Message, ClientError> {
        let msgs = self.messages.read().unwrap();
        if let Some(box_msgs) = msgs.get(mailbox)
            && let Some(msg) = box_msgs.iter().find(|m| m.envelope.uid == uid)
        {
            return Ok(msg.clone());
        }
        Err(ClientError::NotFound)
    }

    async fn send_message(&self, _token: &str, form: &ComposeForm) -> Result<(), ClientError> {
        self.sent.write().unwrap().push(form.clone());
        Ok(())
    }

    async fn mark_read(
        &self,
        _token: &str,
        mailbox: &str,
        uid: u32,
        read: bool,
    ) -> Result<(), ClientError> {
        let mut msgs = self.messages.write().unwrap();
        if let Some(box_msgs) = msgs.get_mut(mailbox)
            && let Some(msg) = box_msgs.iter_mut().find(|m| m.envelope.uid == uid)
        {
            msg.envelope.is_read = read;
            return Ok(());
        }
        Err(ClientError::NotFound)
    }

    async fn delete_message(
        &self,
        _token: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<(), ClientError> {
        let mut msgs = self.messages.write().unwrap();
        if let Some(box_msgs) = msgs.get_mut(mailbox)
            && let Some(pos) = box_msgs.iter().position(|m| m.envelope.uid == uid)
        {
            let msg = box_msgs.remove(pos);
            if mailbox != "Trash" {
                msgs.entry("Trash".to_string()).or_default().push(msg);
            }
            return Ok(());
        }
        Err(ClientError::NotFound)
    }

    async fn mark_flagged(
        &self,
        _token: &str,
        mailbox: &str,
        uid: u32,
        flagged: bool,
    ) -> Result<(), ClientError> {
        let mut msgs = self.messages.write().unwrap();
        if let Some(box_msgs) = msgs.get_mut(mailbox)
            && let Some(msg) = box_msgs.iter_mut().find(|m| m.envelope.uid == uid)
        {
            msg.envelope.is_flagged = flagged;
            return Ok(());
        }
        Err(ClientError::NotFound)
    }

    async fn move_message(
        &self,
        _token: &str,
        uid: u32,
        src_box: &str,
        dest_box: &str,
    ) -> Result<(), ClientError> {
        let mut msgs = self.messages.write().unwrap();
        let msg = {
            let Some(box_msgs) = msgs.get_mut(src_box) else {
                return Err(ClientError::NotFound);
            };
            let Some(pos) = box_msgs.iter().position(|m| m.envelope.uid == uid) else {
                return Err(ClientError::NotFound);
            };
            box_msgs.remove(pos)
        };
        msgs.entry(dest_box.to_string()).or_default().push(msg);
        Ok(())
    }

    fn attachment_url(&self, mailbox: &str, uid: u32, attachment_id: usize) -> String {
        format!("in-memory://{mailbox}/{uid}/{attachment_id}")
    }

    async fn fetch_attachment(
        &self,
        _token: &str,
        mailbox: &str,
        uid: u32,
        attachment_id: usize,
    ) -> Result<(String, String, Vec<u8>), ClientError> {
        let msgs = self.messages.read().unwrap();
        let Some(box_msgs) = msgs.get(mailbox) else {
            return Err(ClientError::NotFound);
        };
        let Some(msg) = box_msgs.iter().find(|m| m.envelope.uid == uid) else {
            return Err(ClientError::NotFound);
        };
        let Some(meta) = msg.attachments.get(attachment_id) else {
            return Err(ClientError::NotFound);
        };
        Ok((
            meta.filename.clone(),
            meta.content_type.clone(),
            b"dummy content".to_vec(),
        ))
    }

    async fn list_contacts(
        &self,
        _token: &str,
    ) -> Result<Vec<lauda_types::ContactInfo>, ClientError> {
        let msgs = self.messages.read().unwrap();
        let mut map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for box_msgs in msgs.values() {
            for m in box_msgs {
                *map.entry(m.envelope.sender.clone()).or_insert(0) += 1;
            }
        }

        let mut contacts: Vec<lauda_types::ContactInfo> = map
            .into_iter()
            .map(|(sender, count)| {
                let (name, email) = if sender.contains('<') && sender.contains('>') {
                    let parts: Vec<&str> = sender.split('<').collect();
                    let n = parts[0].trim().to_string();
                    let e = parts[1].trim_matches('>').trim().to_string();
                    (if n.is_empty() { None } else { Some(n) }, e)
                } else {
                    (None, sender)
                };
                lauda_types::ContactInfo {
                    email,
                    name,
                    message_count: count,
                    has_autocrypt: false,
                    autocrypt_fingerprint: None,
                }
            })
            .collect();

        contacts.sort_by_key(|c| std::cmp::Reverse(c.message_count));
        Ok(contacts)
    }
}
