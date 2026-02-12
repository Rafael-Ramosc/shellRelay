use std::sync::{Arc, Mutex};

use crate::ui::ui_state::UiState;

#[derive(Clone, Default)]
pub struct AppState {
    pub ui: UiState,
    pub my_identity: Option<String>,
    pub status: bool,
}

pub type SharedState = Arc<Mutex<AppState>>;

pub fn snapshot_state(state: &SharedState) -> AppState {
    state.lock().map(|s| s.clone()).unwrap_or_default()
}

pub fn update_state(state: &SharedState, f: impl FnOnce(&mut AppState)) {
    if let Ok(mut s) = state.lock() {
        f(&mut s);
    }
}
