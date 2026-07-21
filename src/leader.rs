use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::MoveDirection;

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
    FocusFileTree,
    FocusMainPane,
    SwitchTab(usize),
    CloseTab,
    OpenLauncher,
    Forward(Vec<u8>),
    MovePaneFocus(MoveDirection),
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
                    KeyCode::Char('w') | KeyCode::Char('W') => KeyAction::CloseTab,
                    KeyCode::Char('h') | KeyCode::Char('H') => KeyAction::OpenLauncher,
                    KeyCode::Char('e') | KeyCode::Char('E') => KeyAction::FocusFileTree,
                    KeyCode::Left => KeyAction::MovePaneFocus(MoveDirection::Left),
                    KeyCode::Right => KeyAction::MovePaneFocus(MoveDirection::Right),
                    KeyCode::Up => KeyAction::MovePaneFocus(MoveDirection::Up),
                    KeyCode::Down => KeyAction::MovePaneFocus(MoveDirection::Down),
                    KeyCode::Char(c @ '1'..='9') => {
                        let idx = (c as usize) - ('1' as usize);
                        KeyAction::SwitchTab(idx)
                    }
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
            KeyCode::Up => b"\x1b[A".to_vec(),
            KeyCode::Down => b"\x1b[B".to_vec(),
            KeyCode::Right => b"\x1b[C".to_vec(),
            KeyCode::Left => b"\x1b[D".to_vec(),
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

    #[test]
    fn test_leader_navigation_chords() {
        let mut leader = LeaderState::default();

        // Ctrl+B e -> FocusFileTree
        leader.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert_eq!(
            leader.handle_key(&KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty())),
            KeyAction::FocusFileTree
        );

        // Ctrl+B Right -> MovePaneFocus(Right)
        leader.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert_eq!(
            leader.handle_key(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty())),
            KeyAction::MovePaneFocus(MoveDirection::Right)
        );

        // Ctrl+B 1..9 -> SwitchTab
        leader.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert_eq!(
            leader.handle_key(&KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty())),
            KeyAction::SwitchTab(0)
        );

        leader.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert_eq!(
            leader.handle_key(&KeyEvent::new(KeyCode::Char('2'), KeyModifiers::empty())),
            KeyAction::SwitchTab(1)
        );
    }

    #[test]
    fn test_leader_ctrl_b_w_close_tab() {
        let mut leader = LeaderState::default();
        leader.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert_eq!(
            leader.handle_key(&KeyEvent::new(KeyCode::Char('w'), KeyModifiers::empty())),
            KeyAction::CloseTab
        );
    }

    #[test]
    fn test_leader_ctrl_b_h_open_launcher() {
        let mut leader = LeaderState::default();
        leader.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert_eq!(
            leader.handle_key(&KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty())),
            KeyAction::OpenLauncher
        );
    }

    #[test]
    fn test_key_event_to_bytes_arrow_keys() {
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Up, KeyModifiers::empty())), b"\x1b[A".to_vec());
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Down, KeyModifiers::empty())), b"\x1b[B".to_vec());
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Right, KeyModifiers::empty())), b"\x1b[C".to_vec());
        assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Left, KeyModifiers::empty())), b"\x1b[D".to_vec());
    }
}
