use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splash::pty::HarnessConfig;
use splash::testing::TestHarness;
use splash::{key_event_to_bytes, KeyAction, LeaderState};

#[test]
fn test_leader_state_ctrl_b_followed_by_non_q_keys() {
    let mut harness = TestHarness::new(
        80,
        24,
        HarnessConfig {
            command: "test".to_string(),
            args: vec![],
        },
    );

    // 1. Initial state check
    assert!(!harness.app.leader_state.is_active());

    // 2. Enter leader state with Ctrl+B
    let act = harness.press_ctrl('b');
    assert_eq!(act, KeyAction::None);
    assert!(harness.app.leader_state.is_active());

    // 3. Press non-q key ('x'): state returns to Normal, action is None (key swallowed!)
    let act_x = harness.press_char('x');
    assert_eq!(act_x, KeyAction::None);
    assert!(!harness.app.leader_state.is_active());

    // 4. Enter leader state again with Ctrl+B
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());

    // 5. Press plain 'b' (not Ctrl+B): swallowed!
    let act_b = harness.press_char('b');
    assert_eq!(act_b, KeyAction::None);
    assert!(!harness.app.leader_state.is_active());

    // 6. Enter leader state again with Ctrl+B
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());

    // 7. Press Ctrl+B: returns Forward([0x02]), resets state to Normal
    let act_ctrl_b = harness.press_ctrl('b');
    assert_eq!(act_ctrl_b, KeyAction::Forward(vec![0x02]));
    assert!(!harness.app.leader_state.is_active());

    // 8. Enter leader state again with Ctrl+B
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());

    // 9. Press Esc key in leader mode: swallowed, returns Normal
    let act_esc = harness.send_key(KeyCode::Esc, KeyModifiers::empty());
    assert_eq!(act_esc, KeyAction::None);
    assert!(!harness.app.leader_state.is_active());
}

#[test]
fn test_leader_state_direct_unit_tests() {
    let mut state = LeaderState::default();
    assert!(!state.is_active());

    let key_ctrl_b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
    assert_eq!(state.handle_key(&key_ctrl_b), KeyAction::None);
    assert!(state.is_active());

    // In LeaderPressed state, pressing non-q ('a') returns None and resets state
    let key_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
    assert_eq!(state.handle_key(&key_a), KeyAction::None);
    assert!(!state.is_active());
}

#[test]
fn test_leader_state_capital_q_and_ctrl_q() {
    let mut harness = TestHarness::new(
        80,
        24,
        HarnessConfig {
            command: "test".to_string(),
            args: vec![],
        },
    );

    // Capital Q
    harness.press_ctrl('b');
    let act_big_q = harness.send_key(KeyCode::Char('Q'), KeyModifiers::SHIFT);
    assert_eq!(act_big_q, KeyAction::Quit);
    assert!(!harness.app.leader_state.is_active());

    // Ctrl+Q in leader mode
    harness.press_ctrl('b');
    let act_ctrl_q = harness.send_key(KeyCode::Char('q'), KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_q, KeyAction::Quit);
    assert!(!harness.app.leader_state.is_active());
}

#[test]
fn test_multi_byte_unicode_input() {
    let mut harness = TestHarness::new(
        80,
        24,
        HarnessConfig {
            command: "test".to_string(),
            args: vec![],
        },
    );

    // 2-byte UTF-8: 'é' (U+00E9 -> 0xC3, 0xA9)
    let act_e = harness.press_char('é');
    assert_eq!(act_e, KeyAction::Forward(vec![0xC3, 0xA9]));

    // 3-byte UTF-8: '€' (U+20AC -> 0xE2, 0x82, 0xAC)
    let act_euro = harness.press_char('€');
    assert_eq!(act_euro, KeyAction::Forward(vec![0xE2, 0x82, 0xAC]));

    // 4-byte UTF-8: '🦀' (U+1F980 -> 0xF0, 0x9F, 0xA6, 0x80)
    let act_crab = harness.press_char('🦀');
    assert_eq!(act_crab, KeyAction::Forward(vec![0xF0, 0x9F, 0xA6, 0x80]));

    // Direct key_event_to_bytes test for multi-byte Unicode
    let key_crab = KeyEvent::new(KeyCode::Char('🦀'), KeyModifiers::empty());
    assert_eq!(key_event_to_bytes(&key_crab), vec![0xF0, 0x9F, 0xA6, 0x80]);

    // Multi-byte char with CONTROL modifier (e.g. Ctrl+é or Ctrl+🦀)
    let act_ctrl_e = harness.send_key(KeyCode::Char('é'), KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_e, KeyAction::None);

    // Multi-byte char in LeaderPressed state
    harness.press_ctrl('b');
    let act_leader_crab = harness.press_char('🦀');
    assert_eq!(act_leader_crab, KeyAction::None);
    assert!(!harness.app.leader_state.is_active());
}

#[test]
fn test_ctrl_chords() {
    let mut harness = TestHarness::new(
        80,
        24,
        HarnessConfig {
            command: "test".to_string(),
            args: vec![],
        },
    );

    // Standard ASCII letters with Ctrl
    assert_eq!(harness.press_ctrl('a'), KeyAction::Forward(vec![0x01]));
    assert_eq!(harness.press_ctrl('c'), KeyAction::Forward(vec![0x03]));
    assert_eq!(harness.press_ctrl('z'), KeyAction::Forward(vec![0x1A]));

    // Uppercase vs lowercase with Ctrl
    let act_ctrl_big_a = harness.send_key(KeyCode::Char('A'), KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_big_a, KeyAction::Forward(vec![0x01]));

    // Non-alphabetic Ctrl chords (FAILURES / BUGS IN key_event_to_bytes!)
    // Ctrl+[ is traditional Escape (0x1B) in terminals
    let act_ctrl_bracket_left = harness.send_key(KeyCode::Char('['), KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_bracket_left, KeyAction::None);
    assert_eq!(key_event_to_bytes(&KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL)), Vec::<u8>::new());

    // Ctrl+] (0x1D)
    let act_ctrl_bracket_right = harness.send_key(KeyCode::Char(']'), KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_bracket_right, KeyAction::None);

    // Ctrl+\ (0x1C)
    let act_ctrl_backslash = harness.send_key(KeyCode::Char('\\'), KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_backslash, KeyAction::None);

    // Ctrl+Enter
    let act_ctrl_enter = harness.send_key(KeyCode::Enter, KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_enter, KeyAction::None);

    // Ctrl+Backspace
    let act_ctrl_backspace = harness.send_key(KeyCode::Backspace, KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_backspace, KeyAction::None);

    // Ctrl+Tab
    let act_ctrl_tab = harness.send_key(KeyCode::Tab, KeyModifiers::CONTROL);
    assert_eq!(act_ctrl_tab, KeyAction::None);
}

#[test]
fn test_arrow_keys_and_escape_sequences() {
    let mut harness = TestHarness::new(
        80,
        24,
        HarnessConfig {
            command: "test".to_string(),
            args: vec![],
        },
    );

    // Arrow keys are mapped to their standard ANSI escape sequences
    assert_eq!(
        harness.send_key(KeyCode::Up, KeyModifiers::empty()),
        KeyAction::Forward(b"\x1b[A".to_vec())
    );
    assert_eq!(
        key_event_to_bytes(&KeyEvent::new(KeyCode::Up, KeyModifiers::empty())),
        b"\x1b[A".to_vec()
    );
    assert_eq!(
        harness.send_key(KeyCode::Down, KeyModifiers::empty()),
        KeyAction::Forward(b"\x1b[B".to_vec())
    );
    assert_eq!(
        harness.send_key(KeyCode::Left, KeyModifiers::empty()),
        KeyAction::Forward(b"\x1b[D".to_vec())
    );
    assert_eq!(
        harness.send_key(KeyCode::Right, KeyModifiers::empty()),
        KeyAction::Forward(b"\x1b[C".to_vec())
    );

    // Home, End, PageUp, PageDown, Delete, Insert
    assert_eq!(
        harness.send_key(KeyCode::Home, KeyModifiers::empty()),
        KeyAction::None
    );
    assert_eq!(
        harness.send_key(KeyCode::End, KeyModifiers::empty()),
        KeyAction::None
    );
    assert_eq!(
        harness.send_key(KeyCode::PageUp, KeyModifiers::empty()),
        KeyAction::None
    );
    assert_eq!(
        harness.send_key(KeyCode::PageDown, KeyModifiers::empty()),
        KeyAction::None
    );
    assert_eq!(
        harness.send_key(KeyCode::Delete, KeyModifiers::empty()),
        KeyAction::None
    );
    assert_eq!(
        harness.send_key(KeyCode::Insert, KeyModifiers::empty()),
        KeyAction::None
    );

    // Function keys F1-F12
    assert_eq!(
        harness.send_key(KeyCode::F(1), KeyModifiers::empty()),
        KeyAction::None
    );
    assert_eq!(
        harness.send_key(KeyCode::F(12), KeyModifiers::empty()),
        KeyAction::None
    );

    // Esc key (standalone) -> 0x1B
    assert_eq!(
        harness.send_key(KeyCode::Esc, KeyModifiers::empty()),
        KeyAction::Forward(vec![0x1B])
    );
}

#[test]
fn test_testharness_rendering_state_transitions() {
    let mut harness = TestHarness::new(
        80,
        24,
        HarnessConfig {
            command: "bash".to_string(),
            args: vec![],
        },
    );

    // Initial state: frame does not contain [LEADER ACTIVE]
    let snapshot1 = harness.buffer_snapshot();
    assert!(snapshot1.contains("Harness: bash"));
    assert!(!snapshot1.contains("[LEADER ACTIVE]"));

    // Activate leader state
    harness.press_ctrl('b');
    let snapshot2 = harness.buffer_snapshot();
    assert!(snapshot2.contains("[LEADER ACTIVE]"));

    // Inject PTY output while leader active
    harness.inject_pty_output("user@splash:~$ ");
    let snapshot3 = harness.buffer_snapshot();
    assert!(snapshot3.contains("user@splash:~$ "));
    assert!(snapshot3.contains("[LEADER ACTIVE]"));

    // Exit leader state via non-q key ('a')
    harness.press_char('a');
    let snapshot4 = harness.buffer_snapshot();
    assert!(!snapshot4.contains("[LEADER ACTIVE]"));
}
