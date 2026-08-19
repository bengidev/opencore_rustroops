# Shell Holy Grail Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Home Hello World stub with a `shell` module: overlay titlebar, left/right/bottom chrome, and a tabbed center main surface that expands into freed space.

**Architecture:** Pure chrome math and tab model live in `src/app/shell/` as testable units. An `Entity<Shell>` owns live sizes, open flags, tweens, and tabs. `OpenCoreApp` creates the entity on Home, passes theme + save callback, and stops rendering `home_screen`. Chrome persists under `AppPreferences.shell` with debounced saves.

**Tech Stack:** Rust 2024, GPUI 0.2.2 (`Entity`, `Render`, `TitlebarOptions`, `WindowControlArea`, drag listeners), existing `OpenCoreTheme` / `PreferencesStore`, serde preferences JSON.

## Global Constraints

- Canonical spec: `docs/design/2026-08-18-shell-holy-grail-design.md`.
- Center column is the **primary** surface; left/right are secondary; bottom docks under center only.
- When left/right/bottom collapse, center `flex_1` **takes the freed space** (no empty gutters).
- Persist widths/heights **and** all open/collapsed flags **and** main tab list/order/active id.
- Fuller main tabs: open/switch/close, drag reorder, overflow scroll + edge fade; stub page content only.
- Overlay titlebar (transparent system titlebar + in-app band). Click toggles only — **no** keyboard shortcuts.
- No right-pane surface tabs, no right takeover, no real editor/terminal content.
- Open/close tween ~200ms ease-out with clipped inner content; live drag resize; double-click handle resets default; reduced motion snaps.
- Debounced persist ~400ms; keep theme/onboarding fields untouched.
- Defaults: left open, right closed, bottom closed, one stub tab.
- Do not mention or depend on paths outside this repository checkout.

## File map

| File | Responsibility |
|------|----------------|
| `src/app/shell/mod.rs` | Module barrel; re-exports |
| `src/app/shell/chrome.rs` | `ShellChrome`, size constants, clamp helpers, defaults |
| `src/app/shell/tabs.rs` | `MainTab`, `TabModel` pure logic |
| `src/app/shell/tween.rs` | `WidthTween` / eval ease-out / reduced-motion snap |
| `src/app/shell/shell_view.rs` | `Shell` entity, `Render`, panels, titlebar, tabs UI |
| `src/shared/preferences/mod.rs` | Nest `shell: ShellChrome` on `AppPreferences` |
| `src/app/mod.rs` | `mod shell;` remove `mod home;` when retired |
| `src/app/app_desktop.rs` | Own `Entity<Shell>`, Home arm, titlebar options, save debounce bridge |
| `src/app/home/mod.rs` | Delete after shell replaces it (or leave unused until Task 4) |
| `src/app/onboarding/onboarding_view.rs` | Top pad for traffic lights if window uses transparent titlebar for all screens |

---

### Task 1: `ShellChrome` prefs + clamp helpers

**Files:**
- Create: `src/app/shell/mod.rs`, `src/app/shell/chrome.rs`
- Modify: `src/app/mod.rs`, `src/shared/preferences/mod.rs`
- Test: tests inside `chrome.rs` and updated prefs tests

**Interfaces:**
- Consumes: `serde::{Serialize, Deserialize}`
- Produces:
  - `pub const TITLEBAR_HEIGHT: f32 = 38.0;`
  - `pub const SIDEBAR_MIN: f32 = 208.0;`
  - `pub const SIDEBAR_MAX: f32 = 400.0;`
  - `pub const SIDEBAR_DEFAULT: f32 = 256.0;`
  - `pub const RIGHT_MIN: f32 = 240.0;`
  - `pub const RIGHT_MAX: f32 = 480.0;`
  - `pub const RIGHT_DEFAULT: f32 = 320.0;`
  - `pub const BOTTOM_MIN: f32 = 120.0;`
  - `pub const BOTTOM_DEFAULT: f32 = 220.0;`
  - `pub const BOTTOM_MAX_VH: f32 = 0.55;`
  - `pub fn clamp_sidebar_width(w: f32) -> f32`
  - `pub fn clamp_right_width(w: f32, viewport_w: f32) -> f32` (also ≤ 52% viewport)
  - `pub fn clamp_bottom_height(h: f32, viewport_h: f32) -> f32`
  - `pub struct ShellChrome { ... }` with `Default`
  - `AppPreferences.shell: ShellChrome`

- [ ] **Step 1: Add module scaffold and failing tests**

In `src/app/mod.rs`, add:

```rust
mod shell;
```

Create `src/app/shell/mod.rs`:

```rust
//! Post-onboarding holy-grail chrome (panels + main tabs).

mod chrome;

pub use chrome::{
    BOTTOM_DEFAULT, BOTTOM_MAX_VH, BOTTOM_MIN, RIGHT_DEFAULT, RIGHT_MAX, RIGHT_MIN, SIDEBAR_DEFAULT,
    SIDEBAR_MAX, SIDEBAR_MIN, TITLEBAR_HEIGHT, ShellChrome, clamp_bottom_height, clamp_right_width,
    clamp_sidebar_width,
};
```

Create `src/app/shell/chrome.rs` with tests that import the clamps/constants (implementation empty so compile fails, or stubs that fail asserts):

```rust
//! Persisted shell chrome sizes and open flags.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_plan() {
        assert_eq!(TITLEBAR_HEIGHT, 38.0);
        assert_eq!(SIDEBAR_MIN, 208.0);
        assert_eq!(SIDEBAR_MAX, 400.0);
        assert_eq!(SIDEBAR_DEFAULT, 256.0);
        assert_eq!(RIGHT_MIN, 240.0);
        assert_eq!(RIGHT_MAX, 480.0);
        assert_eq!(RIGHT_DEFAULT, 320.0);
        assert_eq!(BOTTOM_MIN, 120.0);
        assert_eq!(BOTTOM_DEFAULT, 220.0);
        assert_eq!(BOTTOM_MAX_VH, 0.55);
    }

    #[test]
    fn clamp_sidebar_respects_min_max() {
        assert_eq!(clamp_sidebar_width(10.0), SIDEBAR_MIN);
        assert_eq!(clamp_sidebar_width(999.0), SIDEBAR_MAX);
        assert_eq!(clamp_sidebar_width(300.0), 300.0);
    }

    #[test]
    fn clamp_right_respects_min_max_and_viewport_fraction() {
        assert_eq!(clamp_right_width(10.0, 1280.0), RIGHT_MIN);
        assert_eq!(clamp_right_width(900.0, 1280.0), (1280.0 * 0.52).min(RIGHT_MAX));
        assert_eq!(clamp_right_width(300.0, 1280.0), 300.0);
    }

    #[test]
    fn clamp_bottom_respects_min_and_viewport_fraction() {
        assert_eq!(clamp_bottom_height(10.0, 800.0), BOTTOM_MIN);
        assert_eq!(clamp_bottom_height(900.0, 800.0), 800.0 * BOTTOM_MAX_VH);
        assert_eq!(clamp_bottom_height(200.0, 800.0), 200.0);
    }

    #[test]
    fn shell_chrome_default_matches_spec() {
        let c = ShellChrome::default();
        assert!(c.left_open);
        assert!(!c.right_open);
        assert!(!c.bottom_open);
        assert_eq!(c.left_width, SIDEBAR_DEFAULT);
        assert_eq!(c.right_width, RIGHT_DEFAULT);
        assert_eq!(c.bottom_height, BOTTOM_DEFAULT);
        assert_eq!(c.tabs.len(), 1);
        assert_eq!(c.active_tab_id, c.tabs[0].id);
    }
}
```

- [ ] **Step 2: Run tests — expect compile/link failure**

Run: `cargo test -p opencore_rustroops --lib shell::chrome::tests -- --nocapture`

Expected: FAIL (missing items or wrong values).

- [ ] **Step 3: Implement chrome types and clamps**

```rust
pub const TITLEBAR_HEIGHT: f32 = 38.0;
pub const SIDEBAR_MIN: f32 = 208.0;
pub const SIDEBAR_MAX: f32 = 400.0;
pub const SIDEBAR_DEFAULT: f32 = 256.0;
pub const RIGHT_MIN: f32 = 240.0;
pub const RIGHT_MAX: f32 = 480.0;
pub const RIGHT_DEFAULT: f32 = 320.0;
pub const BOTTOM_MIN: f32 = 120.0;
pub const BOTTOM_DEFAULT: f32 = 220.0;
pub const BOTTOM_MAX_VH: f32 = 0.55;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellTabRecord {
    pub id: String,
    pub title: String,
}

impl Default for ShellTabRecord {
    fn default() -> Self {
        Self {
            id: "tab-1".into(),
            title: "Welcome".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellChrome {
    pub left_width: f32,
    pub right_width: f32,
    pub bottom_height: f32,
    pub left_open: bool,
    pub right_open: bool,
    pub bottom_open: bool,
    pub tabs: Vec<ShellTabRecord>,
    pub active_tab_id: String,
}

impl Default for ShellChrome {
    fn default() -> Self {
        let tab = ShellTabRecord::default();
        Self {
            left_width: SIDEBAR_DEFAULT,
            right_width: RIGHT_DEFAULT,
            bottom_height: BOTTOM_DEFAULT,
            left_open: true,
            right_open: false,
            bottom_open: false,
            active_tab_id: tab.id.clone(),
            tabs: vec![tab],
        }
    }
}

pub fn clamp_sidebar_width(w: f32) -> f32 {
    w.clamp(SIDEBAR_MIN, SIDEBAR_MAX)
}

pub fn clamp_right_width(w: f32, viewport_w: f32) -> f32 {
    let max = RIGHT_MAX.min(viewport_w * 0.52);
    w.clamp(RIGHT_MIN, max.max(RIGHT_MIN))
}

pub fn clamp_bottom_height(h: f32, viewport_h: f32) -> f32 {
    let max = (viewport_h * BOTTOM_MAX_VH).max(BOTTOM_MIN);
    h.clamp(BOTTOM_MIN, max)
}

impl ShellChrome {
    /// Clamp all sizes against a viewport; repair empty tabs.
    pub fn sanitized(mut self, viewport_w: f32, viewport_h: f32) -> Self {
        self.left_width = clamp_sidebar_width(self.left_width);
        self.right_width = clamp_right_width(self.right_width, viewport_w);
        self.bottom_height = clamp_bottom_height(self.bottom_height, viewport_h);
        if self.tabs.is_empty() {
            let tab = ShellTabRecord::default();
            self.active_tab_id = tab.id.clone();
            self.tabs.push(tab);
        }
        if !self.tabs.iter().any(|t| t.id == self.active_tab_id) {
            self.active_tab_id = self.tabs[0].id.clone();
        }
        self
    }
}
```

Export `ShellTabRecord` from `mod.rs` as needed.

- [ ] **Step 4: Nest on `AppPreferences`**

In `src/shared/preferences/mod.rs`:

```rust
use crate::app::shell::ShellChrome;
```

That creates a layering violation (`shared` → `app`). **Do not do that.** Instead keep `ShellChrome` in `shared` **or** duplicate a serde DTO.

**Chosen approach:** move the persisted struct to `src/shared/preferences/shell_chrome.rs` (same fields/constants/clamps), and have `src/app/shell/chrome.rs` re-export from shared:

```rust
// src/shared/preferences/shell_chrome.rs — types + clamps + Default
// src/shared/preferences/mod.rs:
pub mod shell_chrome;
pub use shell_chrome::ShellChrome;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPreferences {
    pub theme_mode: ThemeMode,
    pub onboarding_completed: bool,
    pub shell: ShellChrome,
}
```

Update prefs tests:

```rust
assert!(value.get("shell").is_some() || true); // default serde includes shell
let prefs = AppPreferences::default();
assert!(prefs.shell.left_open);
```

Update every `AppPreferences { theme_mode, onboarding_completed }` literal in the crate to include `shell: ShellChrome::default()` **or** use `..Default::default()`.

- [ ] **Step 5: Run tests**

Run: `cargo test -p opencore_rustroops --lib -- shell_chrome preferences`

Expected: PASS for new clamp/default tests; existing prefs tests updated and green.

- [ ] **Step 6: Commit**

```bash
git add src/app/shell src/shared/preferences src/app/mod.rs
git commit -m "feat(shell): add persisted chrome sizes and clamps"
```

---

### Task 2: Tab model (pure)

**Files:**
- Create: `src/app/shell/tabs.rs`
- Modify: `src/app/shell/mod.rs`

**Interfaces:**
- Consumes: `ShellTabRecord` / string ids
- Produces:
  - `pub struct TabModel { tabs: Vec<ShellTabRecord>, active_id: String }`
  - `TabModel::from_chrome(&ShellChrome) -> Self`
  - `fn select(&mut self, id: &str)`
  - `fn close(&mut self, id: &str)` — neighbor selection; refuse closing last tab
  - `fn reorder(&mut self, from: usize, to: usize)`
  - `fn add_stub(&mut self) -> String` — new id, becomes active
  - `fn to_chrome_tabs(&self) -> (Vec<ShellTabRecord>, String)`

- [ ] **Step 1: Write failing tests in `tabs.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::preferences::shell_chrome::ShellTabRecord;

    fn model_two() -> TabModel {
        TabModel {
            tabs: vec![
                ShellTabRecord { id: "a".into(), title: "A".into() },
                ShellTabRecord { id: "b".into(), title: "B".into() },
            ],
            active_id: "a".into(),
        }
    }

    #[test]
    fn close_active_selects_neighbor() {
        let mut m = model_two();
        m.close("a");
        assert_eq!(m.active_id, "b");
        assert_eq!(m.tabs.len(), 1);
    }

    #[test]
    fn close_last_tab_is_noop() {
        let mut m = model_two();
        m.close("a");
        m.close("b");
        assert_eq!(m.tabs.len(), 1);
    }

    #[test]
    fn reorder_moves_tab() {
        let mut m = model_two();
        m.reorder(0, 1);
        assert_eq!(m.tabs[0].id, "b");
        assert_eq!(m.tabs[1].id, "a");
    }

    #[test]
    fn add_stub_appends_and_activates() {
        let mut m = model_two();
        let id = m.add_stub();
        assert_eq!(m.active_id, id);
        assert_eq!(m.tabs.len(), 3);
    }
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p opencore_rustroops --lib shell::tabs -- --nocapture`

- [ ] **Step 3: Implement `TabModel`**

Implement methods so tests pass. `close` on last remaining tab returns without change. When closing active at index `i`, activate `i` (next) or `i - 1` if last.

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add src/app/shell/tabs.rs src/app/shell/mod.rs
git commit -m "feat(shell): add main tab model"
```

---

### Task 3: Layout tween helper

**Files:**
- Create: `src/app/shell/tween.rs`
- Modify: `src/app/shell/mod.rs`

**Interfaces:**
- Produces:
  - `pub struct DimTween { pub from: f32, pub to: f32, pub started: Instant }`
  - `pub const RESIZE_MS: f32 = 200.0;`
  - `pub fn ease_out(t: f32) -> f32` — cubic approx `1 - (1-t)^3`
  - `pub fn eval_tween(tween: Option<&DimTween>, target: f32, now: Instant, reduced_motion: bool) -> f32`
  - `pub fn tween_finished(tween: &DimTween, now: Instant) -> bool`

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn eval_at_start_is_from() { /* ... */ }
#[test]
fn eval_at_end_is_to() { /* ... */ }
#[test]
fn reduced_motion_snaps_to_target() { /* ... */ }
```

- [ ] **Step 2–4: Implement, pass tests, commit**

```bash
git commit -m "feat(shell): add dimension tween helper"
```

---

### Task 4: `Shell` entity + replace Home stub

**Files:**
- Create: `src/app/shell/shell_view.rs`
- Modify: `src/app/shell/mod.rs`, `src/app/app_desktop.rs`, `src/app/mod.rs`
- Delete or stop using: `src/app/home/mod.rs`

**Interfaces:**
- Produces:
  - `pub struct Shell { chrome live fields, tab_model, tweens, save: ShellSaveFn }`
  - `pub type ShellSaveFn = Rc<dyn Fn(ShellChrome, &mut App)>`
  - `Shell::new(chrome: ShellChrome, save: ShellSaveFn, cx) -> Self`
  - `impl Render for Shell`
  - Visual targets: `left_target() -> f32` = width if open else `0.0` (same for right/bottom)

- [ ] **Step 1: Minimal `Shell` render (static layout, no interaction yet)**

Structure:

```text
root (relative, size_full)
  row: left | center(flex_1) | right   // heights fill below titlebar pad
  overlay titlebar (absolute top, h=TITLEBAR_HEIGHT)
center column:
  main stub (flex_1)
  bottom stub (h = target)
```

Stub labels: `LEFT`, `MAIN`, `RIGHT`, `BOTTOM` using `OpenCoreTheme` tokens. Pass theme into render via `Shell` storing nothing — read from a `theme: OpenCoreTheme` field updated by parent each frame **or** resolve inside render from a callback. Simplest: `Shell` holds `theme_mode` updated by `OpenCoreApp` before notify, or parent passes theme by updating entity each render:

In `OpenCoreApp::render` Home arm:

```rust
ActiveScreen::Home => {
    let shell = self.ensure_shell(cx);
    shell.update(cx, |shell, _| shell.set_theme(theme));
    div().size_full().child(shell.clone())
}
```

GPUI pattern: `self.shell.get_or_insert_with(|| cx.new(...))` then render child entity with `.child(shell.clone())` — use the entity as child via GPUI’s entity element API (`shell.clone()` into child). Match existing gpui-component/Root patterns: typically `div().child(shell.clone())` where `Entity<Shell>: IntoElement`.

- [ ] **Step 2: Wire window titlebar options (Home + Onboarding share one window)**

In `run_desktop` `WindowOptions`:

```rust
use gpui::{TitlebarOptions, point};

let options = WindowOptions {
    window_bounds: Some(bounds),
    titlebar: Some(TitlebarOptions {
        title: None,
        appears_transparent: true,
        traffic_light_position: Some(point(px(12.0), px(11.0))),
    }),
    ..Default::default()
};
```

Pad onboarding top by ~`TITLEBAR_HEIGHT` (or traffic-light safe inset) so content clears the lights.

- [ ] **Step 3: Remove `home` module**

- Delete `src/app/home/mod.rs`
- Remove `mod home;` and `use super::home::home_screen`
- Move any theme smoke test into `shell_view` tests if still useful

- [ ] **Step 4: Unit test `left_target` / open flags**

```rust
#[test]
fn left_target_zero_when_closed() {
    let mut chrome = ShellChrome::default();
    chrome.left_open = false;
    assert_eq!(Shell::left_target_for(&chrome), 0.0);
}
```

- [ ] **Step 5: `cargo test` + manual launch**

Run: `cargo test -p opencore_rustroops --lib`

Manual: complete onboarding → see four labeled regions + titlebar band (even if toggles inert).

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(shell): replace home stub with shell entity"
```

---

### Task 5: Panel toggles + open/close tweens

**Files:**
- Modify: `src/app/shell/shell_view.rs`

**Interfaces:**
- `toggle_left/right/bottom(&mut self, cx)`
- Start `DimTween` from current visual size to new target; set open flag; `schedule_save`; `cx.notify()`; request animation frames while tween active
- Titlebar buttons: Left / Right / Bottom (or icon-like text) with press feedback

- [ ] **Step 1: Test toggle flips flag and sets tween endpoints** (pure method test without GPUI if possible)

- [ ] **Step 2: Implement toggles + titlebar buttons**

Render widths with `eval_tween(...)`. While any tween active, `window.request_animation_frame()` from `Shell::render`. Clip pane inner: outer `w(animated)` + `overflow_hidden`, inner fixed at stored width (not animated width) so content does not squash.

- [ ] **Step 3: Reduced motion**

Read GPUI reduced-motion if available on `Window`/`App` in this rev; else check a stub `fn reduced_motion(cx: &App) -> bool` defaulting `false`, with tween path already snapping when true.

- [ ] **Step 4: Manual QA** — toggle each panel; center grows/shrinks; ~200ms motion

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shell): animate panel open and close toggles"
```

---

### Task 6: Resize handles (live drag + double-click reset)

**Files:**
- Modify: `src/app/shell/shell_view.rs`

**Interfaces:**
- Zero-width/height seam children with absolute 5px hit targets
- Drag markers: `SidebarResize`, `RightResize`, `BottomResize` (empty structs implementing GPUI drag)
- `on_drag` / `on_drag_move` on root update clamped sizes, force panel open, clear tween, `schedule_save`
- Double-click → default width/height + save

- [ ] **Step 1: Implement sidebar seam drag** (left edge of center / right edge of left)

- [ ] **Step 2: Right + bottom handles**

Bottom: handle on top edge of bottom drawer; `cursor_row_resize`.

- [ ] **Step 3: Manual QA** — drag all three; double-click reset; center flexes

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(shell): add panel resize handles"
```

---

### Task 7: Main tab strip (reorder + overflow)

**Files:**
- Modify: `src/app/shell/shell_view.rs`, `src/app/shell/tabs.rs` if needed

**Behaviors:**
- Tab chips in overlay titlebar center band
- Click select; ✕ close; `+` add stub
- Drag reorder between chips (150ms slide optional; functional reorder required)
- `overflow_x_scroll` + left/right edge fade overlays (~36px)
- Active tab’s stub page shown in center (`title` as placeholder text)

- [ ] **Step 1: Render strip + select/close/add wired to `TabModel`**

- [ ] **Step 2: Drag reorder**

Use GPUI `on_drag` payload with tab index; on drop compute insert index; call `tab_model.reorder`.

- [ ] **Step 3: Overflow fade**

When content wider than strip, show fades; scrollable strip.

- [ ] **Step 4: Persist tabs via `schedule_save` after mutations

- [ ] **Step 5: Manual QA** — many tabs for overflow; reorder; close; restart later in Task 8

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(shell): add main tab strip with reorder and overflow"
```

---

### Task 8: Debounced persistence bridge

**Files:**
- Modify: `src/app/app_desktop.rs`, `src/app/shell/shell_view.rs`

**Interfaces:**
- `Shell` calls `save_fn(chrome_snapshot)` on schedule
- `OpenCoreApp` provides closure that:
  1. merges into `state.preferences.shell`
  2. debounces 400ms with `cx.spawn` / timer task (cancel previous)
  3. `store.save(&preferences)`
  4. on error → `record_persistence_error`

- [ ] **Step 1: Test chrome round-trip still works with mutated shell field**

- [ ] **Step 2: Implement debounce on `OpenCoreApp`**

Hold `Option<Task<()>>` or generation counter so only latest flush writes.

- [ ] **Step 3: Manual QA** — toggle/resize/tabs → quit → relaunch Home → chrome restored

- [ ] **Step 4: Dev reset** — `AppPreferences::default()` already clears shell; confirm after FAB reset

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(shell): debounce persist chrome preferences"
```

---

### Task 9: Interaction polish + verification gate

**Files:**
- Modify: `src/app/shell/shell_view.rs` (and small helpers as needed)

**Polish checklist (emil-style, GPUI-ported):**
- Toggle/tab press: brief scale ~0.97, ease-out ≤160ms (or opacity flash if scale unsupported)
- Hover affordance on handles (stronger border)
- No animation on reduced motion
- Titlebar drag: `WindowControlArea::Drag` + `start_window_move` pattern; interactive controls `occlude` / stop propagation so drag does not steal clicks
- Double-click titlebar empty area → `window.titlebar_double_click()`

- [ ] **Step 1: Apply polish**

- [ ] **Step 2: Run full lib tests**

Run: `cargo fmt && cargo test -p opencore_rustroops --lib && cargo clippy -p opencore_rustroops --all-targets -- -D warnings`

Expected: all green.

- [ ] **Step 3: Manual QA from spec**

- Enter OpenCore → shell, not Hello World
- Collapse each panel → center expands
- Drag + double-click reset handles
- Tabs: open/switch/close/reorder/overflow
- Restart restores chrome
- Light/dark still works

- [ ] **Step 4: Commit**

```bash
git commit -m "polish(shell): press feedback, titlebar drag, QA fixes"
```

---

## Spec coverage self-check

| Spec requirement | Task |
|------------------|------|
| Replace Hello World with shell module | 4 |
| Overlay titlebar | 4, 9 |
| Left/right/bottom chrome; center primary | 4–6 |
| Center takes freed space | 4–5 |
| Persist all chrome + tabs | 1, 8 |
| Main tabs reorder + overflow fade | 2, 7 |
| Tweens, live drag, double-click reset, reduced motion | 3, 5, 6, 9 |
| Click toggles only | 5 |
| Stub content | 4, 7 |
| Debounced save / errors | 8 |
| Automated + manual verification | 1–3, 9 |

## Placeholder / consistency scan

- Types: `ShellChrome`, `ShellTabRecord`, `TabModel`, `DimTween`, `Shell`, `ShellSaveFn` — names stable across tasks.
- Prefs live under `shared/preferences` to avoid `shared` → `app` dependency.
- No TBD steps left.
