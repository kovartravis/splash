use splash::pty::HarnessConfig;
use splash::testing::{
    assert_buffer_matches, assert_buffer_matches_regex, TestHarness,
};
use splash::assert_snapshot;

/// 1. Multiline Regex Patterns
#[test]
fn test_multiline_regex_patterns() {
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    // Height 4: Top border, Alpha, Beta, empty, Bottom border
    let mut harness = TestHarness::new(60, 4, config);
    harness.inject_pty_output("Alpha\nBeta");

    let buffer = harness.render_frame();

    // Dot-all mode (?s) allowing . to match newlines across the buffer grid
    assert_buffer_matches_regex(buffer, r"(?s)┌─+┐.*││Alpha\s+││.*││Beta\s+││.*└─+┘");

    // Multiline mode (?m) matching line start and line end anchors on intermediate lines
    assert_buffer_matches_regex(buffer, r"(?m)^││Alpha\s+││$");
    assert_buffer_matches_regex(buffer, r"(?m)^││Beta\s+││$");

    // Explicit newline matching in regex pattern
    assert_buffer_matches_regex(buffer, r"Alpha\s+││\n││Beta");

    // Verify alias assert_buffer_matches works with multiline pattern
    assert_buffer_matches(buffer, r"(?s)Harness: bash.*Alpha.*Beta");
}

/// 2. Escaped Special Characters and Invalid Regex Handling
#[test]
fn test_escaped_special_characters_and_unicode() {
    let config = HarnessConfig {
        command: "test-app".to_string(),
        args: vec![],
    };
    // Width 85 to ensure title "[LEADER ACTIVE]" is not truncated by block border
    let mut harness = TestHarness::new(85, 3, config);
    harness.inject_pty_output("Cost: $100 + $50 (Total: 150) [STATUS: OK] | foo?bar*");

    let buffer = harness.render_frame();

    // Escaped regex special characters: $, +, (, ), [, ], |, ?, *
    assert_buffer_matches_regex(
        buffer,
        r"Cost:\s+\$100\s+\+\s+\$50\s+\(Total:\s+150\)\s+\[STATUS:\s+OK\]\s+\|\s+foo\?bar\*",
    );

    // Unicode box-drawing character escaping and literal matching
    assert_buffer_matches_regex(buffer, r"┌─{10,85}┐");
    assert_buffer_matches_regex(buffer, r"└─{10,85}┘");
    assert_buffer_matches_regex(buffer, r"│┌ Harness:\s+test-app\s+\(Leader:\s+Ctrl\+B\s+\|\s+Exit:\s+Ctrl\+B\s+q\)\s+─+┐│");

    // Verify Leader state regex with escaped brackets
    harness.press_ctrl('b');
    let buffer_leader = harness.render_frame();
    assert_buffer_matches_regex(buffer_leader, r"\[LEADER ACTIVE\]");
}

#[test]
fn test_invalid_regex_pattern_panics() {
    let config = HarnessConfig {
        command: "echo".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(40, 3, config);
    let buffer = harness.render_frame();

    let result = std::panic::catch_unwind(|| {
        assert_buffer_matches_regex(buffer, r"[unclosed character class");
    });

    assert!(result.is_err());
    let panic_msg = downcast_panic_message(result.unwrap_err());
    assert!(
        panic_msg.contains("Invalid regex pattern"),
        "Panic message should mention invalid regex pattern, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("[unclosed character class"),
        "Panic message should display invalid pattern string, got: {}",
        panic_msg
    );
}

/// 3. State Toggling Snapshot Diffs
#[test]
fn test_state_toggling_snapshot_diffs() {
    let config = HarnessConfig {
        command: "zsh".to_string(),
        args: vec![],
    };
    // Height 3 -> 5 lines total (outer top, block top, 1 content row, block bottom, outer bottom)
    let mut harness = TestHarness::new(80, 3, config);

    let top_border = format!("┌{}┐", "─".repeat(80));
    let bottom_border = format!("└{}┘", "─".repeat(80));

    // State 1: Initial State (Leader Inactive)
    let state1_lines = vec![
        top_border.as_str(),
        "│┌ Harness: zsh (Leader: Ctrl+B | Exit: Ctrl+B q) ──────────────────────────────┐│",
        "││                                                                              ││",
        "│└──────────────────────────────────────────────────────────────────────────────┘│",
        bottom_border.as_str(),
    ];
    assert_snapshot!(&mut harness, &state1_lines);

    // State 2: Leader Active (Ctrl+B)
    harness.press_ctrl('b');
    let state2_lines = vec![
        top_border.as_str(),
        "│┌ Harness: zsh (Leader: Ctrl+B | Exit: Ctrl+B q)  [LEADER ACTIVE]──────────────┐│",
        "││                                                                              ││",
        "│└──────────────────────────────────────────────────────────────────────────────┘│",
        bottom_border.as_str(),
    ];
    assert_snapshot!(&mut harness, &state2_lines);

    // State 3: Inject PTY Output while Leader Active
    harness.inject_pty_output("Prompt> hello");
    let state3_lines = vec![
        top_border.as_str(),
        "│┌ Harness: zsh (Leader: Ctrl+B | Exit: Ctrl+B q)  [LEADER ACTIVE]──────────────┐│",
        "││Prompt> hello                                                                 ││",
        "│└──────────────────────────────────────────────────────────────────────────────┘│",
        bottom_border.as_str(),
    ];
    assert_snapshot!(&mut harness, &state3_lines);

    // State 4: Deactivate Leader ('q')
    harness.press_char('q');
    let state4_lines = vec![
        top_border.as_str(),
        "│┌ Harness: zsh (Leader: Ctrl+B | Exit: Ctrl+B q) ──────────────────────────────┐│",
        "││Prompt> hello                                                                 ││",
        "│└──────────────────────────────────────────────────────────────────────────────┘│",
        bottom_border.as_str(),
    ];
    assert_snapshot!(&mut harness, &state4_lines);
}

/// 4. Panic Message Formatting on Assertion Failures
#[test]
fn test_panic_formatting_assert_buffer_matches_regex_failure() {
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(30, 3, config);
    harness.inject_pty_output("Sample Output");
    let buffer = harness.render_frame();

    let result = std::panic::catch_unwind(|| {
        assert_buffer_matches_regex(buffer, r"NON_EXISTENT_PATTERN_\d+");
    });

    assert!(result.is_err());
    let panic_msg = downcast_panic_message(result.unwrap_err());

    assert!(
        panic_msg.contains("Assertion failed: buffer grid does not match regex pattern"),
        "Panic msg missing expected failure header, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("NON_EXISTENT_PATTERN_"),
        "Panic msg missing pattern string, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("Formatted Buffer Grid:\n┌──────────────────────────────┐"),
        "Panic msg missing formatted buffer grid top border, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("││Sample Output               ││"),
        "Panic msg missing buffer content, got: {}",
        panic_msg
    );
}

#[test]
fn test_panic_formatting_assert_snapshot_line_count_mismatch() {
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(20, 3, config);

    // Harness has height 3 -> total 5 lines grid.
    // Supplying 3 lines causes line count mismatch panic.
    let short_expected = vec![
        "┌────────────────────┐",
        "│ Line 1             │",
        "└────────────────────┘",
    ];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_snapshot!(&mut harness, &short_expected);
    }));

    assert!(result.is_err());
    let panic_msg = downcast_panic_message(result.unwrap_err());

    assert!(
        panic_msg.contains("Snapshot line count mismatch: expected 3 lines, got 5 lines."),
        "Panic msg missing line count mismatch description, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("Expected lines:\n┌────────────────────┐\n│ Line 1             │\n└────────────────────┘"),
        "Panic msg missing expected lines block, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("Actual grid:\n┌────────────────────┐"),
        "Panic msg missing actual grid block, got: {}",
        panic_msg
    );
}

#[test]
fn test_panic_formatting_assert_snapshot_line_content_mismatch() {
    let config = HarnessConfig {
        command: "sh".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(60, 3, config);

    let top_border = format!("┌{}┐", "─".repeat(60));
    let bottom_border = format!("└{}┘", "─".repeat(60));

    // Intentional mismatch at line index 2
    let mismatched_expected = vec![
        top_border.as_str(),
        "│┌ Harness: sh (Leader: Ctrl+B | Exit: Ctrl+B q) ───────────┐│",
        "││ INCORRECT EXPECTED LINE                                  ││",
        "│└──────────────────────────────────────────────────────────┘│",
        bottom_border.as_str(),
    ];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_snapshot!(&mut harness, &mismatched_expected);
    }));

    assert!(result.is_err());
    let panic_msg = downcast_panic_message(result.unwrap_err());

    assert!(
        panic_msg.contains("Snapshot line mismatch at line 2:"),
        "Panic msg missing line index, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("Expected: \"││ INCORRECT EXPECTED LINE                                  ││\""),
        "Panic msg missing expected line formatted string, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("Actual:   \"││                                                          ││\""),
        "Panic msg missing actual line formatted string, got: {}",
        panic_msg
    );
    assert!(
        panic_msg.contains("Formatted Buffer Grid:\n┌──────────────────────────────"),
        "Panic msg missing formatted grid output, got: {}",
        panic_msg
    );
}

/// Helper function to extract String payload from Box<dyn Any + Send> panic
fn downcast_panic_message(err: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&'static str>() {
        s.to_string()
    } else {
        "Unknown panic payload type".to_string()
    }
}
