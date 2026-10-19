use lauda_types::{MailboxInfo, Message, MessageEnvelope};

/// Telas ativas da interface de terminal TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiScreen {
    /// Lista de mensagens na caixa postal selecionada.
    EnvelopeList,
    /// Painel de seleção de caixa postal.
    MailboxList,
    /// Leitor de corpo e metadados de mensagem específica.
    MessageReader(u32),
    /// Campo de busca interativo.
    Search,
}

/// Ações disparadas por atalhos de teclado na TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiAction {
    MoveUp,
    MoveDown,
    Enter,
    Back,
    Quit,
    SwitchMailboxPane,
    OpenSearch,
    Refresh,
}

/// Máquina de estados desacoplada da interface TUI.
#[derive(Debug, Clone)]
pub struct TuiApp {
    mailboxes: Vec<MailboxInfo>,
    envelopes: Vec<MessageEnvelope>,
    current_mailbox_name: String,
    selected_envelope_idx: usize,
    selected_mailbox_idx: usize,
    screen: TuiScreen,
    search_query: String,
    active_message: Option<Message>,
    should_quit: bool,
    pub status_message: Option<String>,
}

impl TuiApp {
    /// Inicializa a máquina de estados da TUI com caixas e envelopes pré-carregados.
    pub fn new(mailboxes: Vec<MailboxInfo>, envelopes: Vec<MessageEnvelope>) -> Self {
        let initial_mailbox = mailboxes
            .first()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "INBOX".to_string());

        Self {
            mailboxes,
            envelopes,
            current_mailbox_name: initial_mailbox,
            selected_envelope_idx: 0,
            selected_mailbox_idx: 0,
            screen: TuiScreen::EnvelopeList,
            search_query: String::new(),
            active_message: None,
            should_quit: false,
            status_message: None,
        }
    }

    pub fn current_screen(&self) -> TuiScreen {
        self.screen.clone()
    }

    pub fn selected_envelope_index(&self) -> usize {
        self.selected_envelope_idx
    }

    pub fn selected_mailbox_index(&self) -> usize {
        self.selected_mailbox_idx
    }

    pub fn current_mailbox_name(&self) -> &str {
        &self.current_mailbox_name
    }

    pub fn active_message(&self) -> Option<&Message> {
        self.active_message.as_ref()
    }

    pub fn set_active_message(&mut self, msg: Option<Message>) {
        self.active_message = msg;
    }

    pub fn set_envelopes(&mut self, envs: Vec<MessageEnvelope>) {
        self.envelopes = envs;
        if self.selected_envelope_idx >= self.envelopes.len() {
            self.selected_envelope_idx = self.envelopes.len().saturating_sub(1);
        }
    }

    pub fn set_mailboxes(&mut self, boxes: Vec<MailboxInfo>) {
        self.mailboxes = boxes;
        if self.selected_mailbox_idx >= self.mailboxes.len() {
            self.selected_mailbox_idx = self.mailboxes.len().saturating_sub(1);
        }
    }

    pub fn mailboxes(&self) -> &[MailboxInfo] {
        &self.mailboxes
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Retorna os envelopes filtrados pela consulta de busca atual.
    pub fn filtered_envelopes(&self) -> Vec<&MessageEnvelope> {
        let q = self.search_query.trim().to_lowercase();
        if q.is_empty() {
            self.envelopes.iter().collect()
        } else {
            self.envelopes
                .iter()
                .filter(|e| {
                    e.subject.to_lowercase().contains(&q) || e.sender.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    /// Atualiza o texto da busca.
    pub fn update_search_query(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.selected_envelope_idx = 0;
    }

    /// Processa uma ação ergonômica de navegação.
    pub fn handle_action(&mut self, action: TuiAction) {
        match action {
            TuiAction::MoveDown => match self.screen {
                TuiScreen::EnvelopeList | TuiScreen::Search => {
                    let total = self.filtered_envelopes().len();
                    if total > 0 && self.selected_envelope_idx + 1 < total {
                        self.selected_envelope_idx += 1;
                    }
                }
                TuiScreen::MailboxList => {
                    let total = self.mailboxes.len();
                    if total > 0 && self.selected_mailbox_idx + 1 < total {
                        self.selected_mailbox_idx += 1;
                    }
                }
                TuiScreen::MessageReader(_) => {}
            },
            TuiAction::MoveUp => match self.screen {
                TuiScreen::EnvelopeList | TuiScreen::Search => {
                    if self.selected_envelope_idx > 0 {
                        self.selected_envelope_idx -= 1;
                    }
                }
                TuiScreen::MailboxList => {
                    if self.selected_mailbox_idx > 0 {
                        self.selected_mailbox_idx -= 1;
                    }
                }
                TuiScreen::MessageReader(_) => {}
            },
            TuiAction::Enter => match self.screen {
                TuiScreen::EnvelopeList | TuiScreen::Search => {
                    let filtered = self.filtered_envelopes();
                    if let Some(target) = filtered.get(self.selected_envelope_idx) {
                        self.screen = TuiScreen::MessageReader(target.uid);
                    }
                }
                TuiScreen::MailboxList => {
                    if let Some(mb) = self.mailboxes.get(self.selected_mailbox_idx) {
                        self.current_mailbox_name = mb.name.clone();
                        self.selected_envelope_idx = 0;
                        self.screen = TuiScreen::EnvelopeList;
                    }
                }
                TuiScreen::MessageReader(_) => {}
            },
            TuiAction::Back => match self.screen {
                TuiScreen::MessageReader(_) => {
                    self.active_message = None;
                    self.screen = TuiScreen::EnvelopeList;
                }
                TuiScreen::Search => {
                    self.search_query.clear();
                    self.screen = TuiScreen::EnvelopeList;
                }
                TuiScreen::MailboxList => {
                    self.screen = TuiScreen::EnvelopeList;
                }
                TuiScreen::EnvelopeList => {
                    self.should_quit = true;
                }
            },
            TuiAction::Quit => {
                self.should_quit = true;
            }
            TuiAction::SwitchMailboxPane => {
                if self.screen == TuiScreen::MailboxList {
                    self.screen = TuiScreen::EnvelopeList;
                } else {
                    self.screen = TuiScreen::MailboxList;
                }
            }
            TuiAction::OpenSearch => {
                self.screen = TuiScreen::Search;
            }
            TuiAction::Refresh => {
                self.status_message = Some("Sincronizando com o servidor...".to_string());
            }
        }
    }
}
