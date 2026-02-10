use crate::state::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::collections::HashMap;

pub fn render_ui(frame: &mut ratatui::Frame<'_>, state: &AppState) {
    // -------- MAIN LAYOUT ----------

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), //title
            Constraint::Min(6),    //body
            Constraint::Length(3), //input
        ])
        .split(frame.area());

    // -------- TITLE ----------

    let header_block = Block::default().borders(Borders::ALL).title("Shell Relay");
    let header = Paragraph::new(vec![
        Line::from("Mensagens e lista de usuários").style(Style::default().fg(Color::Yellow)),
    ])
    .block(header_block);

    // -------- BODY LAYOUT ----------

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(chunks[1]);

    // -------- LIST MESSAGE ----------

    let user_names_by_identity: HashMap<&str, &str> = state
        .users
        .iter()
        .map(|u| (u.identity.as_str(), u.name.as_str()))
        .collect();

    let message_items: Vec<ListItem<'_>> = state
        .messages
        .iter()
        .map(|m| {
            let sender = user_names_by_identity
                .get(m.sender.as_str())
                .copied()
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| short_identity(&m.sender));
            let line = format!("#{} {}", sender, m.text);
            ListItem::new(Line::from(line))
        })
        .collect();

    let messages = List::new(message_items)
        .block(Block::default().borders(Borders::ALL).title("Mensagens"))
        .highlight_style(Style::default().bg(Color::DarkGray));

    // -------- LIST USERS + SERVER STATUS ----------

    let users_status = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(95), Constraint::Percentage(5)])
        .split(body[1]);

    let user_items: Vec<ListItem<'_>> = state
        .users
        .iter()
        .map(|u| {
            let dot = if u.online { "●" } else { "○" };
            let color = if u.online {
                Color::Green
            } else {
                Color::DarkGray
            };
            let line = format!("{} {} ({})", dot, u.name, short_identity(&u.identity));
            ListItem::new(Line::from(line).style(Style::default().fg(color)))
        })
        .collect();

    let users_title = format!(
        "Usuarios ({})",
        state.users.iter().filter(|u| u.online).count()
    );

    let users = List::new(user_items)
        .block(Block::default().borders(Borders::ALL).title(users_title))
        .highlight_style(Style::default().bg(Color::DarkGray));

    let status_label = if state.status {
        "Servidor online"
    } else {
        "Servidor offline"
    };
    let status_color = if state.status {
        Color::Green
    } else {
        Color::Red
    };

    let server_status = Paragraph::new(status_label)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Status do servidor"),
        )
        .style(Style::default().fg(status_color))
        .wrap(Wrap { trim: true });

    // -------- INPUT ----------

    let input_title = format!("Input [{}]", state.input_mode.label());
    let input = Paragraph::new(state.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(input_title))
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: false });

    frame.render_widget(header, chunks[0]);

    //body chunk[1]
    frame.render_widget(messages, body[0]);

    // users/status chunks
    frame.render_widget(users, users_status[0]);
    frame.render_widget(server_status, users_status[1]);

    frame.render_widget(input, chunks[2]);
}

fn short_identity(identity: &str) -> String {
    const MAX: usize = 18;
    if identity.len() <= MAX {
        return identity.to_string();
    }

    let head = &identity[..10.min(identity.len())];
    let tail = &identity[identity.len().saturating_sub(6)..];
    format!("{}..{}", head, tail)
}
