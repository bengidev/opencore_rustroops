# Task 8 Report: Debounced persistence bridge

## Implementation

- Added a retained `gpui::Task<()>` to `OpenCoreApp` for shell persistence.
- Shell snapshots now merge immediately into `state.preferences.shell`.
- Each snapshot schedules a 400ms foreground timer; replacing the retained task drops/cancels the prior pending timer, so only the latest pending snapshot flushes.
- The flush clones the current full `AppPreferences` before calling `store.save`, preserving theme and onboarding fields.
- Save failures continue through `record_persistence_error("save shell", error)`.
- Successful dev reset cancels any pending shell save task so stale chrome cannot be written after defaults are restored.
- Added a mutated `ShellChrome` JSON round-trip test and focused GPUI persistence tests.

## Files changed

- `src/app/app_desktop.rs`
- `src/app/shell/shell_view.rs`
- `.superpowers/sdd/2026-08-18-shell-holy-grail-plan/task-8-report.md`

## TDD evidence

RED was captured with:

```text
cargo test shell_persistence_tests -- --nocapture
```

The focused test build failed because `OpenCoreApp::schedule_shell_save` did not exist. After the minimal bridge implementation, GREEN was captured with:

```text
cargo test shell_persistence_tests -- --nocapture
cargo test mutated_shell_chrome_round_trips_through_json -- --nocapture
```

Results: 3 persistence tests passed; 1 mutated JSON round-trip test passed.

The tests cover immediate in-memory merge, unrelated preference preservation at the merge boundary, 400ms debounce timing, latest-write behavior, save-error recording, and mutated shell JSON round-trip.

## Verification

```text
cargo fmt --all -- --check
```

Passed.

```text
cargo test --lib
```

Passed: 102 passed, 0 failed.

```text
cargo build --release
```

Passed: optimized release build completed.

```text
git diff --check
```

Passed.

## Self-review

The persistence callback now updates live state before any asynchronous wait. The delayed write reads the current full preferences document at flush time rather than persisting a stale partial copy. Replacing the retained GPUI task provides cancellation for pending writes, and reset clears the task to protect the default shell state.

## Concerns

- Manual GUI quit/relaunch QA was not observable in this environment and was not claimed.
- No production persistence error was induced outside the focused invalid-store test; the existing error path is reused directly.
