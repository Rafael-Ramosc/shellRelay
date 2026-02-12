use crate::ui::ui_state::{UiMessage, UiUser};

use super::{
    AppState, MAX_REPLY_CHARS, build_context_system_prompt, build_prompt_context, normalize_reply,
    short_identity, truncate_for_context,
};

#[test]
fn short_identity_returns_original_when_small_and_truncates_when_large() {
    assert_eq!(short_identity("abc123"), "abc123");
    assert_eq!(
        short_identity("abcdefghijklmnopqrstuvwxyz"),
        "abcdefghij..uvwxyz"
    );
}

#[test]
fn truncate_for_context_normalizes_newlines_and_limits_size() {
    let input = "linha 1\nlinha 2";
    assert_eq!(truncate_for_context(input, 50), "linha 1 linha 2");
    assert_eq!(truncate_for_context("abcdefghij", 5), "abcde...");
}

#[test]
fn build_prompt_context_uses_requester_name_online_users_and_recent_messages() {
    let mut state = AppState {
        my_identity: Some("id_rafael".to_string()),
        ..Default::default()
    };
    state.ui.users = vec![
        UiUser {
            identity: "id_rafael".to_string(),
            name: "Rafael".to_string(),
            online: true,
        },
        UiUser {
            identity: "id_ai".to_string(),
            name: "Ai".to_string(),
            online: true,
        },
        UiUser {
            identity: "id_offline".to_string(),
            name: "Offline".to_string(),
            online: false,
        },
    ];
    state.ui.messages = vec![
        UiMessage {
            id: 1,
            sender: "id_rafael".to_string(),
            text: "Oi".to_string(),
            sent_at: "2026-02-12T13:44:00Z".to_string(),
        },
        UiMessage {
            id: 2,
            sender: "id_ai".to_string(),
            text: "Ola".to_string(),
            sent_at: "2026-02-12T13:45:00Z".to_string(),
        },
    ];

    let ctx = build_prompt_context(&state);
    assert_eq!(ctx.requester_name, "Rafael");
    assert_eq!(ctx.requester_identity, "id_rafael");
    assert_eq!(ctx.online_users.len(), 2);
    assert!(ctx.online_users.iter().any(|u| u.contains("Rafael")));
    assert!(ctx.online_users.iter().any(|u| u.contains("Ai")));
    assert_eq!(ctx.recent_messages.len(), 2);
    assert!(ctx.recent_messages[0].contains("Rafael"));
    assert!(ctx.recent_messages[1].contains("Ai"));
}

#[test]
fn build_prompt_context_ignores_recent_messages_from_offline_users() {
    let mut state = AppState {
        my_identity: Some("id_rafael".to_string()),
        ..Default::default()
    };
    state.ui.users = vec![
        UiUser {
            identity: "id_rafael".to_string(),
            name: "Rafael".to_string(),
            online: true,
        },
        UiUser {
            identity: "id_online".to_string(),
            name: "Lia".to_string(),
            online: true,
        },
        UiUser {
            identity: "id_offline".to_string(),
            name: "Teste".to_string(),
            online: false,
        },
    ];
    state.ui.messages = vec![
        UiMessage {
            id: 1,
            sender: "id_offline".to_string(),
            text: "msg antiga".to_string(),
            sent_at: String::new(),
        },
        UiMessage {
            id: 2,
            sender: "id_online".to_string(),
            text: "msg atual".to_string(),
            sent_at: String::new(),
        },
    ];

    let ctx = build_prompt_context(&state);
    assert_eq!(ctx.recent_messages.len(), 1);
    assert!(ctx.recent_messages[0].contains("Lia"));
}

#[test]
fn context_system_prompt_contains_core_fields() {
    let mut state = AppState {
        my_identity: Some("id_rafael".to_string()),
        ..Default::default()
    };
    state.ui.users.push(UiUser {
        identity: "id_rafael".to_string(),
        name: "Rafael".to_string(),
        online: true,
    });
    state.ui.messages.push(UiMessage {
        id: 1,
        sender: "id_rafael".to_string(),
        text: "Teste".to_string(),
        sent_at: String::new(),
    });

    let ctx = build_prompt_context(&state);
    let prompt = build_context_system_prompt(&ctx);
    assert!(prompt.contains("Usuario que te chamou"));
    assert!(prompt.contains("Rafael"));
    assert!(prompt.contains("Usuarios online"));
    assert!(prompt.contains("Ultimas mensagens no chat"));
}

#[test]
fn normalize_reply_compacts_and_limits_to_two_sentences() {
    let raw = "Oi,\n tudo bem?   Eu estou bem. Vamos conversar mais um pouco. Terceira frase.";
    let normalized = normalize_reply(raw);
    assert_eq!(normalized, "Oi, tudo bem? Eu estou bem.");
}

#[test]
fn normalize_reply_truncates_very_long_output() {
    let long = "a".repeat(400);
    let normalized = normalize_reply(&long);
    assert!(normalized.chars().count() <= MAX_REPLY_CHARS + 3);
}
