use crate::ui::{
    instructions::{InstructionItem, render_instructions},
    ui_state::{MainMenuItem, UiPopup, UiState},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn render_menu_screen(frame: &mut ratatui::Frame<'_>, state: &UiState, _is_server_online: bool) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let title = Paragraph::new("ShellRelay")
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    let menu_area = centered_rect(42, 40, chunks[1]);
    let menu_items: Vec<ListItem<'_>> = MainMenuItem::ALL
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = idx == state.menu_selected;
            let prefix = if is_selected { ">" } else { " " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(format!("{prefix} {}", item.label()))).style(style)
        })
        .collect();

    let menu = List::new(menu_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Main menu")
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(menu, menu_area);

    match state.popup {
        Some(UiPopup::ChooseName) => render_choose_name_popup(frame, state),
        Some(UiPopup::Soon) => render_soon_popup(frame),
        None => {
            let instructions = menu_instructions();
            render_instructions(frame, chunks[2], &instructions);
        }
    }
}

fn menu_instructions() -> [InstructionItem<'static>; 4] {
    [
        InstructionItem {
            label: "Up",
            key: "Up",
        },
        InstructionItem {
            label: "Down",
            key: "Down",
        },
        InstructionItem {
            label: "Select",
            key: "Enter",
        },
        InstructionItem {
            label: "Quit",
            key: "Q",
        },
    ]
}

fn render_choose_name_popup(frame: &mut ratatui::Frame<'_>, state: &UiState) {
    let area = centered_rect(50, 28, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .title("Choose name")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
        area,
    );

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let hint = Paragraph::new("Type your name to enter the chat.")
        .style(Style::default().fg(Color::Gray))
        .wrap(Wrap { trim: true });
    frame.render_widget(hint, inner[0]);

    let input = Paragraph::new(state.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Name"))
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, inner[1]);

    let instructions = [
        InstructionItem {
            label: "Confirm",
            key: "Enter",
        },
        InstructionItem {
            label: "Delete",
            key: "Backspace",
        },
        InstructionItem {
            label: "Close",
            key: "Esc",
        },
        InstructionItem {
            label: "Quit",
            key: "Q",
        },
    ];
    render_instructions(frame, inner[3], &instructions);
}

fn render_soon_popup(frame: &mut ratatui::Frame<'_>) {
    let area = centered_rect(34, 22, frame.area());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Block::default()
            .title("Options")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
        area,
    );

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let popup = Paragraph::new("Soon").alignment(Alignment::Center).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(popup, inner[1]);

    let instructions = [
        InstructionItem {
            label: "Close",
            key: "Enter",
        },
        InstructionItem {
            label: "Close",
            key: "Esc",
        },
        InstructionItem {
            label: "Quit",
            key: "Q",
        },
    ];
    render_instructions(frame, inner[2], &instructions);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
