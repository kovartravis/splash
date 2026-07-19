# Handoff Report: Milestone 2 Remediation Plan

## 1. Observation

Direct examination of the Forensic Auditor report (`.agents/teamwork_preview_auditor_m2/handoff.md`) and codebase files (`src/testing/snapshot.rs`, `tests/empirical_challenge_m2_1.rs`, `tests/empirical_challenge_m2_2.rs`, `Cargo.toml`) revealed the following exact observations:

### Observation A: Syntax Error & Unused Imports in `tests/empirical_challenge_m2_2.rs`
1. Line 70 declares: `let harness = TestHarness::new(40, 3, config);` without `mut`.
2. Line 72 attempts `let buffer = harness.render_frame();`, which requires `&mut self`, triggering compiler error:
   ```text
   error[E0596]: cannot borrow `harness` as mutable, as it is not declared as mutable
     --> tests/empirical_challenge_m2_2.rs:72:18
      |
   72 |     let buffer = harness.render_frame();
      |                  ^^^^^^^ cannot borrow as mutable
   ```
3. Lines 3–4 import `assert_buffer_contains` and `format_buffer_grid`, which are never invoked in `empirical_challenge_m2_2.rs`, producing Clippy errors under `-D warnings`:
   ```text
   error: unused imports: `assert_buffer_contains` and `format_buffer_grid`
    --> tests/empirical_challenge_m2_2.rs:3:5
   ```

### Observation B: Naive Buffer Grid Formatting & Wide Character Follower Cell Handling
1. In `src/testing/snapshot.rs` (lines 14–21), `format_buffer_grid` iterates through columns `x = 0..buffer.area.width` and unconditionally appends `cell.symbol()` to `row_str`:
   ```rust
   for y in 0..buffer.area.height {
       let mut row_str = String::with_capacity(width);
       for x in 0..buffer.area.width {
           let cell = buffer.get(x, y);
           row_str.push_str(cell.symbol());
       }
       lines.push(format!("│{}│", row_str));
   }
   ```
2. When multi-column wide characters (e.g. CJK `"中文"` or Emoji `"🦀"`) are set in a Ratatui `Buffer`, Ratatui places the character symbol in cell `(x, y)` and places a follower space `" "` in cell `(x+1, y)`.
3. Because `format_buffer_grid` appends both cell `(x, y)` (`"中"`, display width 2) and cell `(x+1, y)` (`" "`, display width 1), the formatted row string visual display width exceeds `buffer.area.width`.
4. In `tests/empirical_challenge_m2_1.rs`:
   - Line 64 currently asserts: `assert_eq!(lines[1], "│中 文   │");` (visually 8 columns wide inside border box of 6 columns).
   - Line 77 currently asserts: `assert_eq!(lines[1], "│🦀     │");` (visually 7 columns wide inside border box of 6 columns).
   - Line 92 currently asserts: `assert_eq!(lines[1], "│中  │");` (visually 4 columns wide inside border box of 3 columns).

---

## 2. Logic Chain

1. **Observation**: `tests/empirical_challenge_m2_2.rs:72` calls `harness.render_frame()` on an immutable binding.
   - **Reasoning**: `TestHarness::render_frame(&mut self)` requires a mutable reference to `self`.
   - **Step**: Modifying line 70 to `let mut harness` resolves `error[E0596]`.
2. **Observation**: `tests/empirical_challenge_m2_2.rs:3-4` imports unused symbols `assert_buffer_contains` and `format_buffer_grid`.
   - **Reasoning**: Unused imports trigger compiler warnings and violate `-D warnings` in Clippy.
   - **Step**: Removing unused symbols from the import declaration cleans up Clippy checks.
3. **Observation**: `format_buffer_grid` in `src/testing/snapshot.rs` iterates `0..width` step by 1, rendering both wide characters and their follower spaces.
   - **Reasoning**: Wide characters occupy 2 display columns in terminal output. Outputting both the wide character AND its follower cell space yields 3 display columns for a 2-column wide character, breaking visual alignment between content rows (`│...│`) and outer borders (`┌...┐`, `└...┘`).
   - **Step**: `format_buffer_grid` must advance `x` by `unicode_width::UnicodeWidthStr::width(sym)` when `sym` display width > 1. This skips follower cells during row string construction, keeping total row display width equal to `buffer.area.width`.
4. **Observation**: Tests in `tests/empirical_challenge_m2_1.rs` currently assert protruding wide character lines (`"│中 文   │"`, `"│🦀     │"`, `"│中  │"`).
   - **Reasoning**: Once `format_buffer_grid` skips follower cells, double-width character rows format cleanly as `"│中文  │"`, `"│🦀    │"`, and `"│中 │"`.
   - **Step**: `tests/empirical_challenge_m2_1.rs` assertions must be updated to reflect properly aligned border boxes.

---

## 3. Caveats

- **Read-Only Constraint**: As Explorer, no changes have been made to `src/` or `tests/`. All remediation steps are documented for Worker execution.
- **Dependency Addition**: Using `unicode_width::UnicodeWidthStr` requires adding `unicode-width = "0.1"` to `Cargo.toml` dependencies (or referencing it via `ratatui`'s transitive dependency). Adding `unicode-width = "0.1"` directly to `Cargo.toml` is recommended for explicit dependency management.
- **Zero-Width Character Handling**: Zero-width unicode characters (e.g. combining accents where `width == 0`) must advance `x` by at least 1 to prevent infinite loops in `while x < buffer.area.width`.

---

## 4. Conclusion & Remediation Action Plan

To fully remediate Milestone 2 and restore 100% integrity across builds, tests, and Clippy:

### Action 1: Add Dependency in `Cargo.toml`
Add `unicode-width = "0.1"` under `[dependencies]` in `Cargo.toml`.

### Action 2: Update `src/testing/snapshot.rs`
Update `format_buffer_grid` to use width-aware cell skipping:
```rust
pub fn format_buffer_grid(buffer: &Buffer) -> String {
    let width = buffer.area.width as usize;
    let height = buffer.area.height as usize;

    let mut lines = Vec::with_capacity(height + 2);
    lines.push(format!("┌{}┐", "─".repeat(width)));

    for y in 0..buffer.area.height {
        let mut row_str = String::with_capacity(width);
        let mut x = 0;
        while x < buffer.area.width {
            let cell = buffer.get(x, y);
            let sym = cell.symbol();
            row_str.push_str(sym);
            let sym_w = unicode_width::UnicodeWidthStr::width(sym);
            if sym_w > 0 {
                x += sym_w as u16;
            } else {
                x += 1;
            }
        }
        lines.push(format!("│{}│", row_str));
    }

    lines.push(format!("└{}┘", "─".repeat(width)));
    lines.join("\n")
}
```

### Action 3: Remediate `tests/empirical_challenge_m2_1.rs`
Update line assertions for double-width characters in `tests/empirical_challenge_m2_1.rs`:
- Line 64: Change `assert_eq!(lines[1], "│中 文   │");` -> `assert_eq!(lines[1], "│中文  │");`.
- Line 77: Change `assert_eq!(lines[1], "│🦀     │");` -> `assert_eq!(lines[1], "│🦀    │");`.
- Line 92: Change `assert_eq!(lines[1], "│中  │");` -> `assert_eq!(lines[1], "│中 │");`.

### Action 4: Remediate `tests/empirical_challenge_m2_2.rs`
- Remove unused imports `assert_buffer_contains` and `format_buffer_grid` from lines 3-4.
- Change line 70 `let harness` to `let mut harness`.

---

## 5. Verification Method

Once Worker executes the remediation plan, verify complete resolution with:

1. **Test Suite Verification**:
   ```bash
   cargo test --all-targets
   ```
   *Expected Result*: 100% test pass across all unit tests, integration tests, and empirical challenge tests.

2. **Clippy Verification**:
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```
   *Expected Result*: Clean build with 0 warnings or errors.
