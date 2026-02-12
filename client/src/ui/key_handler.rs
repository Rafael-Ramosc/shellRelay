use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    module_bindings::{
        DbConnection, send_message as SendMessageReducerExt, set_name as SetNameReducerExt,
    },
    state::{SharedState, update_state},
    ui::ui_state::{MainMenuItem, UiPopup, UiScreen},
};

pub fn handle_key_event(
    key: KeyEvent,
    conn: &DbConnection,
    state: &SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    if key.kind != event::KeyEventKind::Press {
        return Ok(());
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        update_state(state, |s| s.ui.should_quit = true);
        return Ok(());
    }

    if key.code == KeyCode::Char('q') {
        update_state(state, |s| s.ui.should_quit = true);
        return Ok(());
    }

    let (screen, popup) = {
        let guard = state.lock().expect("state poisoned");
        (guard.ui.screen, guard.ui.popup)
    };

    if let Some(popup) = popup {
        return handle_popup_key(key, popup, conn, state);
    }

    match screen {
        UiScreen::MainMenu => handle_menu_key(key, state),
        UiScreen::Chat => handle_chat_key(key, conn, state),
    }
}

fn handle_menu_key(key: KeyEvent, state: &SharedState) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Up => {
            update_state(state, |s| {
                if s.ui.menu_selected > 0 {
                    s.ui.menu_selected -= 1;
                }
            });
        }
        KeyCode::Down => {
            update_state(state, |s| {
                if s.ui.menu_selected + 1 < MainMenuItem::ALL.len() {
                    s.ui.menu_selected += 1;
                }
            });
        }
        KeyCode::Enter => {
            update_state(state, |s| match MainMenuItem::from_index(s.ui.menu_selected) {
                MainMenuItem::EnterChat => {
                    s.ui.popup = Some(UiPopup::ChooseName);
                    s.ui.input.clear();
                }
                MainMenuItem::Options => {
                    s.ui.popup = Some(UiPopup::Soon);
                }
                MainMenuItem::Exit => {
                    s.ui.should_quit = true;
                }
            });
        }
        _ => {}
    }

    Ok(())
}

fn handle_popup_key(
    key: KeyEvent,
    popup: UiPopup,
    conn: &DbConnection,
    state: &SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    match popup {
        UiPopup::ChooseName => match key.code {
            KeyCode::Esc => {
                update_state(state, |s| {
                    s.ui.popup = None;
                    s.ui.input.clear();
                });
            }
            KeyCode::Backspace => {
                update_state(state, |s| {
                    s.ui.input.pop();
                });
            }
            KeyCode::Enter => {
                let name = {
                    let mut guard = state.lock().expect("state poisoned");
                    let text = guard.ui.input.trim().to_string();
                    guard.ui.input.clear();
                    text
                };

                if name.is_empty() {
                    return Ok(());
                }

                let reducer_res = conn.reducers.set_name(name);
                if reducer_res.is_ok() {
                    update_state(state, |s| {
                        s.ui.popup = None;
                        s.ui.screen = UiScreen::Chat;
                        s.ui.users_scroll = 0;
                    });
                } else {
                    update_state(state, |s| {
                        s.status = false;
                    });
                }
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    update_state(state, |s| s.ui.input.push(c));
                }
            }
            _ => {}
        },
        UiPopup::Soon => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                update_state(state, |s| s.ui.popup = None);
            }
            _ => {}
        },
    }

    Ok(())
}

fn handle_chat_key(
    key: KeyEvent,
    conn: &DbConnection,
    state: &SharedState,
) -> Result<(), Box<dyn std::error::Error>> {
    match key.code {
        KeyCode::Up => {
            update_state(state, |s| {
                if s.ui.users_scroll > 0 {
                    s.ui.users_scroll -= 1;
                }
            });
        }
        KeyCode::Down => {
            update_state(state, |s| {
                if s.ui.users_scroll + 1 < s.ui.users.len() {
                    s.ui.users_scroll += 1;
                }
            });
        }
        KeyCode::F(1) => {
            update_state(state, |s| {
                s.ui.screen = UiScreen::MainMenu;
                s.ui.popup = None;
                s.ui.input.clear();
            });
        }
        KeyCode::Esc => {
            update_state(state, |s| s.ui.input.clear());
        }
        KeyCode::Backspace => {
            update_state(state, |s| {
                s.ui.input.pop();
            });
        }
        KeyCode::Enter => {
            let text = {
                let mut guard = state.lock().expect("state poisoned");
                let text = guard.ui.input.trim().to_string();
                guard.ui.input.clear();
                text
            };

            if text.is_empty() {
                return Ok(());
            }

            let reducer_res = conn.reducers.send_message(text);
            if reducer_res.is_err() {
                update_state(state, |s| {
                    s.status = false;
                });
            }
        }
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::ALT)
            {
                update_state(state, |s| s.ui.input.push(c));
            }
        }
        _ => {}
    }

    Ok(())
}
