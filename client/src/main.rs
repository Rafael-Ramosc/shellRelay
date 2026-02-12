mod module_bindings;
mod sync;
mod state;
mod ui;

use std::{
    env, io,
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::{
    event::{self, Event as CEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use module_bindings::DbConnection;
use ratatui::{Terminal, backend::CrosstermBackend};
use spacetimedb_sdk::DbContext;
use state::{AppState, SharedState, snapshot_state, update_state};
use sync::{register_table_callbacks, sync_from_tables};
use ui::key_handler::handle_key_event;
use ui::ui_menu_screen::render_menu_screen;
use ui::ui_message_screen::render_ui;
use ui::ui_state::UiScreen;

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

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    conn: &DbConnection,
    state: &SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let snapshot = snapshot_state(state);
        terminal.draw(|f| match snapshot.ui.screen {
            UiScreen::MainMenu => render_menu_screen(f, &snapshot.ui, snapshot.status),
            UiScreen::Chat => render_ui(f, &snapshot.ui, snapshot.status),
        })?;

        if snapshot.ui.should_quit {
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
