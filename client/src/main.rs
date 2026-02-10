mod module_bindings;
mod state;
mod ui;

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
use ratatui::{Terminal, backend::CrosstermBackend};
use spacetimedb_sdk::{DbContext, Table, TableWithPrimaryKey};
use state::{AppState, InputMode, SharedState, UiMessage, UiUser, snapshot_state, update_state};
use ui::ui_message_screen::render_ui;

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
        status: false,
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
                s.status = true;
            });

            let on_applied_state = Arc::clone(&on_connect_state);
            let on_error_state = Arc::clone(&on_connect_state);
            ctx.subscription_builder()
                .on_applied(move |sub_ctx| {
                    sync_from_tables(&sub_ctx.db, &on_applied_state);
                    update_state(&on_applied_state, |s| {
                        s.status = true;
                    });
                })
                .on_error(move |_err_ctx, _err| {
                    update_state(&on_error_state, |s| {
                        s.status = false;
                    });
                })
                .subscribe_to_all_tables();
        })
        .on_disconnect(move |_ctx, _err| {
            update_state(&on_disconnect_state, |s| {
                s.status = false;
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

            if reducer_res.is_err() {
                update_state(state, |s| {
                    s.status = false;
                });
            }
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
