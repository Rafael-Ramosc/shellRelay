use std::collections::{HashMap, HashSet};

use crate::state::AppState;

use super::{MAX_CONTEXT_MESSAGE_LEN, MAX_CONTEXT_MESSAGES, short_identity, truncate_for_context};

/// Dados consolidados do estado atual para injeção no prompt.
#[derive(Default)]
pub(crate) struct AiPromptContext {
    pub(crate) requester_identity: String,
    pub(crate) requester_name: String,
    pub(crate) online_users: Vec<String>,
    pub(crate) recent_messages: Vec<String>,
}

/// Extrai do estado as informações úteis para resposta contextual da IA.
pub(crate) fn build_prompt_context(state: &AppState) -> AiPromptContext {
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

    let online_identities: HashSet<&str> = state
        .ui
        .users
        .iter()
        .filter(|u| u.online)
        .map(|u| u.identity.as_str())
        .collect();

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
        .filter(|m| m.sender == "System" || online_identities.contains(m.sender.as_str()))
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

pub(crate) fn build_context_system_prompt(context: &AiPromptContext) -> String {
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
        "Contexto do chat atual:\n- Usuario que te chamou: {} ({})\n- Usuarios online: {}\n- Ultimas mensagens no chat (ordem cronologica):\n{}\nFoque apenas em quem esta online agora e nao puxe conversa com usuarios offline.\nUse esse contexto para responder de forma coerente.",
        context.requester_name, context.requester_identity, online_users, recent_messages
    )
}
