use std::{collections::HashMap, sync::Arc};

use spacetimedb_sdk::{Table, TableWithPrimaryKey};

use crate::{
    module_bindings::{DbConnection, MessageTableAccess, RemoteTables, UserTableAccess},
    state::{SharedState, update_state},
    ui::ui_state::{UiMessage, UiUser},
};

const SYSTEM_MESSAGE_ID_BASE: u64 = 1_000_000_000_000_000_000;
const MAX_SYSTEM_MESSAGES: usize = 200;

pub fn register_table_callbacks(conn: &DbConnection, state: &SharedState) {
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

pub fn sync_from_tables(db: &RemoteTables, state: &SharedState) {
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
        let previous_users = s.ui.users.clone();
        let mut presence_events: Vec<String> = Vec::new();

        if s.ui.users_presence_initialized {
            let previous_online: HashMap<&str, bool> = previous_users
                .iter()
                .map(|u| (u.identity.as_str(), u.online))
                .collect();
            let current_online: HashMap<&str, bool> =
                users.iter().map(|u| (u.identity.as_str(), u.online)).collect();

            for user in &users {
                let was_online = previous_online
                    .get(user.identity.as_str())
                    .copied()
                    .unwrap_or(false);
                if user.online && !was_online {
                    presence_events.push(format!("{} connected", display_user_name(user)));
                }
            }

            for user in &previous_users {
                let is_online = current_online
                    .get(user.identity.as_str())
                    .copied()
                    .unwrap_or(false);
                if user.online && !is_online {
                    presence_events.push(format!("{} disconnected", display_user_name(user)));
                }
            }
        }
        s.ui.users_presence_initialized = true;

        for text in presence_events {
            let id = SYSTEM_MESSAGE_ID_BASE.saturating_add(s.ui.next_system_message_id);
            s.ui.next_system_message_id = s.ui.next_system_message_id.saturating_add(1);
            s.ui.system_messages.push(UiMessage {
                id,
                sender: "System".to_string(),
                text,
                sent_at: String::new(),
            });
        }
        if s.ui.system_messages.len() > MAX_SYSTEM_MESSAGES {
            let to_drop = s.ui.system_messages.len() - MAX_SYSTEM_MESSAGES;
            s.ui.system_messages.drain(0..to_drop);
        }

        messages.extend(s.ui.system_messages.iter().cloned());
        messages.sort_by_key(|m| m.id);

        s.ui.messages = messages;
        s.ui.users = users;
        s.ui.users_scroll = s.ui.users_scroll.min(s.ui.users.len().saturating_sub(1));
    });
}

fn display_user_name(user: &UiUser) -> String {
    if !user.name.trim().is_empty() {
        return user.name.clone();
    }

    short_identity(&user.identity)
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
