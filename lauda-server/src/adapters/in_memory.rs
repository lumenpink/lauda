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
    sieve_scripts: RwLock<HashMap<String, String>>, // username -> script
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
        if let Some(user_boxes) = msgs.get(username)
            && let Some(box_msgs) = user_boxes.get(mailbox)
        {
            return Ok(box_msgs.iter().map(|m| m.envelope.clone()).collect());
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
        if let Some(user_boxes) = msgs.get(username)
            && let Some(box_msgs) = user_boxes.get(mailbox)
            && let Some(msg) = box_msgs.iter().find(|m| m.envelope.uid == uid)
        {
            return Ok(msg.clone());
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

    async fn mark_message_read(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
        read: bool,
    ) -> Result<(), BackendError> {
        let mut msgs = self.messages.write().unwrap();
        if let Some(user_boxes) = msgs.get_mut(username)
            && let Some(box_msgs) = user_boxes.get_mut(mailbox)
            && let Some(msg) = box_msgs.iter_mut().find(|m| m.envelope.uid == uid)
        {
            msg.envelope.is_read = read;
            return Ok(());
        }
        Err(BackendError::NotFound(format!(
            "Mensagem UID {uid} não encontrada"
        )))
    }

    async fn delete_message(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
    ) -> Result<(), BackendError> {
        let mut msgs = self.messages.write().unwrap();
        if let Some(user_boxes) = msgs.get_mut(username)
            && let Some(box_msgs) = user_boxes.get_mut(mailbox)
            && let Some(pos) = box_msgs.iter().position(|m| m.envelope.uid == uid)
        {
            let msg = box_msgs.remove(pos);
            if mailbox != "Trash" {
                user_boxes.entry("Trash".to_string()).or_default().push(msg);
            }
            return Ok(());
        }
        Err(BackendError::NotFound(format!(
            "Mensagem UID {uid} não encontrada"
        )))
    }

    async fn mark_message_flagged(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
        flagged: bool,
    ) -> Result<(), BackendError> {
        let mut msgs = self.messages.write().unwrap();
        if let Some(user_boxes) = msgs.get_mut(username)
            && let Some(box_msgs) = user_boxes.get_mut(mailbox)
            && let Some(msg) = box_msgs.iter_mut().find(|m| m.envelope.uid == uid)
        {
            msg.envelope.is_flagged = flagged;
            return Ok(());
        }
        Err(BackendError::NotFound(format!(
            "Mensagem UID {uid} não encontrada"
        )))
    }

    async fn move_message(
        &self,
        username: &str,
        src_box: &str,
        dest_box: &str,
        uid: u32,
    ) -> Result<(), BackendError> {
        let mut msgs = self.messages.write().unwrap();
        if let Some(user_boxes) = msgs.get_mut(username) {
            let msg_opt = if let Some(src_msgs) = user_boxes.get_mut(src_box) {
                src_msgs
                    .iter()
                    .position(|m| m.envelope.uid == uid)
                    .map(|pos| src_msgs.remove(pos))
            } else {
                None
            };

            if let Some(msg) = msg_opt {
                user_boxes
                    .entry(dest_box.to_string())
                    .or_default()
                    .push(msg);
                return Ok(());
            }
        }
        Err(BackendError::NotFound(format!(
            "Mensagem UID {uid} não encontrada"
        )))
    }

    async fn get_attachment(
        &self,
        username: &str,
        mailbox: &str,
        uid: u32,
        attachment_id: usize,
    ) -> Result<(String, String, Vec<u8>), BackendError> {
        let msgs = self.messages.read().unwrap();
        if let Some(user_boxes) = msgs.get(username)
            && let Some(box_msgs) = user_boxes.get(mailbox)
            && let Some(msg) = box_msgs.iter().find(|m| m.envelope.uid == uid)
            && let Some(att) = msg.attachments.iter().find(|a| a.id == attachment_id)
        {
            return Ok((
                att.filename.clone(),
                att.content_type.clone(),
                format!("Conteúdo simulado de {}", att.filename).into_bytes(),
            ));
        }
        Err(BackendError::NotFound(format!(
            "Anexo {attachment_id} da mensagem UID {uid} não encontrado"
        )))
    }

    async fn list_contacts(
        &self,
        username: &str,
    ) -> Result<Vec<lauda_types::ContactInfo>, BackendError> {
        let msgs = self.messages.read().unwrap();
        let mut map: HashMap<String, (Option<String>, usize)> = HashMap::new();

        if let Some(user_boxes) = msgs.get(username) {
            for box_msgs in user_boxes.values() {
                for msg in box_msgs {
                    let sender = &msg.envelope.sender;
                    let (name, email) = if sender.contains('<') && sender.contains('>') {
                        let parts: Vec<&str> = sender.split('<').collect();
                        let n = parts[0].trim().to_string();
                        let e = parts[1].trim_matches('>').trim().to_string();
                        (if n.is_empty() { None } else { Some(n) }, e)
                    } else {
                        (None, sender.clone())
                    };

                    let entry = map.entry(email).or_insert((name, 0));
                    entry.1 += 1;
                }
            }
        }

        let mut contacts: Vec<lauda_types::ContactInfo> = map
            .into_iter()
            .map(|(email, (name, count))| lauda_types::ContactInfo {
                email,
                name,
                message_count: count,
                has_autocrypt: false,
                autocrypt_fingerprint: None,
                birthday: None,
            })
            .collect();

        contacts.sort_by_key(|c| std::cmp::Reverse(c.message_count));
        Ok(contacts)
    }

    async fn search_messages(
        &self,
        username: &str,
        mailbox: &str,
        query: &str,
    ) -> Result<Vec<MessageEnvelope>, BackendError> {
        let msgs_lock = self.messages.read().unwrap();
        let empty_map = HashMap::new();
        let user_boxes = msgs_lock.get(username).unwrap_or(&empty_map);
        let empty_vec = Vec::new();
        let msgs = user_boxes.get(mailbox).unwrap_or(&empty_vec);

        if query.trim().is_empty() {
            let mut list: Vec<MessageEnvelope> = msgs.iter().map(|m| m.envelope.clone()).collect();
            list.sort_by_key(|e| std::cmp::Reverse(e.uid));
            return Ok(list);
        }

        let mut index = lauda_index::MessageIndex::new();
        let provider = lauda_index::SimpleCpuEmbeddingProvider;

        for msg in msgs {
            index.index_message_semantic(mailbox, msg, &provider);
        }

        let results = index.search_hybrid(query, &provider, 0.5);

        let mut envelopes = Vec::new();
        for res in results {
            if let Some(found) = msgs.iter().find(|m| m.envelope.uid == res.uid) {
                envelopes.push(found.envelope.clone());
            }
        }

        Ok(envelopes)
    }

    async fn get_sieve_script(&self, username: &str) -> Result<String, BackendError> {
        let scripts = self.sieve_scripts.read().unwrap();
        Ok(scripts.get(username).cloned().unwrap_or_default())
    }

    async fn put_sieve_script(&self, username: &str, script: &str) -> Result<(), BackendError> {
        let mut scripts = self.sieve_scripts.write().unwrap();
        scripts.insert(username.to_string(), script.to_string());
        Ok(())
    }
}
