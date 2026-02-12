mod ai;
mod module_bindings;
mod state;
mod sync;
mod ui;

// `main.rs` concentra orquestração:
// - conexões com SpacetimeDB (usuário + bot de IA),
// - loop de renderização/eventos da TUI,
// - encaminhamento assíncrono de respostas da IA para o chat.

use std::{
    collections::VecDeque,
    env, io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    time::Duration,
};

use ai::AI_BOT_NAME;
use crossterm::{
    event::{self, Event as CEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use module_bindings::{
    DbConnection, send_message as SendMessageReducerExt, set_name as SetNameReducerExt,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use spacetimedb_sdk::DbContext;
use state::{AppState, SharedState, snapshot_state, update_state};
use sync::{register_table_callbacks, sync_from_tables};
use ui::key_handler::handle_key_event;
use ui::ui_menu_screen::render_menu_screen;
use ui::ui_message_screen::render_ui;
use ui::ui_state::UiScreen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Permite configurar módulo/URI por CLI ou variável de ambiente.
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

    // Conexão principal do usuário humano.
    let on_connect_state = Arc::clone(&state);
    let on_disconnect_state = Arc::clone(&state);

    let conn = DbConnection::builder()
        .with_uri(uri.clone())
        .with_module_name(module_name.clone())
        .on_connect(move |ctx, identity, _token| {
            // Marca conexão ativa e salva identity para uso em contexto da IA.
            update_state(&on_connect_state, |s| {
                s.my_identity = Some(identity.to_string());
                s.status = true;
            });

            // Assina todas as tabelas para manter cache local sincronizado.
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

    // Segunda conexão: usuário-bot que envia as respostas da IA como participante real.
    let ai_online = Arc::new(AtomicBool::new(false));
    let ai_on_connect = Arc::clone(&ai_online);
    let ai_on_disconnect = Arc::clone(&ai_online);
    let ai_conn = DbConnection::builder()
        .with_uri(uri)
        .with_module_name(module_name)
        .on_connect(move |ctx, _identity, _token| {
            // Bot entra no chat imediatamente ao conectar.
            let _ = ctx.reducers.set_name(AI_BOT_NAME.to_string());
            ai_on_connect.store(true, Ordering::SeqCst);
        })
        .on_disconnect(move |_ctx, _err| {
            ai_on_disconnect.store(false, Ordering::SeqCst);
        })
        .build()?;

    register_table_callbacks(&conn, &state);
    let worker = conn.run_threaded();
    let ai_worker = ai_conn.run_threaded();
    // Canal interno: thread da IA produz texto e o loop principal envia pelo bot.
    let (ai_reply_tx, ai_reply_rx) = mpsc::channel::<String>();

    // Inicialização da TUI.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_res = run_app(
        &mut terminal,
        &conn,
        &ai_conn,
        &state,
        &ai_reply_tx,
        &ai_reply_rx,
        &ai_online,
    );

    // Restaura terminal antes de sair.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let _ = conn.disconnect();
    let _ = ai_conn.disconnect();
    let _ = worker.join();
    let _ = ai_worker.join();

    app_res
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    conn: &DbConnection,
    ai_conn: &DbConnection,
    state: &SharedState,
    ai_reply_tx: &Sender<String>,
    ai_reply_rx: &Receiver<String>,
    ai_online: &AtomicBool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Fila de respostas aguardando o bot estar online para publicar.
    let mut pending_ai_replies: VecDeque<String> = VecDeque::new();

    loop {
        // Drena o canal sem bloquear para manter o loop responsivo.
        loop {
            match ai_reply_rx.try_recv() {
                Ok(reply) => pending_ai_replies.push_back(reply),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Publica respostas pendentes assim que o bot estiver conectado.
        if ai_online.load(Ordering::SeqCst) {
            while let Some(reply) = pending_ai_replies.front().cloned() {
                if ai_conn.reducers.send_message(reply).is_ok() {
                    let _ = pending_ai_replies.pop_front();
                } else {
                    break;
                }
            }
        }

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
            // Toda regra de teclado fica centralizada em `key_handler`.
            handle_key_event(key, conn, state, ai_reply_tx)?;
        }
    }

    Ok(())
}
