# Task 5 Report — ShellWorkspace + AppDesktop wiring

## Status: Complete

## Summary

Replaced hand-rolled `Shell` with `ShellWorkspace` (title bar center tabs + `DockArea` + `StatusBar`). `AppDesktop` now loads/saves `preferences.dock_layout` with restored debounce, flush-on-shutdown, and flush-on-window-close. Deleted obsolete shell modules and `ShellChrome`.

## Changes

- **Added** `src/app/shell/workspace.rs` — `ShellWorkspace`, `DockSaveFn`, load/default/recover center host, layout-change subscription, status bar dock toggles
- **Updated** `src/app/app_desktop.rs` — `Entity<ShellWorkspace>`, `PendingDockSave`, real flush, `schedule_dock_layout_save`, `TitleBar::title_bar_options()` on non-Linux
- **Updated** `src/app/shell/mod.rs` — exports workspace; removed old shell surface
- **Deleted** `shell_view.rs`, `tabs.rs`, `tween.rs`, `chrome.rs`, `shared/preferences/shell_chrome.rs`
- **Tests** — `dock_layout_persistence_tests` (merge/debounce/error/shutdown); reset tests updated for `DockAreaState`

## Verification

```
cargo check -p opencore_rustroops  # pass
cargo test -p opencore_rustroops --lib  # 66 passed
```

## Concerns / follow-ups (Task 6)

- Load failure / version mismatch falls back to default with `eprintln!` but no dedicated `#[gpui::test]` yet
- `set_theme` on workspace is a no-op (theme applied at app level)
- Center host recovery walks dock tree; exotic saved layouts may re-apply default

## Commit

```
feat(shell): replace hand-rolled Shell with Dock workspace
```

## Fixups

- **Immediate in-memory merge** — `schedule_dock_layout_save` now sets `state.preferences.dock_layout` synchronously before debouncing disk write; test asserts in-memory layout before clock advance.
- Commit: `fix(shell): merge dock_layout into preferences immediately`
