use std::sync::Arc;

use spacetimedb_sdk::{Table, TableWithPrimaryKey};

use crate::{
    module_bindings::{DbConnection, MessageTableAccess, RemoteTables, UserTableAccess},
    state::{SharedState, update_state},
    ui::ui_state::{UiMessage, UiUser},
};

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
        s.ui.messages = messages;
        s.ui.users = users;
    });
}
