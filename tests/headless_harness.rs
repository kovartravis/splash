use crossterm::event::{KeyCode, KeyModifiers};
use splash::pty::HarnessConfig;
use splash::testing::TestHarness;
use splash::KeyAction;

#[test]
fn test_harness_custom_dimensions_80x24() {
    let config = HarnessConfig {
        command: "test_cmd".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config);
    assert_eq!(harness.app.terminal_size, (80, 24));

    let buffer = harness.render_frame();
    assert_eq!(buffer.area.width, 80);
    assert_eq!(buffer.area.height, 24);
}

#[test]
fn test_harness_custom_dimensions_120x40() {
    let config = HarnessConfig {
        command: "test_cmd".to_string(),
        args: vec!["--flag".to_string()],
    };
    let mut harness = TestHarness::new(120, 40, config);
    assert_eq!(harness.app.terminal_size, (120, 40));

    let buffer = harness.render_frame();
    assert_eq!(buffer.area.width, 120);
    assert_eq!(buffer.area.height, 40);
}

#[test]
fn test_harness_resize() {
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config);

    harness.resize(100, 30);
    assert_eq!(harness.app.terminal_size, (100, 30));

    let buffer = harness.render_frame();
    assert_eq!(buffer.area.width, 100);
    assert_eq!(buffer.area.height, 30);
}

#[test]
fn test_harness_pty_output_injection_and_offscreen_rendering() {
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config);
    
    harness.inject_pty_output("Output line 1\nOutput line 2");
    assert_eq!(harness.app.raw_output, "Output line 1\nOutput line 2");

    let snapshot = harness.buffer_snapshot();
    assert!(!snapshot.is_empty());

    let buffer = harness.render_frame();
    // Check that title block contains command
    let snapshot_str = format!("{:?}", buffer);
    assert!(snapshot_str.contains("Harness: sh"));
}

#[test]
fn test_harness_key_simulation_and_leader_state() {
    let config = HarnessConfig {
        command: "app".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config);

    // Pressing 'a' forwards character
    let action_a = harness.press_char('a');
    assert_eq!(action_a, KeyAction::Forward(vec![b'a']));
    assert!(!harness.app.leader_state.is_active());

    // Pressing Ctrl+B activates leader
    let action_ctrl_b = harness.press_ctrl('b');
    assert_eq!(action_ctrl_b, KeyAction::None);
    assert!(harness.app.leader_state.is_active());

    // Render frame while leader active to check title
    let buffer = harness.render_frame();
    let snapshot_str = format!("{:?}", buffer);
    assert!(snapshot_str.contains("[LEADER ACTIVE]"));

    // Pressing 'q' in leader mode quits
    let action_q = harness.send_key(KeyCode::Char('q'), KeyModifiers::empty());
    assert_eq!(action_q, KeyAction::Quit);
    assert!(!harness.app.leader_state.is_active());
}
