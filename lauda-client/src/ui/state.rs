/// Telas de navegação da interface mobile-first do Lauda Smolclient.
#[derive(Default, Clone, PartialEq, Debug)]
pub enum Screen {
    /// Formulário de autenticação e conexão com servidor.
    #[default]
    Login,
    /// Lista de mensagens da caixa postal ativa.
    Inbox,
    /// Visualizador trifocal de mensagem aberta.
    MessageDetail(u32),
    /// Redação de nova mensagem ou resposta.
    Compose {
        reply_to: Option<String>,
        reply_subject: Option<String>,
    },
}
