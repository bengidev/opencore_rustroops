# Shell Tab Rename Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Double-click a shell tab to rename it inline; chips size to their title; close button appears only on hover.

**Architecture:** Pure rename rules live on `TabModel`. `Shell` owns optional rename session state (`renaming_tab_id` + `Entity<InputState>` + subscriptions). Tab chips drop fixed width for min/max content sizing; close is hover-gated and suppressed while renaming. Commit/cancel use `InputEvent` (Enter/Blur) plus Escape handling that cancels without saving.

**Tech Stack:** Rust 2024, GPUI 0.2.x, `gpui_component::input::{Input, InputState, InputEvent}` (pinned rev `063e55bbc4fb13907a988111e3581595cbcaefde`), existing `ShellChrome` persistence via `schedule_save`.

## Global Constraints

- Canonical spec: `docs/design/2026-08-19-shell-tab-rename-design.md`.
- Enter commits; Escape cancels; blur commits.
- Empty/whitespace-only commit keeps the previous title (trim first).
- Content-sized chips: no fixed `TAB_CHIP_WIDTH`; soft floor + soft ceiling.
- Close (`×`) hover-only for all tabs; hidden while that tab is renaming.
- Double-click must not start tab drag or titlebar maximize.
- Persist titles through existing chrome sync + debounced save.
- Do not change panel resize / traffic-light / onboarding behavior except as needed for tab chip layout.
- Prefer unit tests in `tabs.rs` / pure helpers; add GPUI harness tests only when matching existing `shell_view.rs` patterns.

## File map

| File | Responsibility |
|------|----------------|
| `src/app/shell/tabs.rs` | `TabModel::rename` + unit tests |
| `src/app/shell/shell_view.rs` | Chip sizing, hover close, rename session, Input UI, click handlers |
| `docs/design/2026-08-19-shell-tab-rename-design.md` | Spec (already written; do not rewrite unless behavior changes) |

---

### Task 1: `TabModel::rename`

**Files:**
- Modify: `src/app/shell/tabs.rs`
- Test: same file `#[cfg(test)]`

**Interfaces:**
- Consumes: `TabModel`, `ShellTabRecord`
- Produces:
  ```rust
  pub fn rename(&mut self, id: &str, title: impl AsRef<str>) {
      // trim; unknown id no-op; empty after trim no-op; else set title
  }
  ```

- [ ] **Step 1: Write the failing tests**

Add to `tabs.rs` tests:

```rust
#[test]
fn rename_updates_known_tab_title() {
    let mut m = model_two();
    m.rename("a", "Alpha");
    assert_eq!(m.tabs[0].title, "Alpha");
    assert_eq!(m.active_id, "a");
}

#[test]
fn rename_trims_whitespace() {
    let mut m = model_two();
    m.rename("b", "  Beta  ");
    assert_eq!(m.tabs[1].title, "Beta");
}

#[test]
fn rename_empty_or_whitespace_keeps_previous_title() {
    let mut m = model_two();
    m.rename("a", "   ");
    assert_eq!(m.tabs[0].title, "A");
    m.rename("a", "");
    assert_eq!(m.tabs[0].title, "A");
}

#[test]
fn rename_unknown_id_is_noop() {
    let mut m = model_two();
    m.rename("missing", "Nope");
    assert_eq!(m, model_two());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test -p opencore --lib app::shell::tabs::tests::rename -- --nocapture
```

Expected: FAIL (method `rename` not found) or compile error.

- [ ] **Step 3: Implement `rename`**

```rust
pub fn rename(&mut self, id: &str, title: impl AsRef<str>) {
    let trimmed = title.as_ref().trim();
    if trimmed.is_empty() {
        return;
    }
    let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
        return;
    };
    tab.title = trimmed.to_owned();
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run the same `cargo test` command.

Expected: PASS (4 tests).

- [ ] **Step 5: Commit** (only if the user asked for commits)

```bash
git add src/app/shell/tabs.rs
git commit -m "$(cat <<'EOF'
feat(shell): add TabModel::rename with trim and empty reject

EOF
)"
```

---

### Task 2: Content-sized chips + hover-only close

**Files:**
- Modify: `src/app/shell/shell_view.rs` (constants, `titlebar_chip`, `render_tab_chip`)

**Interfaces:**
- Consumes: existing `render_tab_chip` / `titlebar_chip`
- Produces:
  - Replace fixed width with:
    ```rust
    const TAB_CHIP_MIN_WIDTH: f32 = 64.0;
    const TAB_CHIP_MAX_WIDTH: f32 = 280.0;
    const TAB_CHIP_GAP: f32 = 4.0; // keep
    ```
  - Remove `TAB_CHIP_WIDTH` (or stop using it).
  - `titlebar_chip`: `.min_w(px(TAB_CHIP_MIN_WIDTH)).max_w(px(TAB_CHIP_MAX_WIDTH))` instead of `.w(px(TAB_CHIP_WIDTH))`.
  - Title label: prefer full text (`whitespace_nowrap`); apply `overflow_hidden` + `text_ellipsis` only as overflow protection under `max_w`.
  - Close button: visible only when the chip is hovered; use GPUI group hover:

```rust
// on chip root:
.group("shell-tab")
// on close button wrapper:
.invisible()
.group_hover("shell-tab", |style| style.visible())
```

If `group` / `group_hover` naming differs in this GPUI pin, mirror whatever pattern already works in-repo or use `StatefulInteractiveElement` hover state (`hovered` flag via `on_hover`) to toggle close visibility — prefer the smallest change that matches GPUI APIs in-tree.

Keep close `on_mouse_down` prevent_default / stop_propagation as today.

- [ ] **Step 1: Write a pure sizing helper test (optional but preferred)**

Near other helpers in `shell_view.rs` tests (or next to `titlebar_chip`):

```rust
fn tab_chip_width_for_title_len(estimated_text_px: f32) -> f32 {
    estimated_text_px.clamp(TAB_CHIP_MIN_WIDTH, TAB_CHIP_MAX_WIDTH)
}

#[test]
fn tab_chip_width_clamps_to_soft_floor_and_ceiling() {
    assert_eq!(tab_chip_width_for_title_len(10.0), TAB_CHIP_MIN_WIDTH);
    assert_eq!(tab_chip_width_for_title_len(120.0), 120.0);
    assert_eq!(tab_chip_width_for_title_len(999.0), TAB_CHIP_MAX_WIDTH);
}
```

Note: layout itself is flex-driven; the helper documents the soft bounds used by `min_w`/`max_w`. If you skip the helper, still assert constants exist and chips no longer call `.w(px(112.0))`.

- [ ] **Step 2: Apply layout + hover close in `titlebar_chip` / `render_tab_chip`**

Concrete chip changes:

```rust
div()
    .id(id.into())
    .h_full()
    .min_w(px(TAB_CHIP_MIN_WIDTH))
    .max_w(px(TAB_CHIP_MAX_WIDTH))
    // ... existing padding/flex ...
```

Title child: keep flex_1 but do **not** force a fixed outer width. Close child: hover-gated as above.

- [ ] **Step 3: Compile-check / run existing shell tests**

```bash
cargo test -p opencore --lib app::shell -- --nocapture
```

Expected: PASS (or only pre-existing failures unrelated to chips).

- [ ] **Step 4: Manual visual check**

Run the app, add tabs with short/long titles, confirm chips grow with title and `×` appears only on hover.

- [ ] **Step 5: Commit** (only if the user asked for commits)

```bash
git add src/app/shell/shell_view.rs
git commit -m "$(cat <<'EOF'
fix(shell): size tab chips to title and show close on hover

EOF
)"
```

---

### Task 3: Rename session state + commit/cancel helpers

**Files:**
- Modify: `src/app/shell/shell_view.rs` (`Shell` fields, `new`, rename helpers)

**Interfaces:**
- Consumes: `TabModel::rename`, `InputState`, `InputEvent`, `Subscription`
- Produces on `Shell`:

```rust
renaming_tab_id: Option<String>,
rename_input: Option<Entity<InputState>>,
_rename_subscriptions: Vec<Subscription>, // or Option<Subscription> pair
```

Helper methods (exact names to use):

```rust
fn begin_rename(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>);
fn commit_rename(&mut self, cx: &mut Context<Self>);
fn cancel_rename(&mut self, cx: &mut Context<Self>);
fn clear_rename_session(&mut self);
```

Behavior:

- `begin_rename`:
  - if tab missing, return
  - `select` that tab (via existing `select_tab` or model+sync)
  - store `renaming_tab_id = Some(id)`
  - create `InputState::new(window, cx).default_value(current_title)` (do **not** enable `clean_on_escape`)
  - subscribe:
    - `InputEvent::PressEnter { .. }` → `commit_rename`
    - `InputEvent::Blur` → `commit_rename`
  - focus the input (`input.update(... focus ...)`)
  - `cx.notify()`

- `commit_rename`:
  - take id + read `input.read(cx).value()`
  - `tab_model.rename(&id, value)`
  - `sync_tab_model_to_chrome` + `schedule_save` + `clear_rename_session` + `cx.notify()`

- `cancel_rename`:
  - `clear_rename_session` only (no model change) + `cx.notify()`

- `clear_rename_session`:
  - clear id, drop input entity handle, clear subscriptions

Escape: because `InputState::escape` propagates when `clean_on_escape` is false, attach on the rename field or chip:

```rust
.on_key_down(cx.listener(|shell, event: &KeyDownEvent, _, cx| {
    if is_escape_keystroke(event) {
        cx.stop_propagation();
        shell.cancel_rename(cx);
    }
}))
```

Mirror `is_enter_keystroke` from `onboarding_view.rs` for Escape (`"escape"` keystroke), or compare `event.keystroke.key` per existing app patterns.

- [ ] **Step 1: Write unit-style tests for commit rules via `TabModel` (already covered) + a small Shell helper test if practical**

If constructing full `Shell` in unit tests is heavy, skip GPUI and rely on Task 1 + a pure function:

```rust
fn effective_rename_title(previous: &str, draft: &str) -> String {
    let trimmed = draft.trim();
    if trimmed.is_empty() {
        previous.to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[test]
fn effective_rename_title_rejects_blank_draft() {
    assert_eq!(effective_rename_title("Welcome", "  "), "Welcome");
    assert_eq!(effective_rename_title("Welcome", " Home "), "Home");
}
```

Prefer calling `TabModel::rename` from `commit_rename` rather than duplicating logic — helper above is only if you want an extra assertion without mounting Input.

- [ ] **Step 2: Add fields + helpers to `Shell`**

Update `Shell::new` to initialize:

```rust
renaming_tab_id: None,
rename_input: None,
_rename_subscriptions: Vec::new(),
```

Also update any test constructors that build `Shell { ... }` literals in `shell_view.rs` tests (there is at least one around the end of the file).

- [ ] **Step 3: Implement helpers as specified above**

Import:

```rust
use gpui::{KeyDownEvent, Subscription};
use gpui_component::input::{Input, InputEvent, InputState};
```

`begin_rename` must receive `&mut Window` — change call sites accordingly (listeners that currently omit window need the window parameter).

- [ ] **Step 4: Compile**

```bash
cargo check -p opencore
```

Expected: success (UI still not wired to double-click yet is OK if helpers are used from a stub path, or keep helpers `#[allow(dead_code)]` until Task 4).

- [ ] **Step 5: Commit** (only if the user asked for commits)

```bash
git add src/app/shell/shell_view.rs
git commit -m "$(cat <<'EOF'
feat(shell): add tab rename session commit and cancel helpers

EOF
)"
```

---

### Task 4: Wire double-click rename UI into `render_tab_chip`

**Files:**
- Modify: `src/app/shell/shell_view.rs` (`render_tab_chip`, click handler)

**Interfaces:**
- Consumes: Task 2 chip layout, Task 3 session helpers
- Produces: interactive rename UX per spec

- [ ] **Step 1: Change chip `on_click` to branch on `click_count`**

Replace select-only handler with:

```rust
let rename_id = tab.id.clone();
let on_chip_click = cx.listener(move |shell, event: &gpui::ClickEvent, window, cx| {
    cx.stop_propagation();
    if event.click_count() >= 2 {
        shell.begin_rename(&rename_id, window, cx);
        return;
    }
    shell.select_tab(&rename_id, cx);
});
```

Use `>= 2` (not `== 2`) only if platform reports higher counts; otherwise `== 2` matching panel-reset handlers is fine — pick one and stay consistent with `shell_view.rs` seam handlers (`== 2`).

- [ ] **Step 2: While `renaming_tab_id == Some(tab.id)`, render Input instead of title label**

```rust
.when_some(self.rename_input.clone().filter(|_| {
    self.renaming_tab_id.as_deref() == Some(tab.id.as_str())
}), |chip, input| {
    chip.child(
        Input::new(&input)
            .appearance(false)
            .bordered(false)
            .focus_bordered(false)
            .with_size(gpui_component::Size::XSmall) // or Small if XSmall missing
            .flex_1()
            .min_w(px(48.0))
            .on_key_down(/* Escape → cancel_rename */),
    )
})
.when(self.renaming_tab_id.as_deref() != Some(tab.id.as_str()), |chip| {
    chip.child(/* existing title label */)
})
```

Hide close button entirely while renaming that tab (even if hovered).

- [ ] **Step 3: Disable drag while renaming**

If `renaming_tab_id` is `Some`, do not attach `on_drag` / treat drag as no-op for that chip so double-click cannot start a drag. Single-click select on other tabs should still `commit_rename` via Input blur.

- [ ] **Step 4: Run shell tests + check**

```bash
cargo test -p opencore --lib app::shell -- --nocapture
cargo check -p opencore
```

Expected: PASS / success.

- [ ] **Step 5: Manual QA checklist**

1. Double-click tab → field focused, title selected or caret ready  
2. Type new name, Enter → chip label updates, width follows title  
3. Double-click, edit, click away → commits  
4. Double-click, edit, Escape → restores previous title  
5. Clear all text, Enter → previous title kept  
6. Hover shows `×`; while renaming, `×` hidden  
7. Restart app → renamed title still present  
8. Double-click does not maximize window / start drag  

- [ ] **Step 6: Commit** (only if the user asked for commits)

```bash
git add src/app/shell/shell_view.rs src/app/shell/tabs.rs
git commit -m "$(cat <<'EOF'
feat(shell): inline double-click tab rename with content-sized chips

EOF
)"
```

---

## Spec coverage (self-review)

| Spec requirement | Task |
|------------------|------|
| Double-click → inline rename | Task 4 |
| Inactive tab selects then renames | Task 3 `begin_rename` / Task 4 |
| Enter commit / Escape cancel / blur commit | Task 3 |
| Empty/whitespace keeps previous | Task 1 (+ Task 3 uses it) |
| Persist via chrome save | Task 3 `commit_rename` |
| Content-sized chips + soft floor/ceiling | Task 2 |
| Close on hover; hidden while renaming | Task 2 + Task 4 |
| No drag / titlebar maximize from double-click | Task 4 |
| `TabModel::rename` tests | Task 1 |

No TBD placeholders. Types consistent: `rename(&str, impl AsRef<str>)`, session fields as listed, helpers `begin_rename` / `commit_rename` / `cancel_rename`.
