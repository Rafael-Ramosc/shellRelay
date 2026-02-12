use std::{
    collections::HashMap,
    env,
    sync::{Arc, mpsc::Sender},
    thread,
};

use ollama_rs::{
    Ollama,
    generation::chat::{ChatMessage, request::ChatMessageRequest},
};

use crate::{
    state::{AiHistoryEntry, AiRole, AppState, SharedState, update_state},
    sync::add_local_system_message,
};

/// Nome usado pela conexão-bot no chat.
pub const AI_BOT_NAME: &str = "Ai";
/// Configuração padrão de acesso ao Ollama local.
const DEFAULT_MODEL: &str = "mistral:7b";
const DEFAULT_HOST: &str = "http://127.0.0.1";
const DEFAULT_PORT: u16 = 11434;
/// Quantidade máxima de turns guardados no histórico local da IA.
const MAX_HISTORY_ENTRIES: usize = 20;
/// Janela de contexto de mensagens recentes do chat.
const MAX_CONTEXT_MESSAGES: usize = 16;
/// Limite de tamanho por mensagem antes de injetar no prompt contextual.
const MAX_CONTEXT_MESSAGE_LEN: usize = 320;
/// Prompt base do comportamento do modelo.
const SYSTEM_PROMPT: &str = "You are a chat participant";

/// Dispara a geração da IA sem bloquear a UI.
///
/// Fluxo:
/// 1) atualiza histórico local,
/// 2) captura snapshot de contexto do chat,
/// 3) chama Ollama em thread separada,
/// 4) devolve a resposta via `reply_tx` para publicação pelo bot.
pub fn request_mage_reply(state: &SharedState, user_text: String, reply_tx: Sender<String>) {
    let (history, prompt_context) = {
        let mut snapshot = Vec::new();
        let mut prompt_context = AiPromptContext::default();
        update_state(state, |s| {
            s.ai_history.push(AiHistoryEntry {
                role: AiRole::User,
                content: user_text.clone(),
            });
            trim_history(&mut s.ai_history);
            snapshot = s.ai_history.clone();
            prompt_context = build_prompt_context(s);
        });
        (snapshot, prompt_context)
    };

    let state = Arc::clone(state);
    thread::spawn(move || {
        // Toda chamada de rede fica fora da thread principal da TUI.
        let result = fetch_ollama_reply(history, prompt_context);
        match result {
            Ok(reply) => {
                update_state(&state, |s| {
                    s.ai_history.push(AiHistoryEntry {
                        role: AiRole::Assistant,
                        content: reply.clone(),
                    });
                    trim_history(&mut s.ai_history);
                });

                if let Err(err) = reply_tx.send(reply) {
                    add_local_system_message(
                        &state,
                        "System",
                        format!("Erro ao enfileirar resposta da IA: {err}"),
                    );
                }
            }
            Err(err) => {
                // Erros da IA viram mensagem local para facilitar diagnóstico em runtime.
                add_local_system_message(&state, "System", format!("Erro ao chamar Ollama: {err}"));
            }
        }
    });
}

/// Constrói request do Ollama com:
/// - prompt base do sistema,
/// - prompt contextual do estado atual do chat,
/// - histórico resumido user/assistant.
fn fetch_ollama_reply(
    history: Vec<AiHistoryEntry>,
    prompt_context: AiPromptContext,
) -> Result<String, String> {
    let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let host = env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = env::var("OLLAMA_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("falha ao iniciar runtime async: {e}"))?;

    // Usa `block_on` para manter API síncrona neste módulo e evitar propagar async
    // por toda a aplicação.
    runtime.block_on(async move {
        let client = Ollama::new(host, port);
        let mut messages = vec![
            ChatMessage::system(SYSTEM_PROMPT.to_string()),
            ChatMessage::system(build_context_system_prompt(&prompt_context)),
        ];

        for entry in history {
            if entry.content.trim().is_empty() {
                continue;
            }

            let message = match entry.role {
                AiRole::User => ChatMessage::user(entry.content),
                AiRole::Assistant => ChatMessage::assistant(entry.content),
            };
            messages.push(message);
        }

        let request = ChatMessageRequest::new(model, messages);
        let response = client
            .send_chat_messages(request)
            .await
            .map_err(|e| e.to_string())?;

        let reply = response.message.content.trim().to_string();
        if reply.is_empty() {
            return Err("resposta vazia do modelo".to_string());
        }
        Ok(reply)
    })
}

/// Mantém histórico com tamanho fixo para não crescer indefinidamente.
fn trim_history(history: &mut Vec<AiHistoryEntry>) {
    if history.len() > MAX_HISTORY_ENTRIES {
        let to_drop = history.len() - MAX_HISTORY_ENTRIES;
        history.drain(0..to_drop);
    }
}

/// Dados consolidados do estado atual para injeção no prompt.
#[derive(Default)]
struct AiPromptContext {
    requester_identity: String,
    requester_name: String,
    online_users: Vec<String>,
    recent_messages: Vec<String>,
}

/// Extrai do estado as informações úteis para resposta contextual da IA.
fn build_prompt_context(state: &AppState) -> AiPromptContext {
    let users_by_identity: HashMap<&str, &str> = state
        .ui
        .users
        .iter()
        .map(|u| (u.identity.as_str(), u.name.as_str()))
        .collect();

    let requester_identity = state.my_identity.clone().unwrap_or_default();
    // Preferimos nome amigável; fallback para identity curta.
    let requester_name = users_by_identity
        .get(requester_identity.as_str())
        .copied()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| short_identity(&requester_identity));

    let online_users = state
        .ui
        .users
        .iter()
        .filter(|u| u.online)
        .map(|u| {
            let name = if u.name.trim().is_empty() {
                short_identity(&u.identity)
            } else {
                u.name.clone()
            };
            format!("{name} ({})", short_identity(&u.identity))
        })
        .collect();

    // Captura as mensagens mais recentes em ordem cronológica.
    let mut recent_messages: Vec<String> = state
        .ui
        .messages
        .iter()
        .rev()
        .filter(|m| !m.text.trim().is_empty())
        .take(MAX_CONTEXT_MESSAGES)
        .map(|m| {
            let sender = users_by_identity
                .get(m.sender.as_str())
                .copied()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| short_identity(&m.sender));
            let text = truncate_for_context(&m.text, MAX_CONTEXT_MESSAGE_LEN);
            if m.sent_at.trim().is_empty() {
                format!("{sender}: {text}")
            } else {
                format!("[{}] {sender}: {text}", m.sent_at.trim())
            }
        })
        .collect();
    recent_messages.reverse();

    AiPromptContext {
        requester_identity,
        requester_name,
        online_users,
        recent_messages,
    }
}

/// Monta um prompt de sistema com o retrato atual da sala.
fn build_context_system_prompt(context: &AiPromptContext) -> String {
    let online_users = if context.online_users.is_empty() {
        "nenhum".to_string()
    } else {
        context.online_users.join(", ")
    };

    let recent_messages = if context.recent_messages.is_empty() {
        "nenhuma".to_string()
    } else {
        context.recent_messages.join("\n")
    };

    format!(
        "Contexto do chat atual:\n- Usuario que te chamou: {} ({})\n- Usuarios online: {}\n- Ultimas mensagens no chat (ordem cronologica):\n{}\nUse esse contexto para responder de forma coerente.",
        context.requester_name, context.requester_identity, online_users, recent_messages
    )
}

/// Exibe identity reduzida para manter legibilidade no prompt/contexto.
fn short_identity(identity: &str) -> String {
    const MAX: usize = 18;
    if identity.len() <= MAX {
        return identity.to_string();
    }

    let head = &identity[..10.min(identity.len())];
    let tail = &identity[identity.len().saturating_sub(6)..];
    format!("{head}..{tail}")
}

/// Remove quebras e limita texto para não explodir tokens no prompt.
fn truncate_for_context(text: &str, max_chars: usize) -> String {
    let clean = text.replace('\n', " ").trim().to_string();
    let char_count = clean.chars().count();
    if char_count <= max_chars {
        return clean;
    }

    let truncated: String = clean.chars().take(max_chars).collect();
    format!("{truncated}...")
}
