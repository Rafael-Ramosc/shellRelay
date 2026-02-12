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
            let line = match format_message_datetime(&m.sent_at) {
                Some(date_time) => format!("[{}] {}: {}", date_time, sender, m.text),
                None => format!("{}: {}", sender, m.text),
            };
            ListItem::new(Line::from(line))
        })
        .collect();

    let messages = List::new(message_items)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .highlight_style(Style::default().bg(Color::DarkGray));

    // -------- LIST USERS ----------

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
        InstructionItem {
            label: "Quit",
            key: "Q",
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

fn short_identity(identity: &str) -> String {
    const MAX: usize = 18;
    if identity.len() <= MAX {
        return identity.to_string();
    }

    let head = &identity[..10.min(identity.len())];
    let tail = &identity[identity.len().saturating_sub(6)..];
    format!("{}..{}", head, tail)
}

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
