use crate::ui::{
    instructions::{InstructionItem, render_instructions},
    ui_state::UiState,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::collections::HashMap;

/// Renderiza a tela principal de chat (mensagens, usuários, input e rodapé).
pub fn render_ui(frame: &mut ratatui::Frame<'_>, state: &UiState, is_server_online: bool) {
    // -------- MAIN LAYOUT ----------

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), //title
            Constraint::Min(6),    //body
            Constraint::Length(3), //input
            Constraint::Length(1), //instructions
        ])
        .split(frame.area());

    // -------- TITLE ----------

    let header_block = Block::default().borders(Borders::ALL).title("ShellRelay");
    let status_label = if is_server_online {
        "Server online"
    } else {
        "Server offline"
    };
    let status_color = if is_server_online {
        Color::Green
    } else {
        Color::Red
    };

    // -------- BODY LAYOUT ----------

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(chunks[1]);
    let messages_inner_width = body[0].width.saturating_sub(2) as usize;

    // -------- LIST MESSAGE ----------
    // Mapeia identity -> nome para exibir remetentes de forma amigável.

    let user_names_by_identity: HashMap<&str, &str> = state
        .users
        .iter()
        .map(|u| (u.identity.as_str(), u.name.as_str()))
        .collect();

    let message_lines: Vec<Line<'_>> = state
        .messages
        .iter()
        .flat_map(|m| {
            let sender = user_names_by_identity
                .get(m.sender.as_str())
                .copied()
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| short_identity(&m.sender));

            let prefix = match format_message_datetime(&m.sent_at) {
                Some(date_time) => format!("[{}] {}: ", date_time, sender),
                None => format!("{}: ", sender),
            };
            let wrapped_lines = wrap_message_lines(&prefix, &m.text, messages_inner_width);
            wrapped_lines
                .into_iter()
                .map(Line::from)
                .collect::<Vec<Line<'_>>>()
        })
        .collect();
    let messages_visible_rows = body[0].height.saturating_sub(2) as usize;
    let messages_max_scroll = if messages_visible_rows == 0 {
        0
    } else {
        message_lines.len().saturating_sub(messages_visible_rows)
    };
    let messages_scroll = messages_max_scroll
        .saturating_sub(state.messages_scroll_from_bottom.min(messages_max_scroll));
    let messages = Paragraph::new(message_lines)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .scroll((messages_scroll.min(u16::MAX as usize) as u16, 0))
        .wrap(Wrap { trim: false });

    // -------- LIST USERS ----------
    // Lista lateral com scroll independente da lista de mensagens.

    let users_visible_rows = body[1].height.saturating_sub(2) as usize;
    let reserve_hint_row = users_visible_rows > 1 && state.users.len() > users_visible_rows;
    let users_list_rows = if reserve_hint_row {
        users_visible_rows - 1
    } else {
        users_visible_rows
    };
    let users_max_scroll = if users_list_rows == 0 {
        0
    } else {
        state.users.len().saturating_sub(users_list_rows)
    };
    let users_scroll = state.users_scroll.min(users_max_scroll);
    let users_end = users_scroll
        .saturating_add(users_list_rows)
        .min(state.users.len());

    let user_items: Vec<ListItem<'_>> = state
        .users
        .get(users_scroll..users_end)
        .unwrap_or(&[])
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

    let users_online = state.users.iter().filter(|u| u.online).count();
    let users_offline = state.users.len().saturating_sub(users_online);
    let users_title = format!(
        "Users (Online: {} | Offline: {})",
        users_online, users_offline
    );

    let users = List::new(user_items)
        .block(Block::default().borders(Borders::ALL).title(users_title))
        .highlight_style(Style::default().bg(Color::DarkGray));

    // -------- INPUT ----------

    let input_title = "Message";
    let input = Paragraph::new(state.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(input_title))
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: false });

    frame.render_widget(header_block, chunks[0]);
    let header_inner = Rect {
        x: chunks[0].x.saturating_add(1),
        y: chunks[0].y.saturating_add(1),
        width: chunks[0].width.saturating_sub(2),
        height: chunks[0].height.saturating_sub(2),
    };
    if header_inner.width > 0 && header_inner.height > 0 {
        let header_content = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(18)])
            .split(header_inner);

        let header_text = Paragraph::new(vec![
            Line::from("Messages and user list").style(Style::default().fg(Color::Yellow)),
        ]);
        frame.render_widget(header_text, header_content[0]);

        let header_status = Paragraph::new(status_label)
            .alignment(Alignment::Right)
            .style(Style::default().fg(status_color));
        frame.render_widget(header_status, header_content[1]);
    }

    //body chunk[1]
    frame.render_widget(messages, body[0]);
    render_messages_overflow_hint(frame, body[0], messages_scroll, messages_max_scroll);

    // users chunk
    frame.render_widget(users, body[1]);
    if reserve_hint_row {
        render_users_overflow_hint(frame, body[1], users_scroll, users_max_scroll);
    }

    frame.render_widget(input, chunks[2]);

    let instructions = [
        InstructionItem {
            label: "Send",
            key: "Enter",
        },
        InstructionItem {
            label: "Messages",
            key: "PgUp/PgDn",
        },
        InstructionItem {
            label: "Users",
            key: "Up/Down",
        },
        InstructionItem {
            label: "Delete",
            key: "Backspace",
        },
        InstructionItem {
            label: "Menu",
            key: "F1",
        },
    ];
    render_instructions(frame, chunks[3], &instructions);
}

fn render_users_overflow_hint(
    frame: &mut ratatui::Frame<'_>,
    users_area: Rect,
    users_scroll: usize,
    users_max_scroll: usize,
) {
    if users_max_scroll == 0 {
        return;
    }

    let hint_area = Rect {
        x: users_area.x.saturating_add(1),
        y: users_area
            .y
            .saturating_add(users_area.height.saturating_sub(2)),
        width: users_area.width.saturating_sub(2),
        height: 1,
    };
    if hint_area.width == 0 {
        return;
    }

    let hint_text = if users_scroll == 0 {
        "Scroll down ↓"
    } else if users_scroll >= users_max_scroll {
        "Scroll up ↑"
    } else {
        "Scroll ↑↓"
    };

    let hint = Paragraph::new(hint_text)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, hint_area);
}

/// Hint de navegação da caixa de mensagens quando existe overflow vertical.
fn render_messages_overflow_hint(
    frame: &mut ratatui::Frame<'_>,
    messages_area: Rect,
    messages_scroll: usize,
    messages_max_scroll: usize,
) {
    if messages_max_scroll == 0 {
        return;
    }

    let hint_area = Rect {
        x: messages_area.x.saturating_add(1),
        y: messages_area
            .y
            .saturating_add(messages_area.height.saturating_sub(2)),
        width: messages_area.width.saturating_sub(2),
        height: 1,
    };
    if hint_area.width == 0 {
        return;
    }

    let hint_text = if messages_scroll == 0 {
        "Older messages ↑"
    } else if messages_scroll >= messages_max_scroll {
        "Newer messages ↓"
    } else {
        "Older ↑ | Newer ↓"
    };

    let hint = Paragraph::new(hint_text)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, hint_area);
}

/// Abrevia identity longa para caber no layout do terminal.
fn short_identity(identity: &str) -> String {
    const MAX: usize = 18;
    if identity.len() <= MAX {
        return identity.to_string();
    }

    let head = &identity[..10.min(identity.len())];
    let tail = &identity[identity.len().saturating_sub(6)..];
    format!("{}..{}", head, tail)
}

/// Tenta normalizar timestamps em formato curto `dd/mm/yyyy hh:mm`.
fn format_message_datetime(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let (date_raw, time_raw) = raw
        .split_once('T')
        .or_else(|| raw.split_once(' '))
        .unwrap_or(("", ""));

    if date_raw.is_empty() || time_raw.is_empty() {
        return Some(raw.to_string());
    }

    let mut date_parts = date_raw.split('-');
    let year = date_parts.next();
    let month = date_parts.next();
    let day = date_parts.next();
    if year.is_none() || month.is_none() || day.is_none() || date_parts.next().is_some() {
        return Some(raw.to_string());
    }

    let time_part = time_raw
        .split(['Z', '+'])
        .next()
        .unwrap_or(time_raw)
        .split('.')
        .next()
        .unwrap_or(time_raw);

    let hm = if time_part.len() >= 5 {
        &time_part[..5]
    } else {
        return Some(raw.to_string());
    };

    Some(format!(
        "{}/{}/{} {}",
        day.unwrap_or_default(),
        month.unwrap_or_default(),
        year.unwrap_or_default(),
        hm
    ))
}

/// Quebra uma mensagem em múltiplas linhas sem perder o contexto do prefixo
/// (`[data] nome:`), alinhando visualmente as linhas seguintes.
fn wrap_message_lines(prefix: &str, text: &str, total_width: usize) -> Vec<String> {
    if total_width == 0 {
        return vec![];
    }

    let prefix_width = prefix.chars().count();
    if prefix_width >= total_width {
        let full = format!("{prefix}{text}");
        return wrap_plain_lines(&full, total_width);
    }

    let content_width = (total_width - prefix_width).max(1);
    let wrapped_content = wrap_plain_lines(text, content_width);
    let mut out = Vec::with_capacity(wrapped_content.len().max(1));
    let indent = " ".repeat(prefix_width);

    if wrapped_content.is_empty() {
        out.push(prefix.to_string());
        return out;
    }

    for (idx, line) in wrapped_content.iter().enumerate() {
        if idx == 0 {
            out.push(format!("{prefix}{line}"));
        } else {
            out.push(format!("{indent}{line}"));
        }
    }

    out
}

/// Quebra texto bruto por largura fixa preservando quebras de linha existentes.
fn wrap_plain_lines(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![];
    }

    let mut out = Vec::new();
    let lines: Vec<&str> = if text.is_empty() {
        vec![""]
    } else {
        text.split('\n').collect()
    };

    for line in lines {
        if line.is_empty() {
            out.push(String::new());
            continue;
        }

        let chars: Vec<char> = line.chars().collect();
        let mut start = 0usize;
        while start < chars.len() {
            let end = (start + width).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            out.push(chunk);
            start = end;
        }
    }

    out
}

#[cfg(test)]
#[path = "../tests/ui_message_screen_tests.rs"]
mod tests;
