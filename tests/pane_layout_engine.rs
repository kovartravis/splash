use splash::app::{Focus, SplitDirection, SplashApp, Tab, PaneContent, HarnessTab};
use splash::pty::HarnessConfig;
use splash::testing::TestHarness;

#[test]
fn test_pane_layout_engine() {
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    let mut harness = TestHarness::new(80, 24, config.clone());

    // 1. Initial state: One tab, one pane
    assert_eq!(harness.app.tabs.len(), 1);
    let tab = &harness.app.tabs[0];
    assert_eq!(tab.active_pane_id, 0);

    // 2. Programmatically split panes
    let new_pane_content = PaneContent::Harness(HarnessTab::new("echo"));
    harness.app.split_active_pane(SplitDirection::Vertical, new_pane_content);

    // 4. Focus shifts to the new pane
    let tab = &harness.app.tabs[0];
    assert_eq!(tab.active_pane_id, 1);

    // 5. Closing the last pane in a Tab closes the Tab
    harness.app.close_active_pane();
    assert_eq!(harness.app.tabs.len(), 1);
    assert_eq!(harness.app.tabs[0].active_pane_id, 0);

    harness.app.close_active_pane();
    assert_eq!(harness.app.tabs.len(), 0);
}

#[test]
fn test_pane_layout_engine_space_distribution() {
    let config = HarnessConfig {
        command: "bash".to_string(),
        args: vec![],
    };
    // Large terminal to see splits clearly
    let mut harness = TestHarness::new(100, 30, config.clone());
    harness.render_frame();
    
    // Split vertically
    let new_pane_content = PaneContent::Harness(HarnessTab::new("echo"));
    harness.app.split_active_pane(SplitDirection::Vertical, new_pane_content);
    harness.render_frame();

    let tab = &harness.app.tabs[0];
    let panes = tab.panes();
    assert_eq!(panes.len(), 2);
    
    if let PaneContent::Harness(h1) = &panes[0].content {
        if let PaneContent::Harness(h2) = &panes[1].content {
            let (r1, c1) = h1.last_size.unwrap();
            let (r2, c2) = h2.last_size.unwrap();
            
            // Vertical split implies Ratatui's Direction::Vertical, which splits the height
            assert_eq!(c1, c2); // Widths should be exactly equal
            // Heights should differ by at most 1 due to integer division
            assert!((r1 as i32 - r2 as i32).abs() <= 1, "r1: {}, r2: {}", r1, r2);
        } else {
            panic!("Expected harness tab");
        }
    } else {
        panic!("Expected harness tab");
    }
}
