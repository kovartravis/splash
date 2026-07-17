use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, PartialEq, Clone, Default)]
pub enum LeaderState {
    #[default]
    Normal,
    LeaderPressed,
}

#[derive(Debug, PartialEq, Clone)]
pub enum KeyAction {
    None,
    Quit,
    Forward(Vec<u8>),
}

impl LeaderState {
    pub fn is_active(&self) -> bool {
        matches!(self, LeaderState::LeaderPressed)
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> KeyAction {
        match self {
            LeaderState::Normal => {
                if key.code == KeyCode::Char('b') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    *self = LeaderState::LeaderPressed;
                    KeyAction::None
                } else {
                    let bytes = key_event_to_bytes(key);
                    if bytes.is_empty() {
                        KeyAction::None
                    } else {
                        KeyAction::Forward(bytes)
                    }
                }
            }
            LeaderState::LeaderPressed => {
                *self = LeaderState::Normal;
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => KeyAction::Quit,
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        KeyAction::Forward(vec![0x02])
                    }
                    _ => KeyAction::None,
                }
            }
        }
    }
}

pub fn key_event_to_bytes(key: &KeyEvent) -> Vec<u8> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char(c) => {
                let lower = c.to_ascii_lowercase();
                if lower.is_ascii_lowercase() {
                    vec![(lower as u8) - b'a' + 1]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    } else {
        match key.code {
            KeyCode::Char(c) => {
                let mut buf = [0; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            }
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Tab => vec![b'\t'],
            KeyCode::Esc => vec![0x1b],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leader_state_machine() {
        let mut leader = LeaderState::default();

        // Normal key -> Forward to PTY
        let key_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        assert_eq!(leader.handle_key(&key_a), KeyAction::Forward(vec![b'a']));

        // Ctrl+B -> Enter LeaderPressed state
        let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        assert_eq!(leader.handle_key(&key_ctrl_b), KeyAction::None);
        assert!(leader.is_active());

        // In LeaderPressed state: 'q' -> Quit
        let key_q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        assert_eq!(leader.handle_key(&key_q), KeyAction::Quit);
    }
}
