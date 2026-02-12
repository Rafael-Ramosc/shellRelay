// `app.rs` concentra orquestração:
// - conexões com SpacetimeDB (usuário + bots de IA),
// - loop de renderização/eventos da TUI,
// - encaminhamento assíncrono de respostas das IAs para o chat.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::ai::{
    AI_BOT_COUNT, AI_PROACTIVE_COOLDOWN_SECS, AI_PROACTIVE_IDLE_SECS, AI_PROACTIVE_START_CHANCE,
    AI_TO_AI_REPLY_CHANCE_IDLE, AI_TO_AI_REPLY_CHANCE_WITH_HUMANS, AiBotProfile, AiGeneratedReply,
    MAX_AI_CHAIN_MESSAGES, generate_bot_profiles, request_bot_reply,
};
use crate::module_bindings::{
    DbConnection, send_message as SendMessageReducerExt, set_name as SetNameReducerExt,
};
use crate::state::{AppState, SharedState, snapshot_state, update_state};
use crate::sync::{SYSTEM_MESSAGE_ID_BASE, register_table_callbacks, sync_from_tables};
use crate::ui::key_handler::handle_key_event;
use crate::ui::ui_menu_screen::render_menu_screen;
use crate::ui::ui_message_screen::render_ui;
use crate::ui::ui_state::UiScreen;
use crossterm::{
    event::{self, Event as CEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::{
    RngExt,
    prelude::{IndexedRandom, SliceRandom},
    rng,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use spacetimedb_sdk::DbContext;

struct AiBotRuntime {
    profile: AiBotProfile,
    conn: DbConnection,
    online: Arc<AtomicBool>,
    identity: Arc<Mutex<Option<String>>>,
    worker: JoinHandle<()>,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
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

    // Instancia múltiplos bots com nomes/profissões sorteados no startup.
    let bot_profiles = generate_bot_profiles(AI_BOT_COUNT);
    let mut ai_bots = Vec::with_capacity(bot_profiles.len());
    for profile in bot_profiles {
        let online = Arc::new(AtomicBool::new(false));
        let identity = Arc::new(Mutex::new(None::<String>));

        let on_connect_online = Arc::clone(&online);
        let on_disconnect_online = Arc::clone(&online);
        let on_connect_identity = Arc::clone(&identity);
        let on_disconnect_identity = Arc::clone(&identity);
        let bot_name = profile.name.clone();

        let conn_bot = DbConnection::builder()
            .with_uri(uri.clone())
            .with_module_name(module_name.clone())
            .on_connect(move |ctx, identity, _token| {
                if let Ok(mut slot) = on_connect_identity.lock() {
                    *slot = Some(identity.to_string());
                }
                let _ = ctx.reducers.set_name(bot_name.clone());
                on_connect_online.store(true, Ordering::SeqCst);
            })
            .on_disconnect(move |_ctx, _err| {
                if let Ok(mut slot) = on_disconnect_identity.lock() {
                    *slot = None;
                }
                on_disconnect_online.store(false, Ordering::SeqCst);
            })
            .build()?;

        let worker = conn_bot.run_threaded();
        ai_bots.push(AiBotRuntime {
            profile,
            conn: conn_bot,
            online,
            identity,
            worker,
        });
    }

    register_table_callbacks(&conn, &state);
    let worker = conn.run_threaded();

    // Canal interno: threads de IA produzem texto e o loop principal envia via bots.
    let (ai_reply_tx, ai_reply_rx) = mpsc::channel::<AiGeneratedReply>();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_res = run_app(
        &mut terminal,
        &conn,
        &ai_bots,
        &state,
        &ai_reply_tx,
        &ai_reply_rx,
    );

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    let _ = conn.disconnect();
    let _ = worker.join();
    for bot in &ai_bots {
        let _ = bot.conn.disconnect();
    }
    for bot in ai_bots {
        let _ = bot.worker.join();
    }

    app_res
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    conn: &DbConnection,
    ai_bots: &[AiBotRuntime],
    state: &SharedState,
    ai_reply_tx: &Sender<AiGeneratedReply>,
    ai_reply_rx: &Receiver<AiGeneratedReply>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending_ai_replies: HashMap<String, VecDeque<String>> = ai_bots
        .iter()
        .map(|b| (b.profile.name.clone(), VecDeque::new()))
        .collect();

    let mut initialized_seen = false;
    let mut last_seen_message_id: u64 = 0;
    let mut consecutive_ai_messages: usize = 0;
    let mut last_chat_activity = Instant::now();
    let mut last_proactive_attempt = Instant::now();

    loop {
        // Drena o canal sem bloquear para manter o loop responsivo.
        loop {
            match ai_reply_rx.try_recv() {
                Ok(reply) => {
                    if let Some(queue) = pending_ai_replies.get_mut(&reply.bot_name) {
                        queue.push_back(reply.text);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Publica respostas pendentes assim que o bot correspondente estiver online.
        for bot in ai_bots {
            if !bot.online.load(Ordering::SeqCst) {
                continue;
            }
            let Some(queue) = pending_ai_replies.get_mut(&bot.profile.name) else {
                continue;
            };
            while let Some(reply) = queue.front().cloned() {
                if bot.conn.reducers.send_message(reply).is_ok() {
                    let _ = queue.pop_front();
                } else {
                    break;
                }
            }
        }

        let snapshot = snapshot_state(state);
        let bot_identities = current_bot_identity_set(ai_bots);
        let online_human_identities: HashSet<&str> = snapshot
            .ui
            .users
            .iter()
            .filter(|u| u.online && !bot_identities.contains(u.identity.as_str()))
            .map(|u| u.identity.as_str())
            .collect();
        let online_human_count = online_human_identities.len();

        let mut all_message_ids: Vec<u64> = snapshot
            .ui
            .messages
            .iter()
            .filter(|m| m.id < SYSTEM_MESSAGE_ID_BASE)
            .map(|m| m.id)
            .collect();
        all_message_ids.sort_unstable();
        if !initialized_seen {
            last_seen_message_id = all_message_ids.last().copied().unwrap_or(0);
            initialized_seen = true;
        } else {
            let mut new_messages: Vec<_> = snapshot
                .ui
                .messages
                .iter()
                .filter(|m| m.id > last_seen_message_id && m.id < SYSTEM_MESSAGE_ID_BASE)
                .cloned()
                .collect();
            new_messages.sort_by_key(|m| m.id);

            for message in new_messages {
                last_seen_message_id = last_seen_message_id.max(message.id);
                if message.text.trim().is_empty() {
                    continue;
                }

                let sender_is_ai = bot_identities.contains(message.sender.as_str());
                if !sender_is_ai && !online_human_identities.contains(message.sender.as_str()) {
                    continue;
                }
                last_chat_activity = Instant::now();

                if sender_is_ai {
                    consecutive_ai_messages = consecutive_ai_messages.saturating_add(1);
                } else {
                    consecutive_ai_messages = 0;
                }

                let directed_bot = find_directed_bot(ai_bots, &message.sender, &message.text);
                let maybe_bot = directed_bot.or_else(|| {
                    choose_responder_bot(
                        ai_bots,
                        &bot_identities,
                        &message.sender,
                        sender_is_ai,
                        consecutive_ai_messages,
                        online_human_count,
                    )
                });
                if let Some(bot) = maybe_bot {
                    request_bot_reply(
                        state,
                        bot.profile.clone(),
                        message.text.clone(),
                        ai_reply_tx.clone(),
                    );
                }
            }

            maybe_start_proactive_ai_chat(
                ai_bots,
                state,
                ai_reply_tx,
                &pending_ai_replies,
                online_human_count,
                &mut last_chat_activity,
                &mut last_proactive_attempt,
            );
        }

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

fn current_bot_identity_set(ai_bots: &[AiBotRuntime]) -> HashSet<String> {
    ai_bots
        .iter()
        .filter_map(|bot| bot.identity.lock().ok().and_then(|id| id.clone()))
        .collect()
}

fn choose_responder_bot<'a>(
    ai_bots: &'a [AiBotRuntime],
    bot_identities: &HashSet<String>,
    sender_identity: &str,
    sender_is_ai: bool,
    consecutive_ai_messages: usize,
    online_human_count: usize,
) -> Option<&'a AiBotRuntime> {
    let mut rng = rng();

    let mut candidates: Vec<&AiBotRuntime> = ai_bots
        .iter()
        .filter(|bot| bot.online.load(Ordering::SeqCst))
        .collect();

    if sender_is_ai {
        if consecutive_ai_messages >= MAX_AI_CHAIN_MESSAGES {
            return None;
        }
        let ai_reply_chance = if online_human_count > 0 {
            AI_TO_AI_REPLY_CHANCE_WITH_HUMANS
        } else {
            AI_TO_AI_REPLY_CHANCE_IDLE
        };
        if !rng.random_bool(ai_reply_chance) {
            return None;
        }
        candidates.retain(|bot| {
            bot.identity
                .lock()
                .ok()
                .and_then(|v| v.clone())
                .is_none_or(|id| id != sender_identity)
        });
    } else if sender_identity == "System" || bot_identities.contains(sender_identity) {
        return None;
    }

    candidates.choose(&mut rng).copied()
}

fn maybe_start_proactive_ai_chat(
    ai_bots: &[AiBotRuntime],
    state: &SharedState,
    ai_reply_tx: &Sender<AiGeneratedReply>,
    pending_ai_replies: &HashMap<String, VecDeque<String>>,
    online_human_count: usize,
    last_chat_activity: &mut Instant,
    last_proactive_attempt: &mut Instant,
) {
    if ai_bots.len() < 2 {
        return;
    }
    if pending_ai_replies.values().any(|q| !q.is_empty()) {
        return;
    }

    let cooldown = Duration::from_secs(AI_PROACTIVE_COOLDOWN_SECS);
    if last_proactive_attempt.elapsed() < cooldown {
        return;
    }
    *last_proactive_attempt = Instant::now();

    let idle_window = Duration::from_secs(AI_PROACTIVE_IDLE_SECS);
    if last_chat_activity.elapsed() < idle_window {
        return;
    }

    let mut rng = rng();
    if !rng.random_bool(AI_PROACTIVE_START_CHANCE) {
        return;
    }

    let mut online_bots: Vec<&AiBotRuntime> = ai_bots
        .iter()
        .filter(|bot| bot.online.load(Ordering::SeqCst))
        .collect();
    if online_bots.len() < 2 {
        return;
    }

    online_bots.shuffle(&mut rng);
    let starter = online_bots[0];
    let target = online_bots[1];
    let opening = proactive_opening_prompt(&target.profile.name, online_human_count > 0);

    request_bot_reply(state, starter.profile.clone(), opening, ai_reply_tx.clone());
    *last_chat_activity = Instant::now();
}

fn proactive_opening_prompt(target_name: &str, has_humans_online: bool) -> String {
    const TOPICS: &[&str] = &[
        "comida da taverna",
        "chuva no reino",
        "boatos de missao",
        "equipamento novo",
        "musica de viagem",
        "rumores da cidade",
        "preco das pocoes",
        "historia engracada do dia",
    ];
    let mut rng = rng();
    let topic = TOPICS.choose(&mut rng).copied().unwrap_or("algo leve");

    if has_humans_online {
        return format!(
            "Escreva UMA mensagem curta e casual para {} sobre {}. \
            Tom de chat entre amigos, sem cumprimento formal e sem oferecer ajuda.",
            target_name, topic
        );
    }

    format!(
        "Escreva UMA mensagem curta puxando papo com {} sobre {}. \
        So conversa leve entre personagens, sem fala de assistente.",
        target_name, topic
    )
}

fn find_directed_bot<'a>(
    ai_bots: &'a [AiBotRuntime],
    sender_identity: &str,
    message_text: &str,
) -> Option<&'a AiBotRuntime> {
    let lowered_message = message_text.to_lowercase();

    ai_bots
        .iter()
        .filter(|bot| bot.online.load(Ordering::SeqCst))
        .filter(|bot| {
            bot.identity
                .lock()
                .ok()
                .and_then(|v| v.clone())
                .is_none_or(|id| id != sender_identity)
        })
        .find(|bot| lowered_message.contains(&bot.profile.name.to_lowercase()))
}
