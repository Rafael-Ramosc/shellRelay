mod module_bindings;

use std::{
    env, io,
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use module_bindings::{
    DbConnection, MessageTableAccess, UserTableAccess, send_message as SendMessageReducerExt,
    set_name as SetNameReducerExt,
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey};

#[derive(Clone, Copy, Default)]
enum InputMode {
    #[default]
    Message,
    Name,
}

impl InputMode {
    fn label(self) -> &'static str {
        match self {
            Self::Message => "Mensagem",
            Self::Name => "Nome",
        }
    }

    fn toggle(&mut self) {
        *self = match *self {
            Self::Message => Self::Name,
            Self::Name => Self::Message,
        };
    }
}

#[derive(Clone, Default)]
struct UiMessage {
    id: u64,
    sender: String,
    text: String,
    sent_at: String,
}

#[derive(Clone, Default)]
struct UiUser {
    identity: String,
    name: String,
    online: bool,
}

#[derive(Clone, Default)]
struct AppState {
    messages: Vec<UiMessage>,
    users: Vec<UiUser>,
    input: String,
    input_mode: InputMode,
    my_identity: Option<String>,
    status: String,
    should_quit: bool,
}

type SharedState = Arc<Mutex<AppState>>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module_name = env::args()
        .nth(1)
        .or_else(|| env::var("STDB_MODULE").ok())
        .unwrap_or_else(|| "shell-relay-test".to_string());

    let uri = env::args()
        .nth(2)
        .or_else(|| env::var("STDB_URI").ok())
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string());

    let state = Arc::new(Mutex::new(AppState {
        status: format!("Conectando em {} ({})...", module_name, uri),
        ..Default::default()
    }));

    let on_connect_state = Arc::clone(&state);
    let on_disconnect_state = Arc::clone(&state);

    let conn = DbConnection::builder()
        .with_uri(uri)
        .with_module_name(module_name)
        .on_connect(move |ctx, identity, _token| {
            update_state(&on_connect_state, |s| {
                s.my_identity = Some(identity.to_string());
                s.status = format!("Conectado como {}", short_identity(&identity.to_string()));
            });

            let on_applied_state = Arc::clone(&on_connect_state);
            let on_error_state = Arc::clone(&on_connect_state);
            ctx.subscription_builder()
                .on_applied(move |sub_ctx| {
                    sync_from_tables(&sub_ctx.db, &on_applied_state);
                    update_state(&on_applied_state, |s| {
                        s.status = "Assinatura ativa".to_string();
                    });
                })
                .on_error(move |_err_ctx, err| {
                    update_state(&on_error_state, |s| {
                        s.status = format!("Erro de assinatura: {err}");
                    });
                })
                .subscribe_to_all_tables();
        })
        .on_disconnect(move |_ctx, err| {
            update_state(&on_disconnect_state, |s| {
                s.status = match err {
                    Some(e) => format!("Desconectado: {e}"),
                    None => "Desconectado".to_string(),
                };
            });
        })
        .build()?;

    register_table_callbacks(&conn, &state);
    let worker = conn.run_threaded();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_res = run_app(&mut terminal, &conn, &state);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let _ = conn.disconnect();
    let _ = worker.join();

    app_res
}

fn register_table_callbacks(conn: &DbConnection, state: &SharedState) {
    let s = Arc::clone(state);
    let _ = conn.db.message().on_insert(move |ctx, _row| {
        sync_from_tables(&ctx.db, &s);
    });

    let s = Arc::clone(state);
    let _ = conn.db.message().on_delete(move |ctx, _row| {
        sync_from_tables(&ctx.db, &s);
    });

    let s = Arc::clone(state);
    let _ = conn.db.message().on_update(move |ctx, _old, _new| {
        sync_from_tables(&ctx.db, &s);
    });

    let s = Arc::clone(state);
    let _ = conn.db.user().on_insert(move |ctx, _row| {
        sync_from_tables(&ctx.db, &s);
    });

    let s = Arc::clone(state);
    let _ = conn.db.user().on_delete(move |ctx, _row| {
        sync_from_tables(&ctx.db, &s);
    });

    let s = Arc::clone(state);
    let _ = conn.db.user().on_update(move |ctx, _old, _new| {
        sync_from_tables(&ctx.db, &s);
    });
}

fn sync_from_tables(db: &module_bindings::RemoteTables, state: &SharedState) {
    let mut messages: Vec<UiMessage> = db
        .message()
        .iter()
        .map(|m| UiMessage {
            id: m.id,
            sender: m.sender.to_string(),
            text: m.text,
            sent_at: m.sent_at.to_string(),
        })
        .collect();
    messages.sort_by_key(|m| m.id);

    let mut users: Vec<UiUser> = db
        .user()
        .iter()
        .map(|u| UiUser {
            identity: u.identity.to_string(),
            name: u.name,
            online: u.online,
        })
        .collect();
    users.sort_by(|a, b| {
        b.online
            .cmp(&a.online)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    update_state(state, |s| {
        s.messages = messages;
        s.users = users;
    });
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    conn: &DbConnection,
    state: &SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let snapshot = snapshot_state(state);
        terminal.draw(|f| render_ui(f, &snapshot))?;

        if snapshot.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(50))?
            && let CEvent::Key(key) = event::read()?
        {
            handle_key_event(key, conn, state)?;
        }
    }

    Ok(())
}

fn handle_key_event(
    key: KeyEvent,
    conn: &DbConnection,
    state: &SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    if key.kind != event::KeyEventKind::Press {
        return Ok(());
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        update_state(state, |s| s.should_quit = true);
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') => {
            update_state(state, |s| s.should_quit = true);
        }
        KeyCode::Tab => {
            update_state(state, |s| s.input_mode.toggle());
        }
        KeyCode::Esc => {
            update_state(state, |s| s.input.clear());
        }
        KeyCode::Backspace => {
            update_state(state, |s| {
                s.input.pop();
            });
        }
        KeyCode::Enter => {
            let (mode, text) = {
                let mut guard = state.lock().expect("state poisoned");
                let text = guard.input.trim().to_string();
                guard.input.clear();
                (guard.input_mode, text)
            };

            if text.is_empty() {
                return Ok(());
            }

            let reducer_res = match mode {
                InputMode::Message => conn.reducers.send_message(text),
                InputMode::Name => conn.reducers.set_name(text),
            };

            update_state(state, |s| match reducer_res {
                Ok(()) => {
                    s.status = match mode {
                        InputMode::Message => "Mensagem enviada".to_string(),
                        InputMode::Name => "Nome enviado".to_string(),
                    };
                }
                Err(e) => {
                    s.status = format!("Falha ao chamar reducer: {e}");
                }
            });
        }
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
            {
                update_state(state, |s| s.input.push(c));
            }
        }
        _ => {}
    }

    Ok(())
}

fn render_ui(frame: &mut ratatui::Frame<'_>, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header = Paragraph::new(vec![
        Line::from("q sair | Enter enviar | Tab alterna Mensagem/Nome | Esc limpa"),
        Line::from(state.status.as_str()).style(Style::default().fg(Color::Cyan)),
    ])
    .block(Block::default().borders(Borders::ALL).title("Shell Relay"));
    frame.render_widget(header, chunks[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(chunks[1]);

    let message_items: Vec<ListItem<'_>> = state
        .messages
        .iter()
        .map(|m| {
            let sender = short_identity(&m.sender);
            let line = format!("#{} {}", sender, m.text);
            ListItem::new(Line::from(line))
        })
        .collect();

    let messages = List::new(message_items)
        .block(Block::default().borders(Borders::ALL).title("Mensagens"))
        .highlight_style(Style::default().bg(Color::DarkGray));
    frame.render_widget(messages, body[0]);

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
    frame.render_widget(users, body[1]);

    let input_title = format!("Input [{}]", state.input_mode.label());
    let input = Paragraph::new(state.input.as_str())
        .block(Block::default().borders(Borders::ALL).title(input_title))
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[2]);
}

fn snapshot_state(state: &SharedState) -> AppState {
    state.lock().map(|s| s.clone()).unwrap_or_default()
}

fn update_state(state: &SharedState, f: impl FnOnce(&mut AppState)) {
    if let Ok(mut s) = state.lock() {
        f(&mut s);
    }
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
