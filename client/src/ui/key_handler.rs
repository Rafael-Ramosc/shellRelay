use crossterm::event::{self, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    module_bindings::{
        DbConnection, send_message as SendMessageReducerExt, set_name as SetNameReducerExt,
    },
    state::{SharedState, update_state},
    ui::ui_state::InputMode,
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

    match key.code {
        KeyCode::Char('q') => {
            update_state(state, |s| s.ui.should_quit = true);
        }
        KeyCode::Tab => {
            update_state(state, |s| s.ui.input_mode.toggle());
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
            let (mode, text) = {
                let mut guard = state.lock().expect("state poisoned");
                let text = guard.ui.input.trim().to_string();
                guard.ui.input.clear();
                (guard.ui.input_mode, text)
            };

            if text.is_empty() {
                return Ok(());
            }

            let reducer_res = match mode {
                InputMode::Message => conn.reducers.send_message(text),
                InputMode::Name => conn.reducers.set_name(text),
            };

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
