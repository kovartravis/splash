use crossterm::event::{KeyCode, KeyModifiers};
use splash::leader::key_event_to_bytes;
use splash::pty::HarnessConfig;
use splash::testing::TestHarness;
use splash::KeyAction;
use std::time::Instant;

fn create_harness(width: u16, height: u16) -> TestHarness {
    let config = HarnessConfig {
        command: "test_stress".to_string(),
        args: vec![],
    };
    TestHarness::new(width, height, config)
}

#[test]
fn test_rapid_resizes_and_extreme_dimensions() {
    let mut harness = create_harness(80, 24);

    let extreme_sizes = vec![
        (1, 1),
        (200, 100),
        (80, 24),
        (1, 0),
        (0, 1),
        (0, 0),
        (255, 255),
        (256, 256),
        (500, 500),
        (80, 24),
    ];

    for &(w, h) in &extreme_sizes {
        harness.resize(w, h);
        assert_eq!(harness.app.terminal_size, (w, h));
        let buffer = harness.render_frame();
        // Backend clamps to u8::MAX (255) max dimensions
        let expected_w = w.min(255);
        let expected_h = h.min(255);
        assert_eq!(buffer.area.width, expected_w, "Width mismatch for size ({}, {})", w, h);
        assert_eq!(buffer.area.height, expected_h, "Height mismatch for size ({}, {})", w, h);
    }

    let start = Instant::now();
    for i in 0..1000 {
        let w = (i % 200 + 1) as u16;
        let h = (i % 100 + 1) as u16;
        harness.resize(w, h);
        harness.render_frame();
    }
    let elapsed = start.elapsed();
    println!("1000 rapid resizes took {:?}", elapsed);
}

#[test]
fn test_large_pty_output_chunks_10000_lines() {
    let mut harness = create_harness(80, 24);

    let mut chunk = String::with_capacity(10_000 * 20);
    for i in 0..10_000 {
        chunk.push_str(&format!("Line number {}\n", i));
    }

    let start_push = Instant::now();
    harness.inject_pty_output(&chunk);
    let push_duration = start_push.elapsed();

    assert_eq!(harness.app.raw_output.lines().count(), 10_000);

    let start_render = Instant::now();
    let buffer = harness.render_frame();
    let render_duration = start_render.elapsed();

    println!(
        "10,000 lines push: {:?}, render: {:?}",
        push_duration, render_duration
    );
    assert_eq!(buffer.area.width, 80);
    assert_eq!(buffer.area.height, 24);
}

#[test]
fn test_large_pty_output_chunks_100000_lines() {
    let mut harness = create_harness(80, 24);

    let mut chunk = String::with_capacity(100_000 * 20);
    for i in 0..100_000 {
        chunk.push_str(&format!("Log output row {}\n", i));
    }

    let start_push = Instant::now();
    harness.inject_pty_output(&chunk);
    let push_duration = start_push.elapsed();

    let start_render = Instant::now();
    let buffer = harness.render_frame();
    let render_duration = start_render.elapsed();

    println!(
        "100,000 lines push: {:?}, render: {:?}",
        push_duration, render_duration
    );
    assert_eq!(buffer.area.width, 80);
    assert_eq!(buffer.area.height, 24);
}

#[test]
fn test_raw_output_performance_degradation_benchmark() {
    let mut harness = create_harness(80, 24);

    for multiplier in 1..=5 {
        let line_count = multiplier * 50_000;
        let mut chunk = String::with_capacity(50_000 * 20);
        for i in 0..50_000 {
            chunk.push_str(&format!("Line {}\n", i));
        }
        harness.inject_pty_output(&chunk);

        let start = Instant::now();
        harness.render_frame();
        let render_dur = start.elapsed();

        println!(
            "Render time with {} lines (raw_output len: {} bytes): {:?}",
            line_count,
            harness.app.raw_output.len(),
            render_dur
        );
    }
}

#[test]
fn test_large_single_line_output() {
    let mut harness = create_harness(80, 24);

    let long_line = "A".repeat(100_000);
    harness.inject_pty_output(&long_line);

    let start_render = Instant::now();
    let buffer = harness.render_frame();
    let render_duration = start_render.elapsed();

    println!("Single 100,000 char line render: {:?}", render_duration);
    assert_eq!(buffer.area.width, 80);
}

#[test]
fn test_ansi_escape_sequences_and_control_chars() {
    let mut harness = create_harness(80, 24);

    let ansi_text = "\x1b[31mRed Text\x1b[0m\n\x1b[1;32mGreen Bold\x1b[0m\r\n\x07\x08\x00Special Chars";
    harness.inject_pty_output(ansi_text);

    let buffer = harness.render_frame();
    assert_eq!(buffer.area.width, 80);
}

#[test]
fn test_rapid_key_sequences() {
    let mut harness = create_harness(80, 24);

    let start = Instant::now();
    for i in 0..10_000 {
        let ch = (b'a' + (i % 26) as u8) as char;
        let action = harness.press_char(ch);
        assert_eq!(action, KeyAction::Forward(vec![ch as u8]));
    }
    let elapsed = start.elapsed();
    println!("10,000 rapid keypresses took {:?}", elapsed);
}

#[test]
fn test_rapid_leader_toggling() {
    let mut harness = create_harness(80, 24);

    for _ in 0..1000 {
        let action1 = harness.press_ctrl('b');
        assert_eq!(action1, KeyAction::None);
        assert!(harness.app.leader_state.is_active());

        let action2 = harness.press_ctrl('b');
        assert_eq!(action2, KeyAction::Forward(vec![0x02]));
        assert!(!harness.app.leader_state.is_active());
    }
}

#[test]
fn test_rapid_leader_quit_sequence() {
    let mut harness = create_harness(80, 24);

    for _ in 0..500 {
        let action1 = harness.press_ctrl('b');
        assert_eq!(action1, KeyAction::None);
        assert!(harness.app.leader_state.is_active());

        let action2 = harness.send_key(KeyCode::Char('q'), KeyModifiers::empty());
        assert_eq!(action2, KeyAction::Quit);
        assert!(!harness.app.leader_state.is_active());
    }
}

#[test]
fn test_interleaved_heavy_workload() {
    let mut harness = create_harness(80, 24);

    let start = Instant::now();
    for i in 0..500 {
        harness.inject_pty_output(&format!("Batch output chunk {}\n", i));
        harness.press_char('x');
        harness.press_ctrl('b');
        harness.press_ctrl('b');
        let w = (80 + (i % 40)) as u16;
        let h = (24 + (i % 20)) as u16;
        harness.resize(w, h);
        harness.render_frame();
    }
    let elapsed = start.elapsed();
    println!("500 heavy interleaved cycles took {:?}", elapsed);
}

#[test]
fn test_key_event_to_bytes_unmapped_keys() {
    // Arrow keys
    let up = key_event_to_bytes(&crossterm::event::KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    let down = key_event_to_bytes(&crossterm::event::KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    let left = key_event_to_bytes(&crossterm::event::KeyEvent::new(KeyCode::Left, KeyModifiers::empty()));
    let right = key_event_to_bytes(&crossterm::event::KeyEvent::new(KeyCode::Right, KeyModifiers::empty()));

    println!("Arrow Up bytes: {:?}", up);
    println!("Arrow Down bytes: {:?}", down);
    println!("Arrow Left bytes: {:?}", left);
    println!("Arrow Right bytes: {:?}", right);

    // Verify Arrow keys return empty bytes (BUG: arrow keys unmapped)
    assert!(up.is_empty(), "Arrow Up should be unmapped currently");
    assert!(down.is_empty(), "Arrow Down should be unmapped currently");

    // Function keys
    let f1 = key_event_to_bytes(&crossterm::event::KeyEvent::new(KeyCode::F(1), KeyModifiers::empty()));
    assert!(f1.is_empty(), "F1 should be unmapped currently");

    // Ctrl+[ (ESC in terminals)
    let ctrl_bracket = key_event_to_bytes(&crossterm::event::KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL));
    assert!(ctrl_bracket.is_empty(), "Ctrl+[ should be unmapped currently");
}

#[test]
fn test_leader_state_swallows_unrecognized_keys() {
    let mut harness = create_harness(80, 24);

    // Press Ctrl+B to activate leader mode
    harness.press_ctrl('b');
    assert!(harness.app.leader_state.is_active());

    // Press 'x' (an unhandled key in leader mode)
    let action = harness.press_char('x');

    // State returns to normal, but 'x' was swallowed (KeyAction::None) instead of KeyAction::Forward(vec![b'x'])
    assert_eq!(action, KeyAction::None, "Unrecognized key in leader mode was swallowed");
    assert!(!harness.app.leader_state.is_active(), "Leader state should reset to normal");
}

#[test]
fn test_utf8_split_boundary_in_pty_buffer() {
    // Simulate multi-byte UTF-8 char (e.g. 🦀 4 bytes: [240, 159, 166, 128])
    // split across a buffer read boundary (first 2 bytes in chunk 1, last 2 in chunk 2)
    let part1 = &[240u8, 159u8];
    let part2 = &[166u8, 128u8];

    let text1 = String::from_utf8_lossy(part1).to_string();
    let text2 = String::from_utf8_lossy(part2).to_string();

    let combined = format!("{}{}", text1, text2);
    println!("UTF-8 split boundary string result: {:?}", combined);

    // from_utf8_lossy produces replacement chars U+FFFD () when split across buffer boundaries
    assert_ne!(combined, "🦀", "UTF-8 multi-byte char split across read buffer boundaries becomes corrupted");
}
