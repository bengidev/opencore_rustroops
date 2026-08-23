# Final Branch Review — Dock Shell Migration

**Reviewer:** Final whole-branch review subagent  
**Date:** 2026-08-23  
**Repo:** `opencore_rustroops`  
**Spec:** `docs/superpowers/specs/2026-08-23-dock-shell-design.md`  
**Plan:** `docs/superpowers/plans/2026-08-23-dock-shell.md`  
**Progress:** `.superpowers/sdd/progress.md` (Tasks 1–7 complete)

## Branch scope

**Feature commits (8):**

| Commit | Summary |
|--------|---------|
| `26ceb2f` | `refactor(prefs): replace ShellChrome with optional DockAreaState` |
| `79795db` | `feat(shell): add Dock stub panels and registry` |
| `ac4ab11` | `feat(shell): add default holy-grail Dock layout` |
| `2c60a59` | `feat(shell): render center Dock stubs in TitleBar tab strip` |
| `f934687` | `feat(shell): replace hand-rolled Shell with Dock workspace` |
| `38f67e3` | `fix(shell): merge dock_layout into preferences immediately` |
| `13d1f5b` | `fix(shell): reset Dock layout on load failure or version mismatch` |
| `438274b` | `chore(shell): verification cleanups` (fmt + clippy) |

**Touched surface (post-migration):**

- `src/app/shell/` — `workspace.rs`, `titlebar_tabs.rs`, `panels.rs`, `default_layout.rs`, `mod.rs`
- `src/app/app_desktop.rs` — `ShellWorkspace` wiring, `dock_layout` persistence
- `src/shared/preferences/mod.rs` — `dock_layout: Option<DockAreaState>`
- **Removed:** `shell_view.rs`, `tabs.rs`, `tween.rs`, `chrome.rs`, `shell_chrome.rs`

**Verification (Task 7 report):** `cargo fmt`, `cargo check`, `cargo test --lib` (70 passed), `cargo clippy -D warnings` — all PASS.

---

## Acceptance criteria

| # | Criterion | Verdict | Evidence |
|---|-----------|---------|----------|
| 1 | Post-onboarding UI is Dock-driven (titlebar + dock + status bar) | ✅ | `OpenCoreApp` `ActiveScreen::Home` → `ensure_shell()` → `ShellWorkspace`; render stack: `center_title_bar` + `DockArea` + `StatusBar` (`workspace.rs:123–169`) |
| 2 | Default holy-grail stubs; left open; right/bottom closed | ✅ | `apply_default_holy_grail` sets left `true` @ 256px, right/bottom `false` @ 320/220 (`default_layout.rs`); four `#[gpui::test]` layout assertions in `workspace.rs` |
| 3 | Center tabs in title bar; edge docks keep in-panel tabs | ✅ | `center_title_bar` + `CenterStubHost` internal tabs (`titlebar_tabs.rs`, `panels.rs`); center `DockItem::Panel` renders body only (no Dock center tab chrome per pinned `DockArea` render path); edges use `DockItem::tab` in left/right/bottom docks |
| 4 | Layout persists via `DockAreaState` (immediate memory + debounce disk + quit flush) | ✅ | `DockEvent::LayoutChanged` → `schedule_dock_layout_save` (400ms debounce, immediate `state.preferences.dock_layout` merge per `38f67e3`); `flush_pending_shell_save` on shutdown + window close (`app_desktop.rs`) |
| 5 | Old `ShellChrome` persistence removed | ✅ | No `ShellChrome` / `shell_chrome.rs` in tree; `AppPreferences` has `dock_layout` only; legacy `shell` JSON ignored (`app_preferences_ignores_legacy_shell_field`) |
| 6 | Load failure / version mismatch → default | ⚠️ Partial | In-memory reset via explicit `match` + `eprintln!` + `apply_default_holy_grail` (`workspace.rs:43–73`); four GPUI tests cover None / mismatch / unrecoverable / corrupt center. **Does not persist default after reset** (see Important #1) |
| 7 | No story/demo panels | ✅ | Only `LeftStubPanel`, `RightStubPanel`, `BottomStubPanel`, `MainStubPanel`, `CenterStubHost`; no `gpui-component-story` imports in `src/` |

---

## Findings

### Critical

None.

### Important

1. **Reset path does not save default layout to disk (spec gap).** Design data-flow requires: version mismatch / load failure → default layout → **save**. `ShellWorkspace::new` applies default but never calls `save`; `DockArea::set_center` / `set_*_dock` do not emit `LayoutChanged`, and the layout subscription is registered **after** initial layout setup. Stale `dock_layout` remains on disk until the user mutates layout (toggle dock, resize, etc.). UX still shows default each launch, but prefs never self-heal on quit-without-interaction.

2. **Persistence tests use synthetic `marker_layout` fixtures, not golden `dump`/`load` round-trip.** `dock_layout_persistence_tests` only set `version` on `DockAreaState::default()`. Debounce/shutdown/immediate-merge semantics are covered, but real panel tree serialization is not. Rollup from T5.

3. **No automated test for window-close flush.** Shutdown flush is tested; `App::on_window_closed` → `flush_pending_shell_save` path is not. Rollup from T5.

### Minor

1. **Legacy `shell_*` naming** — `PendingDockSave` vs `pending_shell_save`, `flush_shell_save`, `flush_pending_shell_save` (behavior is dock-layout correct).

2. **`ShellWorkspace::set_theme` is a no-op** — theme applied at app level via `apply_nothing_theme`; acceptable for stub shell.

3. **`dock_load_failure_resets_default` exercises corrupt center panel name / recovery fallback**, not a true `DockArea::load` `Err` (pinned API returns `Ok` today). Defensive `Err` arm untested. Rollup from T6.

4. **No GUI smoke test** — Task 7 verified compile/test gates only; manual checklist (titlebar tabs, toggles, relaunch restore) not run in CI or review.

5. **Right dock toggle uses `StatusBar::child`** instead of `.right()` — cosmetic; matches plan sketch.

6. **Task artifact commit IDs** — some `task-*-report.md` files cite hashes that differ from progress ledger; documentation only.

---

## Per-task rollup (synthesized)

| Task | Per-task verdict | Carried forward |
|------|------------------|-----------------|
| T1 Preferences | ✅ Clean | Persistence tests renamed/wired in T5 |
| T2 Stub panels | ✅ Clean | Last-tab close guard added in T4 |
| T3 Default layout | ✅ Clean | Constants consolidated; `shell_chrome` deleted in T5 |
| T4 Titlebar tabs | ✅ Clean | Wired in T5 `workspace.rs` |
| T5 Workspace + desktop | ✅ Clean | Immediate merge fixed (`38f67e3`); golden dump + window-close test gaps remain |
| T6 Load reset | ✅ Clean | Reset-save gap + true `Err` test gap remain |
| T7 Verification | ✅ Clean | 70 lib tests; clippy/fmt clean |

---

## Architecture notes (positive)

- Clean module split matches plan file structure.
- `CenterStubHost` pattern correctly avoids duplicate center Dock tab chrome while hosting multi-tab state in title bar.
- `register_shell_panels` at boot + `from_panel_state` restore path is sound.
- `recover_center_host` tree walk handles split/tab center items.
- gpui-component pin unchanged at `063e55b…`; APIs match pinned checkout.
- Debounce latest-wins, shutdown flush, save-error recording, and dev-reset cancel-pending-save invariants preserved from old shell.

---

## Ready to merge?

**YES** — with one Important follow-up.

All seven design success criteria are met in runtime behavior. The migration is structurally complete: old hand-rolled shell deleted, Dock workspace wired, persistence restored, tests and clippy green. The reset-without-save gap is a spec deviation, not a launch blocker (users still see the correct default UI; disk self-heals on first layout interaction). Recommend merging and filing a small follow-up to call `save(dock.dump())` after reset paths (or emit an initial save on workspace construct when falling back from bad saved state).

**Suggested post-merge follow-up (non-blocking):**

```rust
// After apply_default_holy_grail in mismatch/Err/recovery paths:
save(dock_area.read(cx).dump(cx), cx);
```

Plus: golden `DockAreaState` fixture test and window-close flush test when convenient.

---

## Fixed

1. **Reset path now persists default layout to disk.** `ShellWorkspace::new` tracks `reset_to_default` across all `apply_default_holy_grail` paths (None, version mismatch, load `Err`, unrecoverable center host) and calls `save(dock.dump(cx))` once after layout setup, before the `LayoutChanged` subscription. `dock_reset_persists_default_layout` `#[gpui::test]` asserts the save callback receives `DOCK_LAYOUT_VERSION` after a version-mismatch construct.
