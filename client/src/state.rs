use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default)]
pub enum InputMode {
    #[default]
    Message,
    Name,
}

impl InputMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Message => "Mensagem",
            Self::Name => "Nome",
        }
    }

    pub fn toggle(&mut self) {
        *self = match *self {
            Self::Message => Self::Name,
            Self::Name => Self::Message,
        };
    }
}

#[derive(Clone, Default)]
pub struct UiMessage {
    pub id: u64,
    pub sender: String,
    pub text: String,
    pub sent_at: String,
}

#[derive(Clone, Default)]
pub struct UiUser {
    pub identity: String,
    pub name: String,
    pub online: bool,
}

#[derive(Clone, Default)]
pub struct AppState {
    pub messages: Vec<UiMessage>,
    pub users: Vec<UiUser>,
    pub input: String,
    pub input_mode: InputMode,
    pub my_identity: Option<String>,
    pub status: bool,
    pub should_quit: bool,
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
