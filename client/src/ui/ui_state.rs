/// Telas principais da aplicação.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum UiScreen {
    #[default]
    MainMenu,
    Chat,
}

/// Popups modais exibidos sobre a tela atual.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiPopup {
    ChooseName,
    Soon,
}

/// Itens disponíveis no menu principal.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MainMenuItem {
    EnterChat,
    Options,
    Exit,
}

impl MainMenuItem {
    pub const ALL: [Self; 3] = [Self::EnterChat, Self::Options, Self::Exit];

    pub fn label(self) -> &'static str {
        match self {
            Self::EnterChat => "Enter chat",
            Self::Options => "Options",
            Self::Exit => "Exit",
        }
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::EnterChat)
    }
}

#[derive(Clone, Default)]
pub struct UiMessage {
    /// ID para ordenação estável no chat.
    pub id: u64,
    /// Identity de quem enviou (ou "System" em mensagens locais).
    pub sender: String,
    pub text: String,
    /// Timestamp textual recebido do backend.
    pub sent_at: String,
}

#[derive(Clone, Default)]
pub struct UiUser {
    pub identity: String,
    pub name: String,
    pub online: bool,
}

#[derive(Clone, Default)]
pub struct UiState {
    /// Tela em foco.
    pub screen: UiScreen,
    /// Popup modal atual, se existir.
    pub popup: Option<UiPopup>,
    /// Índice selecionado no menu principal.
    pub menu_selected: usize,
    /// Distância do final da lista de mensagens (0 = "travado" nas mais novas).
    pub messages_scroll_from_bottom: usize,
    /// Offset vertical da lista de usuários.
    pub users_scroll: usize,
    /// Evita disparar eventos de presença antes da primeira sincronização.
    pub users_presence_initialized: bool,
    /// Contador para IDs de mensagens locais do sistema.
    pub next_system_message_id: u64,
    /// Mensagens locais não persistidas no servidor.
    pub system_messages: Vec<UiMessage>,
    /// Lista renderizada no painel de mensagens (backend + locais).
    pub messages: Vec<UiMessage>,
    pub users: Vec<UiUser>,
    /// Buffer do input atual.
    pub input: String,
    /// Flag global de encerramento do app.
    pub should_quit: bool,
}
