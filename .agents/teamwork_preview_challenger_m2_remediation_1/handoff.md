# Milestone 2 Remediation Empirical Challenge Report

## 1. Observation

During empirical stress-testing of Milestone 2 Remediation across `format_buffer_grid`, wide-character handling, multi-byte emojis, zero-width characters, and snapshot assertions, the following empirical results were recorded:

### Test Execution Commands & Results
- **Command**: `cargo test --all-targets`
- **Result**: `ok. 78 passed; 0 failed; 0 ignored; 0 measured` across all 9 test suites:
  - `src/lib.rs` (13 unit tests PASSED)
  - `tests/empirical_challenge_m1_2.rs` (7 tests PASSED)
  - `tests/empirical_challenge_m2_1.rs` (12 tests PASSED)
  - `tests/empirical_challenge_m2_2.rs` (7 tests PASSED)
  - `tests/empirical_challenge_m2_remediation.rs` (8 tests PASSED)
  - `tests/empirical_challenge_m2_remediation_2.rs` (13 tests PASSED)
  - `tests/headless_harness.rs` (5 tests PASSED)
  - `tests/snapshot_inspection.rs` (7 tests PASSED)
  - `tests/stress_tests.rs` (13 tests PASSED)

### Detailed Findings & Flaws Discovered

1. **Defect 1 (HIGH SEVERITY): Out-of-Bounds Panic on Non-Zero Origin Buffers (`Rect::new(x > 0, y > 0, w, h)`)**
   - **Code Location**: `src/testing/snapshot.rs:17-18`
   - **Observed Behavior**: `format_buffer_grid` loops `x` from `0..buffer.area.width` and `y` from `0..buffer.area.height`, calling `buffer.get(x, y)`. However, Ratatui's `Buffer::get(x, y)` expects absolute coordinates `(buffer.area.x + x, buffer.area.y + y)`.
   - **Verbatim Error**:
     ```
     thread 'test_nonzero_rect_origin_buffer_panic' panicked at src/testing/snapshot.rs:18:31:
     Trying to access position outside the buffer: x=0, y=0, area=Rect { x: 5, y: 10, width: 10, height: 3 }
     ```
   - **Blast Radius**: Any call to `format_buffer_grid` or snapshot assertions on a sub-buffer or window chunk with non-zero rect origin panics and crashes the process.

2. **Defect 2 (LOW SEVERITY): Border Box Protrusion on Right Boundary Double-Width Symbols**
   - **Code Location**: `src/testing/snapshot.rs:16-27`
   - **Observed Behavior**: When a double-width symbol (display width 2, e.g., `"中"`) is directly set in the last column of a buffer (`x = width - 1`), `sym_w` is 2, advancing `x` past `width`. The resulting row string display width inside `│...│` becomes `width + 1`, pushing the right border `│` out by 1 column beyond the top border (`┌...┐`).
   - **Note**: Ratatui's high-level `set_string` API truncates characters that do not fit within remaining row width, preventing this in standard rendering, but direct cell symbol mutation (`buffer.get_mut(width - 1, y).set_symbol(...)`) exposes the layout mismatch.

3. **Verified Remediation Capabilities**:
   - **Double-Width Follower Skipping**: Follower spaces at `x+1` for double-width CJK ("中文") and emojis ("🦀") are correctly skipped, preserving border box alignment (`│中文    │`).
   - **Complex Emojis & ZWJ**: Flags ("🇺🇸"), skin tone modifiers ("👋🏽"), and ZWJ sequences ("👨‍👩‍👧‍👦", "👩‍💻") format cleanly without buffer corruption.
   - **Zero-Width & Combining Characters**: Characters like `\u{200B}` (Zero Width Space) and `\u{0301}` (Combining Acute Accent) process safely.
   - **Control Characters**: Embedded `\n` in cell symbols splits row strings into multiple lines.
   - **Snapshot Assertions**: `assert_snapshot!`, `assert_buffer_contains`, `assert_buffer_matches`, and `assert_buffer_matches_regex` function correctly.

---

## 2. Logic Chain

1. **Observation**: `buffer.get(x, y)` is called with `x = 0..area.width` and `y = 0..area.height` in `format_buffer_grid` (`src/testing/snapshot.rs:18`).
2. **Deduction**: `Buffer::get` in Ratatui indexing checks `x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height`. When `area.x > 0` or `area.y > 0`, passing `(0, 0)` causes an out-of-bounds assertion panic.
3. **Conclusion**: `format_buffer_grid` must offset coordinates by `buffer.area.x + x` and `buffer.area.y + y` to support offset buffers.

1. **Observation**: `x` advances by `sym_w` (2) when `cell.symbol()` has `unicode_width` 2.
2. **Deduction**: If `x = width - 1`, `x + sym_w` becomes `width + 1`. The while loop terminates after pushing 1 character of width 2 into a slot of remaining width 1.
3. **Conclusion**: Content row string display width becomes `width + 1`, causing 1-column visual protrusion past top/bottom borders.

1. **Observation**: Standard CJK and Emoji rendering with `set_string` populates cell `(0, y)` with the wide symbol and cell `(1, y)` with an empty/follower space.
2. **Deduction**: `format_buffer_grid` advances `x` by `sym_w` (2), skipping cell `(1, y)`.
3. **Conclusion**: Total row length equals buffer width, keeping borders perfectly aligned.

---

## 3. Caveats

- **Scope Limit**: As an Empirical Challenger operating under review-only constraints, code fixes in `src/testing/snapshot.rs` were NOT applied directly; empirical test cases demonstrating and verifying the findings were added to `tests/empirical_challenge_m2_remediation.rs`.
- **Ratatui set_string boundary defense**: High-level calls to `set_string` at `x = width - 1` drop wide characters that exceed bounds, preventing border protrusion under normal usage.

---

## 4. Conclusion

Milestone 2 Remediation is **PARTIALLY VERIFIED WITH DEFECTS**:
- **PASSED**: Wide-character CJK and emoji follower cell skipping, ZWJ sequences, complex multi-byte emojis, zero-width characters, and snapshot macros.
- **DEFECT (HIGH SEVERITY)**: `format_buffer_grid` panics on non-zero origin `Buffer`s due to missing `buffer.area.x` / `buffer.area.y` offsets.
- **DEFECT (LOW SEVERITY)**: Direct cell symbol mutation with wide characters at `x = width - 1` causes 1-column border box protrusion.

---

## 5. Verification Method

To independently verify all findings and test suites:

```bash
cargo test --all-targets
```

To run the dedicated empirical challenge test suite:
```bash
cargo test --test empirical_challenge_m2_remediation -- --nocapture
```

Specific test targets:
- `test_nonzero_rect_origin_buffer_panic`: Verifies out-of-bounds panic on offset Rect buffers.
- `test_manual_cell_symbol_wide_char_at_boundary`: Verifies border protrusion at right boundary.
- `test_cjk_and_emoji_follower_cell_skipping`: Verifies wide-character follower skipping alignment.
- `test_complex_emoji_zwj_and_skin_tone_alignment`: Verifies complex multi-byte emoji ZWJ handling.
- `test_snapshot_assertions_coverage`: Verifies snapshot macros and regex assertions.
