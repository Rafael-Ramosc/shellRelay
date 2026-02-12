#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum UiScreen {
    #[default]
    MainMenu,
    Chat,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiPopup {
    ChooseName,
    Soon,
}

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
pub struct UiState {
    pub screen: UiScreen,
    pub popup: Option<UiPopup>,
    pub menu_selected: usize,
    pub users_scroll: usize,
    pub users_presence_initialized: bool,
    pub next_system_message_id: u64,
    pub system_messages: Vec<UiMessage>,
    pub messages: Vec<UiMessage>,
    pub users: Vec<UiUser>,
    pub input: String,
    pub should_quit: bool,
}
