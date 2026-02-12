use std::sync::{Arc, Mutex};

use crate::{
    state::AppState,
    ui::ui_state::{UiMessage, UiUser},
};

use super::{SYSTEM_MESSAGE_ID_BASE, add_local_system_message, display_user_name, short_identity};

#[test]
fn short_identity_truncates_long_values() {
    assert_eq!(short_identity("abc"), "abc");
    assert_eq!(
        short_identity("abcdefghijklmnopqrstuvwxyz"),
        "abcdefghij..uvwxyz"
    );
}

#[test]
fn display_user_name_prefers_name_and_falls_back_to_identity() {
    let named = UiUser {
        identity: "id_user".to_string(),
        name: "Rafael".to_string(),
        online: true,
    };
    let unnamed = UiUser {
        identity: "abcdefghijklmnopqrstuvwxyz".to_string(),
        name: "   ".to_string(),
        online: true,
    };

    assert_eq!(display_user_name(&named), "Rafael");
    assert_eq!(display_user_name(&unnamed), "abcdefghij..uvwxyz");
}

#[test]
fn add_local_system_message_appends_and_rebuilds_message_list() {
    let state = Arc::new(Mutex::new(AppState::default()));
    {
        let mut guard = state.lock().expect("lock state");
        guard.ui.messages.push(UiMessage {
            id: 42,
            sender: "id_user".to_string(),
            text: "mensagem remota".to_string(),
            sent_at: "2026-02-12T10:00:00Z".to_string(),
        });
    }

    add_local_system_message(&state, "System", "hello");

    let guard = state.lock().expect("lock state");
    assert_eq!(guard.ui.system_messages.len(), 1);
    assert_eq!(guard.ui.system_messages[0].sender, "System");
    assert_eq!(guard.ui.system_messages[0].text, "hello");
    assert_eq!(guard.ui.messages.len(), 2);
    assert_eq!(guard.ui.messages[0].id, 42);
    assert!(guard.ui.messages[1].id >= SYSTEM_MESSAGE_ID_BASE);
}
