use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::style::{Color, Modifier};
use splash::app::Focus;
use splash::pty::HarnessConfig;
use splash::testing::{assert_buffer_contains, TestHarness};
use splash::tree::FileTree;

#[test]
fn test_file_tree_visuals_icons_indentation_and_highlighting() {
    let temp_dir = std::env::temp_dir().join(format!("splash_test_visuals_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();
    std::fs::File::create(temp_dir.join("README.md")).unwrap();
    std::fs::File::create(temp_dir.join("src/main.rs")).unwrap();

    let file_tree = FileTree::new(&temp_dir).unwrap();
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::with_file_tree(80, 24, config, file_tree);

    // Focus FileTree via Ctrl+B Left
    harness.press_ctrl('b');
    harness.send_key(KeyCode::Left, KeyModifiers::empty());
    assert_eq!(harness.app.focus, Focus::FileTree);

    // Render frame
    let buffer = harness.render_frame().clone();

    // Verify initial collapsed directory rendering and indentation
    assert_buffer_contains(&buffer, " File Tree ");
    assert_buffer_contains(&buffer, "▶ src");
    assert_buffer_contains(&buffer, "  README.md");

    // Check highlighted active selection cell (x=1, y=2) has Yellow foreground & BOLD modifier
    let cell_selected = buffer.get(1, 2);
    assert_eq!(cell_selected.symbol(), "▶");
    assert_eq!(cell_selected.fg, Color::Yellow);
    assert!(cell_selected.modifier.contains(Modifier::BOLD));

    // Expand "src" directory (Right arrow key)
    harness.send_key(KeyCode::Right, KeyModifiers::empty());
    let buffer_expanded = harness.render_frame().clone();

    // Verify expanded directory icon '▼' and 2-space depth indentation for child 'main.rs'
    assert_buffer_contains(&buffer_expanded, "▼ src");
    assert_buffer_contains(&buffer_expanded, "    main.rs");

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_file_tree_keyboard_navigation_interactive_sequence() {
    let temp_dir = std::env::temp_dir().join(format!("splash_test_interactive_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();
    std::fs::File::create(temp_dir.join("a.txt")).unwrap();
    std::fs::File::create(temp_dir.join("b.txt")).unwrap();

    let file_tree = FileTree::new(&temp_dir).unwrap();
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::with_file_tree(80, 24, config, file_tree);

    // Focus FileTree
    harness.app.focus = Focus::FileTree;

    // Navigate down with 'j' key
    harness.press_char('j');
    assert_eq!(harness.app.file_tree.selected_index(), 1);

    // Navigate down with Down arrow
    harness.send_key(KeyCode::Down, KeyModifiers::empty());
    assert_eq!(harness.app.file_tree.selected_index(), 2);

    // Navigate up with 'k' key
    harness.press_char('k');
    assert_eq!(harness.app.file_tree.selected_index(), 1);

    // Navigate up with Up arrow
    harness.send_key(KeyCode::Up, KeyModifiers::empty());
    assert_eq!(harness.app.file_tree.selected_index(), 0);

    // Expand "src" with Enter key
    harness.send_key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(harness.app.file_tree.entries()[0].is_expanded, true);

    // Collapse "src" with Left arrow
    harness.send_key(KeyCode::Left, KeyModifiers::empty());
    assert_eq!(harness.app.file_tree.entries()[0].is_expanded, false);

    // Offscreen snapshot verification
    let snapshot = harness.buffer_snapshot();
    assert!(snapshot.contains("▶ src"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_file_tab_opening_focusing_and_wrapping_visuals() {
    let temp_dir = std::env::temp_dir().join(format!("splash_test_tab_vis_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("src")).unwrap();
    std::fs::write(temp_dir.join("README.md"), "This is a long README file content designed to test wrapping and tab switching in Splash harness.").unwrap();

    let file_tree = FileTree::new(&temp_dir).unwrap();
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::with_file_tree(80, 24, config, file_tree);

    // Focus FileTree
    harness.app.focus = Focus::FileTree;

    // Navigate to "README.md" (index 1: index 0 is "src" folder, index 1 is "README.md")
    harness.press_char('j');
    assert_eq!(harness.app.file_tree.selected_entry().unwrap().name, "README.md");

    // Press Enter to open file tab
    harness.send_key(KeyCode::Enter, KeyModifiers::empty());

    // Focus should now be MainPane, active tab index should be 1
    assert_eq!(harness.app.focus, Focus::MainPane);
    assert_eq!(harness.app.active_tab_index, 1);

    // Render frame and verify tab bar & main pane title & content
    let buffer = harness.render_frame().clone();
    assert_buffer_contains(&buffer, "[2: README.md]");
    assert_buffer_contains(&buffer, "Main Pane (File: README.md)");
    assert_buffer_contains(&buffer, "This is a long README file content");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_file_tab_scrolling_controls_interactive_sequence() {
    let temp_dir = std::env::temp_dir().join(format!("splash_test_tab_scroll_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let content = (1..=50).map(|i| format!("Line {:02}", i)).collect::<Vec<_>>().join("\n");
    std::fs::write(temp_dir.join("long_file.txt"), content).unwrap();

    let file_tree = FileTree::new(&temp_dir).unwrap();
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::with_file_tree(80, 24, config, file_tree);

    // Open file tab
    harness.app.focus = Focus::FileTree;
    harness.send_key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(harness.app.focus, Focus::MainPane);

    // Scroll down 3 lines using Down arrow
    harness.send_key(KeyCode::Down, KeyModifiers::empty());
    harness.send_key(KeyCode::Down, KeyModifiers::empty());
    harness.send_key(KeyCode::Down, KeyModifiers::empty());

    let buffer = harness.render_frame().clone();
    // Top line visible in main pane should now be "Line 04" (0-indexed line offset 3)
    assert_buffer_contains(&buffer, "Line 04");

    // Scroll up 1 line using Up arrow
    harness.send_key(KeyCode::Up, KeyModifiers::empty());
    let buffer_up = harness.render_frame().clone();
    assert_buffer_contains(&buffer_up, "Line 03");

    // Scroll down half-screen using PageDown (terminal height 24 -> inner height 21 -> half-page = 10 lines. 2 + 10 = 12 -> "Line 13")
    harness.send_key(KeyCode::PageDown, KeyModifiers::empty());
    let buffer_pgdn = harness.render_frame().clone();
    assert_buffer_contains(&buffer_pgdn, "Line 13");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_tab_closure_ctrl_b_w_and_empty_workspace_visuals() {
    let temp_dir = std::env::temp_dir().join(format!("splash_test_close_vis_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    std::fs::write(temp_dir.join("note.txt"), "Note content").unwrap();

    let file_tree = FileTree::new(&temp_dir).unwrap();
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::with_file_tree(80, 24, config, file_tree);

    // Initially 1 tab ("bash")
    assert_eq!(harness.app.tabs.len(), 1);

    // Open file tab ("note.txt") -> 2 tabs total
    harness.app.focus = Focus::FileTree;
    harness.send_key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(harness.app.tabs.len(), 2);
    assert_eq!(harness.app.active_tab_index, 1);

    // Close active file tab via Ctrl+B w
    harness.press_ctrl('b');
    harness.press_char('w');
    assert_eq!(harness.app.tabs.len(), 1);
    assert_eq!(harness.app.active_tab_index, 0);

    // Close harness tab via Ctrl+B w -> 0 tabs remaining
    harness.press_ctrl('b');
    harness.press_char('w');
    assert!(harness.app.tabs.is_empty());

    // Render frame and verify Empty Workspace screen
    let buffer = harness.render_frame().clone();
    assert_buffer_contains(&buffer, "Empty Workspace");
    assert_buffer_contains(&buffer, "No tabs are currently open.");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_harness_launcher_ctrl_b_h_interactive_sequence() {
    let temp_dir = std::env::temp_dir().join(format!("splash_test_launch_vis_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let file_tree = FileTree::new(&temp_dir).unwrap();
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::with_file_tree(80, 24, config, file_tree);

    // Open Harness Launcher via Ctrl+B h
    harness.press_ctrl('b');
    harness.press_char('h');
    assert_eq!(harness.app.launcher_input, Some(String::new()));

    // Render launcher frame
    let buffer_prompt = harness.render_frame().clone();
    assert_buffer_contains(&buffer_prompt, "Harness Launcher");

    // Type "codex"
    harness.press_char('c');
    harness.press_char('o');
    harness.press_char('d');
    harness.press_char('e');
    harness.press_char('x');

    let buffer_typed = harness.render_frame().clone();
    assert_buffer_contains(&buffer_typed, "> codex");

    // Press Enter -> spawns new tab [2: codex]
    harness.send_key(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(harness.app.tabs.len(), 2);
    assert_eq!(harness.app.active_tab_index, 1);
    assert!(harness.app.launcher_input.is_none());

    let buffer_spawned = harness.render_frame().clone();
    assert_buffer_contains(&buffer_spawned, "[2: codex]");
    assert_buffer_contains(&buffer_spawned, "Main Pane (Harness: codex)");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
