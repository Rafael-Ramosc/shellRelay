use std::sync::{Arc, Mutex};

use crate::ui::ui_state::UiState;

/// Papel de cada item salvo no histórico local da IA.
#[derive(Clone)]
pub enum AiRole {
    User,
    Assistant,
}

/// Entrada de histórico enviada ao modelo para manter continuidade.
#[derive(Clone)]
pub struct AiHistoryEntry {
    pub role: AiRole,
    pub content: String,
}

/// Estado global compartilhado pela aplicação TUI.
///
/// Tudo que a UI precisa para renderizar e reagir a eventos passa por aqui.
#[derive(Clone, Default)]
pub struct AppState {
    /// Estado puramente visual (tela, input, listas etc.).
    pub ui: UiState,
    /// Identity da conexão principal no SpacetimeDB.
    pub my_identity: Option<String>,
    /// Status de conectividade com o backend.
    pub status: bool,
    /// Histórico curto de contexto usado nas chamadas da IA.
    pub ai_history: Vec<AiHistoryEntry>,
}

/// Tipo utilitário para compartilhar `AppState` entre threads.
pub type SharedState = Arc<Mutex<AppState>>;

/// Copia imutável do estado para renderização sem segurar lock por muito tempo.
pub fn snapshot_state(state: &SharedState) -> AppState {
    state.lock().map(|s| s.clone()).unwrap_or_default()
}

/// Atualiza o estado de forma centralizada e segura.
pub fn update_state(state: &SharedState, f: impl FnOnce(&mut AppState)) {
    if let Ok(mut s) = state.lock() {
        f(&mut s);
    }
}
