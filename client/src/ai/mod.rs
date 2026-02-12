use std::{
    env,
    sync::{Arc, mpsc::Sender},
    thread,
};

use ollama_rs::{
    Ollama,
    generation::chat::{ChatMessage, request::ChatMessageRequest},
    models::ModelOptions,
};

use crate::{
    state::{AiHistoryEntry, AiRole, AppState, SharedState, update_state},
    sync::add_local_system_message,
};

mod bots;
mod config;
mod context;
mod text;

use context::AiPromptContext;

pub use bots::{
    AiBotProfile, FANTASY_NAMES, RPG_PROFESSIONS, generate_bot_profiles, profession_roleplay_style,
};
pub use config::{
    AI_BOT_COUNT, AI_PROACTIVE_COOLDOWN_SECS, AI_PROACTIVE_IDLE_SECS, AI_PROACTIVE_START_CHANCE,
    AI_TO_AI_REPLY_CHANCE_IDLE, AI_TO_AI_REPLY_CHANCE_WITH_HUMANS, MAX_AI_CHAIN_MESSAGES,
};

/// Configuração padrão de acesso ao Ollama local.
const DEFAULT_MODEL: &str = "mistral:7b";
const DEFAULT_HOST: &str = "http://127.0.0.1";
const DEFAULT_PORT: u16 = 11434;
/// Quantidade máxima de turns guardados no histórico local da IA.
const MAX_HISTORY_ENTRIES: usize = 12;
/// Janela de contexto de mensagens recentes do chat.
const MAX_CONTEXT_MESSAGES: usize = 16;
/// Limite de tamanho por mensagem antes de injetar no prompt contextual.
const MAX_CONTEXT_MESSAGE_LEN: usize = 320;
/// Limite duro de tamanho da resposta final enviada ao chat.
const MAX_REPLY_CHARS: usize = 220;

const BASE_SYSTEM_PROMPT: &str = "Voce e uma pessoa real participando do chat em portugues do Brasil. \
Responda de forma natural, curta e direta, como conversa de dia a dia. \
Use no maximo 2 frases curtas. \
Fale no mesmo tom da conversa, sem discurso pronto e sem frases de efeito. \
Evite comecar sempre com cumprimento e evite repetir o nome da pessoa em toda mensagem. \
Nao faca autoapresentacao, nao de mensagem do dia e nao vire narrador. \
Nao ofereca ajuda do nada; so ajude se pedirem de forma direta. \
Evite formalidade exagerada (ex.: \"um prazer\", \"estou a disposicao\", \"agendar reuniao\"). \
Nao use linguagem teatral, mistica, floreada ou palestrinha. \
Nao diga que e IA, modelo ou assistente virtual. \
Evite repetir a pergunta do usuario e evite repetir assunto sem novidade.";

#[derive(Clone, Debug)]
pub struct AiGeneratedReply {
    pub bot_name: String,
    pub text: String,
}

/// Dispara a geração da IA para um bot específico sem bloquear a UI.
pub fn request_bot_reply(
    state: &SharedState,
    bot: AiBotProfile,
    incoming_text: String,
    reply_tx: Sender<AiGeneratedReply>,
) {
    let (history, prompt_context) = {
        let mut snapshot = Vec::new();
        let mut prompt_context = AiPromptContext::default();
        let history_key = bot.name.clone();
        update_state(state, |s| {
            let bot_history = s.ai_histories.entry(history_key).or_default();
            bot_history.push(AiHistoryEntry {
                role: AiRole::User,
                content: incoming_text.clone(),
            });
            trim_history(bot_history);
            snapshot = bot_history.clone();
            prompt_context = build_prompt_context(s);
        });
        (snapshot, prompt_context)
    };

    let state = Arc::clone(state);
    thread::spawn(move || {
        let result = fetch_ollama_reply(history, prompt_context, &bot);
        match result {
            Ok(reply) => {
                let history_key = bot.name.clone();
                update_state(&state, |s| {
                    let bot_history = s.ai_histories.entry(history_key).or_default();
                    bot_history.push(AiHistoryEntry {
                        role: AiRole::Assistant,
                        content: reply.clone(),
                    });
                    trim_history(bot_history);
                });

                if let Err(err) = reply_tx.send(AiGeneratedReply {
                    bot_name: bot.name.clone(),
                    text: reply,
                }) {
                    add_local_system_message(
                        &state,
                        "System",
                        format!("Erro ao enfileirar resposta da IA ({}): {err}", bot.name),
                    );
                }
            }
            Err(err) => {
                add_local_system_message(
                    &state,
                    "System",
                    format!("Erro ao chamar Ollama ({}): {err}", bot.name),
                );
            }
        }
    });
}

fn fetch_ollama_reply(
    history: Vec<AiHistoryEntry>,
    prompt_context: AiPromptContext,
    bot: &AiBotProfile,
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

    runtime.block_on(async move {
        let client = Ollama::new(host, port);
        let roleplay_prompt = build_roleplay_system_prompt(bot);
        let mut messages = vec![
            ChatMessage::system(BASE_SYSTEM_PROMPT.to_string()),
            ChatMessage::system(roleplay_prompt),
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

        let options = ModelOptions::default()
            .num_predict(70)
            .temperature(0.85)
            .top_p(0.95)
            .repeat_penalty(1.35);
        let request = ChatMessageRequest::new(model, messages).options(options);
        let response = client
            .send_chat_messages(request)
            .await
            .map_err(|e| e.to_string())?;

        let reply = normalize_reply(&response.message.content);
        if reply.is_empty() {
            return Err("resposta vazia do modelo".to_string());
        }
        Ok(reply)
    })
}

fn build_roleplay_system_prompt(bot: &AiBotProfile) -> String {
    format!(
        "Seu nome neste chat e {} e sua profissao de fantasia e {}. {} \
        Voce e apenas mais uma pessoa no chat, nao um guia, tutor ou atendente. \
        Traga esse estilo de forma leve, sem personagem caricaturado. \
        Mantenha respostas curtas e naturais.",
        bot.name,
        bot.profession,
        profession_roleplay_style(&bot.profession)
    )
}

fn trim_history(history: &mut Vec<AiHistoryEntry>) {
    if history.len() > MAX_HISTORY_ENTRIES {
        let to_drop = history.len() - MAX_HISTORY_ENTRIES;
        history.drain(0..to_drop);
    }
}

fn build_prompt_context(state: &AppState) -> AiPromptContext {
    context::build_prompt_context(state)
}

fn build_context_system_prompt(context: &AiPromptContext) -> String {
    context::build_context_system_prompt(context)
}

fn short_identity(identity: &str) -> String {
    text::short_identity(identity)
}

fn truncate_for_context(text: &str, max_chars: usize) -> String {
    text::truncate_for_context(text, max_chars)
}

fn normalize_reply(text: &str) -> String {
    text::normalize_reply(text)
}

#[cfg(test)]
#[path = "../tests/ai_tests.rs"]
mod tests;
