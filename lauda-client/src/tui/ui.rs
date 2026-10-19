use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::tui::app::{TuiApp, TuiScreen};

/// Renderiza o estado completo da interface TUI no Frame do Ratatui.
pub fn render(app: &mut TuiApp, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Conteúdo Principal
            Constraint::Length(3), // Footer / Barra de Status
        ])
        .split(frame.area());

    // 1. Header Bar
    let mailbox_badge = format!(" CAIXA: [{}] ", app.current_mailbox_name());
    let title = Line::from(vec![
        Span::styled(
            " LAUDA SMOLCLIENT ",
            Style::default()
                .bg(Color::Red)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            mailbox_badge,
            Style::default()
                .bg(Color::Black)
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));
    let header_widget = Paragraph::new(title).block(header_block);
    frame.render_widget(header_widget, chunks[0]);

    // 2. Conteúdo Principal baseado na tela ativa
    match app.current_screen() {
        TuiScreen::MailboxList => {
            render_mailbox_list(app, frame, chunks[1]);
        }
        TuiScreen::EnvelopeList => {
            render_envelope_list(app, frame, chunks[1]);
        }
        TuiScreen::MessageReader(_) => {
            render_message_reader(app, frame, chunks[1]);
        }
        TuiScreen::Search => {
            render_search_screen(app, frame, chunks[1]);
        }
    }

    // 3. Footer / Help Bar
    let help_text = match app.current_screen() {
        TuiScreen::MailboxList => "[j/k: Mover] [Enter: Selecionar] [Esc/q: Cancelar]",
        TuiScreen::EnvelopeList => {
            "[j/k: Mover] [Enter: Abrir] [m: Caixas] [/: Buscar] [r: Sincronizar] [q: Sair]"
        }
        TuiScreen::MessageReader(_) => "[q/Esc: Voltar à lista] [r: Responder]",
        TuiScreen::Search => "[Digitar para filtrar] [Esc: Cancelar busca] [Enter: Abrir]",
    };

    let status = app.status_message.as_deref().unwrap_or(help_text);
    let footer_widget = Paragraph::new(status).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(footer_widget, chunks[2]);
}

fn render_mailbox_list(app: &TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let selected_idx = app.selected_mailbox_index();
    let items: Vec<ListItem> = app
        .mailboxes()
        .iter()
        .enumerate()
        .map(|(idx, mb)| {
            let is_selected = idx == selected_idx;
            let style = if is_selected {
                Style::default()
                    .bg(Color::Red)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let content = format!(
                "  {} ({} total, {} não lidas)",
                mb.name, mb.total_messages, mb.unread_messages
            );
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Caixas Postais (Selecione com Enter) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(list, area);
}

fn render_envelope_list(app: &TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let envelopes = app.filtered_envelopes();
    let selected_idx = app.selected_envelope_index();

    let items: Vec<ListItem> = if envelopes.is_empty() {
        vec![
            ListItem::new("  (Nenhuma mensagem nesta caixa)")
                .style(Style::default().fg(Color::DarkGray)),
        ]
    } else {
        envelopes
            .iter()
            .enumerate()
            .map(|(idx, env)| {
                let is_selected = idx == selected_idx;
                let unread_marker = if !env.is_read { "● " } else { "  " };
                let flag_marker = if env.is_flagged { "★ " } else { "  " };
                let att_marker = if env.has_attachments { "📎 " } else { "" };

                let style = if is_selected {
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else if !env.is_read {
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };

                let line = format!(
                    "{}{}{}{:<25} | {:<40} | {}",
                    unread_marker, flag_marker, att_marker, env.sender, env.subject, env.date
                );
                ListItem::new(line).style(style)
            })
            .collect()
    };

    let title = format!(" Mensagens em [{}] ", app.current_mailbox_name());
    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White)),
    );
    frame.render_widget(list, area);
}

fn render_message_reader(app: &TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(5)])
        .split(area);

    if let Some(msg) = app.active_message() {
        let headers = vec![
            Line::from(vec![
                Span::styled("De: ", Style::default().fg(Color::Yellow)),
                Span::raw(&msg.envelope.sender),
            ]),
            Line::from(vec![
                Span::styled("Assunto: ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    &msg.envelope.subject,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Data: ", Style::default().fg(Color::Yellow)),
                Span::raw(&msg.envelope.date),
            ]),
        ];
        let header_widget = Paragraph::new(headers).block(
            Block::default()
                .title(" Detalhes do E-mail ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(header_widget, chunks[0]);

        let body = if let Some(ref auto) = msg.txt_auto
            && !auto.is_empty()
        {
            auto.as_str()
        } else if let Some(ref orig) = msg.txt_original
            && !orig.is_empty()
        {
            orig.as_str()
        } else {
            "(Mensagem sem conteúdo textual legível)"
        };

        let body_widget = Paragraph::new(body).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(" Corpo da Mensagem ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        );
        frame.render_widget(body_widget, chunks[1]);
    } else {
        let loading = Paragraph::new("Carregando mensagem...").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(loading, area);
    }
}

fn render_search_screen(app: &TuiApp, frame: &mut Frame, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);

    let query_line = format!("🔍 {}", app.search_query());
    let search_bar = Paragraph::new(query_line).block(
        Block::default()
            .title(" Busca Instantânea (Assunto ou Remetente) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(search_bar, chunks[0]);

    render_envelope_list(app, frame, chunks[1]);
}
