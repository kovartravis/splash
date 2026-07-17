use crossterm::event::{KeyCode, KeyModifiers};
use splash::assert_snapshot;
use splash::leader::{KeyAction, LeaderState};
use splash::pty::HarnessConfig;
use splash::testing::{
    assert_buffer_contains, assert_buffer_matches_regex, format_buffer_grid, TestHarness,
};

fn create_test_config() -> HarnessConfig {
    HarnessConfig {
        command: "test_cmd".to_string(),
        args: vec!["--arg".to_string()],
    }
}

#[test]
fn test_leader_activation_via_ctrl_b() {
    let mut harness = TestHarness::new(80, 8, create_test_config());

    // Initially leader state is Normal
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());

    // Render initial frame and verify leader title does not contain [LEADER ACTIVE]
    let buffer = harness.render_frame();
    let initial_grid = format_buffer_grid(buffer);
    assert!(!initial_grid.contains("[LEADER ACTIVE]"));

    // Press Ctrl+B to activate leader state
    let action = harness.press_ctrl('b');
    assert_eq!(action, KeyAction::None);
    assert_eq!(harness.app.leader_state, LeaderState::LeaderPressed);
    assert!(harness.app.leader_state.is_active());

    // Render frame and assert buffer contains [LEADER ACTIVE]
    let buffer = harness.render_frame();
    assert_buffer_contains(buffer, "[LEADER ACTIVE]");
    assert_buffer_matches_regex(buffer, r"\[LEADER ACTIVE\]");

    // Snapshot assertion on leader active frame (80x8 -> 10 formatted lines)
    let expected_lines = vec![
        "┌────────────────────────────────────────────────────────────────────────────────┐",
        "│┌ Harness: test_cmd (Leader: Ctrl+B | Exit: Ctrl+B q)  [LEADER ACTIVE]─────────┐│",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "│└──────────────────────────────────────────────────────────────────────────────┘│",
        "└────────────────────────────────────────────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &expected_lines);

    // Press Ctrl+B again to deactivate leader state and forward byte 0x02
    let action = harness.press_ctrl('b');
    assert_eq!(action, KeyAction::Forward(vec![0x02]));
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());

    // Render frame and verify [LEADER ACTIVE] is removed
    let buffer = harness.render_frame();
    let deactivated_grid = format_buffer_grid(buffer);
    assert!(!deactivated_grid.contains("[LEADER ACTIVE]"));
}

#[test]
fn test_leader_shortcuts_c_quote_percent_esc() {
    let mut harness = TestHarness::new(80, 6, create_test_config());

    // Shortcut 'c'
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());
    let action = harness.press_char('c');
    assert_eq!(action, KeyAction::None);
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());

    // Shortcut '"'
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());
    let action = harness.press_char('"');
    assert_eq!(action, KeyAction::None);
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());

    // Shortcut '%'
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());
    let action = harness.press_char('%');
    assert_eq!(action, KeyAction::None);
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());

    // Shortcut Esc
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());
    let action = harness.send_key(KeyCode::Esc, KeyModifiers::empty());
    assert_eq!(action, KeyAction::None);
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());

    // Verify buffer frame output after resetting to Normal (80x6 -> 8 formatted lines)
    let buffer = harness.render_frame();
    assert_buffer_matches_regex(buffer, r"Harness:\s+test_cmd");
    assert!(!format_buffer_grid(buffer).contains("[LEADER ACTIVE]"));

    let expected_lines = vec![
        "┌────────────────────────────────────────────────────────────────────────────────┐",
        "│┌ Harness: test_cmd (Leader: Ctrl+B | Exit: Ctrl+B q) ─────────────────────────┐│",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "│└──────────────────────────────────────────────────────────────────────────────┘│",
        "└────────────────────────────────────────────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &expected_lines);
}

#[test]
fn test_leader_shortcut_quit_actions() {
    let mut harness = TestHarness::new(80, 6, create_test_config());

    // Press Ctrl+B then 'q' -> KeyAction::Quit
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());
    let action = harness.press_char('q');
    assert_eq!(action, KeyAction::Quit);
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());

    // Press Ctrl+B then 'Q' -> KeyAction::Quit
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());
    let action = harness.press_char('Q');
    assert_eq!(action, KeyAction::Quit);
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
    assert!(!harness.app.leader_state.is_active());
}

#[test]
fn test_leader_key_interactive_sequence() {
    let mut harness = TestHarness::new(80, 7, create_test_config());

    // 1. Send normal characters (forwarded to PTY)
    let action_a = harness.press_char('a');
    assert_eq!(action_a, KeyAction::Forward(vec![b'a']));
    let action_enter = harness.send_key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(action_enter, KeyAction::Forward(vec![b'\r']));

    // 2. Activate leader mode
    let action_ctrl_b = harness.press_ctrl('b');
    assert_eq!(action_ctrl_b, KeyAction::None);
    assert!(harness.app.leader_state.is_active());

    // Verify snapshot frame displays [LEADER ACTIVE] (80x7 -> 9 formatted lines)
    let leader_active_lines = vec![
        "┌────────────────────────────────────────────────────────────────────────────────┐",
        "│┌ Harness: test_cmd (Leader: Ctrl+B | Exit: Ctrl+B q)  [LEADER ACTIVE]─────────┐│",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "││                                                                              ││",
        "│└──────────────────────────────────────────────────────────────────────────────┘│",
        "└────────────────────────────────────────────────────────────────────────────────┘",
    ];
    assert_snapshot!(&mut harness, &leader_active_lines);

    // 3. Deactivate via Esc
    let action_esc = harness.send_key(KeyCode::Esc, KeyModifiers::empty());
    assert_eq!(action_esc, KeyAction::None);
    assert!(!harness.app.leader_state.is_active());

    // 4. Re-activate leader mode and quit
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());
    let action_q = harness.press_char('q');
    assert_eq!(action_q, KeyAction::Quit);
    assert_eq!(harness.app.leader_state, LeaderState::Normal);
}
