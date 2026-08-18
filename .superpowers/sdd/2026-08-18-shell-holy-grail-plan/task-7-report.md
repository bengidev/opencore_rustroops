# Task 7 report: main tab strip

## Implementation

- Added a centered titlebar tab strip after the 68px traffic-light-safe inset.
- Rendered tab chips with active styling, select-on-chip-click, `×` close controls, and `+` stub-tab creation.
- Added `TabDrag` payloads and GPUI `on_drag`/`drag_over`/`on_drop` handlers. Drops compute the post-removal target index before calling `TabModel::reorder`.
- Added an `overflow_x_scroll` strip with a persistent `ScrollHandle` and 36px left/right fade overlays.
- The main stub page now displays the active tab title.
- Added `Shell` tab mutation helpers. Each mutation synchronizes `TabModel` into `ShellChrome`, calls `schedule_save`, and notifies the shell.

## Files

- `src/app/shell/shell_view.rs`
- `.superpowers/sdd/2026-08-18-shell-holy-grail-plan/task-7-report.md`

## TDD evidence

RED:

```text
cargo test shell_view::tests::tab_model_snapshot_syncs_into_persisted_chrome --lib
error[E0432]: unresolved import `super::tab_drop_index`
error[E0599]: no method named `sync_tab_model_to_chrome`
```

GREEN:

```text
cargo test tab_ --lib
running 6 tests
test result: ok. 6 passed; 0 failed
```

The new tests cover model-to-chrome synchronization and source-removal-aware drop indexing. `tabs.rs` was not modified, so the deferred model collision/active-id test item was left for final triage.

## Verification

```text
cargo test --lib
running 93 tests
test result: ok. 93 passed; 0 failed

cargo fmt -- --check
passed

cargo build --release
Finished `release` profile [optimized] target(s) in 0.45s

git diff --check
passed
```

## Self-review and concerns

- Scope is limited to the shell view and the required report; no model or unrelated files were changed.
- Save scheduling is intentionally called after select, close, add, and reorder handlers, including no-op model operations, matching the task ruling that every tab mutation path schedules persistence.
- Manual visual QA was not performed because this environment does not provide an observable running GPUI window. Overflow appearance, chip hit targets, and drag feel should be checked in Task 7 manual QA.
- Existing untracked design/plan documents were present before this task and were not touched.
