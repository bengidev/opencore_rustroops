# Adaptive GUI Scaling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Onboarding and home reflow when the user drags a window edge: centered 680px column, vertical scroll, unchanged type/spacing tokens.

**Architecture:** A shared `fluid_page` at `OpenCoreApp::render` fills the viewport, scrolls vertically via a `ScrollHandle` + gpui-component scrollbar, and hosts an inner `min-height: 100%` column. Screens use `content_column` for the readable cap. Launch sizes and `WindowResizeIntent` stay. Debug FAB clamps to live `viewport_size()`.

**Tech Stack:** Rust 2024, GPUI (`ScrollHandle`, `overflow_y_scroll`, `track_scroll`, `WindowOptions.window_min_size`), gpui-component (`scroll::ScrollableElement::vertical_scrollbar`), existing `OpenCoreTheme`.

## Global Constraints

- Layout model is **fluid reflow** (not uniform zoom, not breakpoint chrome).
- Screens: **onboarding and home**.
- Too-small window: **vertical scroll**; no type/spacing compression.
- Wide window: **readable column**; side margins grow.
- Shared page shell at the composition root.
- Window size persistence: **not in v1**.
- Launch / transition sizes **unchanged**: 960×680 onboarding, 1280×800 home, including the Enter / reset jump.
- Type / spacing / ASCII px stay on **current tokens**.
- Constants: `CONTENT_MAX_WIDTH = 680`, `PAGE_INSET_H = 16`, `PAGE_INSET_V = 20`, `WINDOW_MIN_SIZE = 360×240`.
- Horizontal page scroll is **not** the primary overflow (ASCII may clip inside its box).
- Preferences remain `theme_mode` + `onboarding_completed`.
- Do **not** remove `WindowResizeIntent` or `center_window`.
- Canonical spec: `docs/design/2026-08-13-adaptive-gui-scaling-design.md`.

## File map

| File | Responsibility |
|------|----------------|
| `src/app/layout/mod.rs` | Constants, `column_width`, `window_min_size`, `reset_scroll`, `fluid_page`, `content_column` |
| `src/app/mod.rs` | `mod layout;` |
| `src/app/app_desktop.rs` | Own `ScrollHandle`, wrap screens in `fluid_page`, set `window_min_size`, reset scroll on screen change, live FAB viewport |
| `src/app/onboarding/onboarding_view.rs` | `min_h` page body, `content_column` for hero/copy, ASCII `max_w_full`, insets from layout constants |
| `src/app/home/mod.rs` | `min_h` page body, `content_column` for Hello World stack |
| `src/app/dev_reset/mod.rs` | Pure `clamp_fab_origin`; drag uses live viewport |

---

### Task 1: Layout constants and `column_width`

**Files:**
- Create: `src/app/layout/mod.rs`
- Modify: `src/app/mod.rs` (add `mod layout;`)

**Interfaces:**
- Consumes: nothing
- Produces:
  - `pub const CONTENT_MAX_WIDTH: f32 = 680.0;`
  - `pub const PAGE_INSET_H: f32 = 16.0;`
  - `pub const PAGE_INSET_V: f32 = 20.0;`
  - `pub const WINDOW_MIN_WIDTH: f32 = 360.0;`
  - `pub const WINDOW_MIN_HEIGHT: f32 = 240.0;`
  - `pub fn column_width(viewport: f32, inset: f32, max: f32) -> f32`
  - `pub fn window_min_size() -> gpui::Size<gpui::Pixels>`

- [ ] **Step 1: Declare the module and write failing tests**

In `src/app/mod.rs`, add with the other `mod` lines (after `mod home;` is fine):

```rust
mod layout;
```

Create `src/app/layout/mod.rs` with tests only (no items yet — this must fail to compile):

```rust
//! Shared fluid page shell: vertical scroll + centered max-width column.

#[cfg(test)]
mod tests {
    use super::{
        CONTENT_MAX_WIDTH, PAGE_INSET_H, PAGE_INSET_V, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
        column_width, window_min_size,
    };
    use gpui::{px, size};

    #[test]
    fn constants_match_spec() {
        assert_eq!(CONTENT_MAX_WIDTH, 680.0);
        assert_eq!(PAGE_INSET_H, 16.0);
        assert_eq!(PAGE_INSET_V, 20.0);
        assert_eq!(WINDOW_MIN_WIDTH, 360.0);
        assert_eq!(WINDOW_MIN_HEIGHT, 240.0);
    }

    #[test]
    fn column_width_caps_at_max_on_wide_viewport() {
        assert_eq!(column_width(1280.0, PAGE_INSET_H, CONTENT_MAX_WIDTH), 680.0);
    }

    #[test]
    fn column_width_uses_viewport_minus_insets_when_narrow() {
        assert_eq!(column_width(400.0, PAGE_INSET_H, CONTENT_MAX_WIDTH), 368.0);
    }

    #[test]
    fn column_width_zero_viewport_returns_zero() {
        assert_eq!(column_width(0.0, PAGE_INSET_H, CONTENT_MAX_WIDTH), 0.0);
    }

    #[test]
    fn column_width_never_negative() {
        assert_eq!(column_width(10.0, PAGE_INSET_H, CONTENT_MAX_WIDTH), 0.0);
    }

    #[test]
    fn window_min_size_is_360_by_240() {
        assert_eq!(
            window_min_size(),
            size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib app::layout::tests -- --nocapture`

Expected: compile error `cannot find function 'column_width' in this scope` (and missing constants).

- [ ] **Step 3: Add the minimal implementation above the tests in `src/app/layout/mod.rs`**

```rust
//! Shared fluid page shell: vertical scroll + centered max-width column.

use gpui::{px, size, Pixels, Size};

/// Readable column cap for onboarding hero/copy and the home stack.
pub const CONTENT_MAX_WIDTH: f32 = 680.0;

/// Horizontal page inset (onboarding `EDGE_INSET_H`).
pub const PAGE_INSET_H: f32 = 16.0;

/// Vertical page inset (onboarding `EDGE_INSET_V`).
pub const PAGE_INSET_V: f32 = 20.0;

/// OS window minimum width so the frame cannot vanish.
pub const WINDOW_MIN_WIDTH: f32 = 360.0;

/// OS window minimum height so the frame cannot vanish.
pub const WINDOW_MIN_HEIGHT: f32 = 240.0;

/// Inner column width: `min(max(0, viewport - 2×inset), max)`.
pub fn column_width(viewport: f32, inset: f32, max: f32) -> f32 {
    let inner = (viewport - 2.0 * inset).max(0.0);
    inner.min(max)
}

/// Native window minimum size for `WindowOptions.window_min_size`.
pub fn window_min_size() -> Size<Pixels> {
    size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))
}
```

Keep the `#[cfg(test)] mod tests` block from Step 1 below this.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib app::layout::tests -- --nocapture`

Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add src/app/layout/mod.rs src/app/mod.rs
git commit -m "feat(layout): add fluid column width helpers and window min size"
```

---

### Task 2: `fluid_page`, `content_column`, and `reset_scroll`

**Files:**
- Modify: `src/app/layout/mod.rs`

**Interfaces:**
- Consumes: `CONTENT_MAX_WIDTH` from Task 1
- Produces:
  - `pub fn reset_scroll(scroll: &gpui::ScrollHandle)`
  - `pub fn fluid_page(scroll: &gpui::ScrollHandle, child: impl gpui::IntoElement) -> impl gpui::IntoElement`
  - `pub fn content_column(max_width: f32, child: impl gpui::IntoElement) -> impl gpui::IntoElement`

- [ ] **Step 1: Write failing tests**

Append to the tests module in `src/app/layout/mod.rs`:

```rust
    use super::{content_column, fluid_page, reset_scroll};
    use gpui::{div, point, ScrollHandle};

    #[test]
    fn reset_scroll_sets_offset_to_zero() {
        let scroll = ScrollHandle::new();
        scroll.set_offset(point(px(10.0), px(40.0)));
        reset_scroll(&scroll);
        assert_eq!(scroll.offset(), point(px(0.0), px(0.0)));
    }

    #[test]
    fn fluid_page_builds() {
        let scroll = ScrollHandle::new();
        let _ = fluid_page(&scroll, div().child("page"));
    }

    #[test]
    fn content_column_builds_at_spec_max_width() {
        let _ = content_column(CONTENT_MAX_WIDTH, div().child("column"));
    }
```

Add `content_column`, `fluid_page`, `reset_scroll` to the existing `use super::{...}` list in that tests module.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib app::layout::tests -- --nocapture`

Expected: compile error `cannot find function 'reset_scroll'` (and `fluid_page` / `content_column`).

- [ ] **Step 3: Implement the builders**

Add to the imports at the top of `src/app/layout/mod.rs`:

```rust
use gpui::{
    div, point, px, relative, size, InteractiveElement, IntoElement, ParentElement, Pixels,
    ScrollHandle, Size, StatefulInteractiveElement, Styled,
};
use gpui_component::scroll::ScrollableElement as _;
```

Remove the previous `use gpui::{px, size, Pixels, Size};` so there is a single import block.

Add these functions after `window_min_size`:

```rust
/// Jump the page scroller to the top-left origin.
pub fn reset_scroll(scroll: &ScrollHandle) {
    scroll.set_offset(point(px(0.0), px(0.0)));
}

/// Full-window vertical scroller with an inner min-height 100% column.
pub fn fluid_page(scroll: &ScrollHandle, child: impl IntoElement) -> impl IntoElement {
    div()
        .id("fluid-page")
        .size_full()
        .relative()
        .child(
            div()
                .id("fluid-page-scroll")
                .size_full()
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .track_scroll(scroll)
                .child(
                    div()
                        .w_full()
                        .min_h(relative(1.0))
                        .flex()
                        .flex_col()
                        .child(child),
                ),
        )
        .vertical_scrollbar(scroll)
}

/// Centered readable column: `width: 100%` capped at `max_width`.
pub fn content_column(max_width: f32, child: impl IntoElement) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .justify_center()
        .child(div().w_full().max_w(px(max_width)).child(child))
}
```

Do **not** put page padding on `fluid_page`. Screens keep (or take) `PAGE_INSET_*` so onboarding is not double-padded when this shell is wired in Task 3.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib app::layout::tests -- --nocapture`

Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add src/app/layout/mod.rs
git commit -m "feat(layout): add fluid_page, content_column, and scroll reset"
```

---

### Task 3: Wire the shell in `OpenCoreApp`

**Files:**
- Modify: `src/app/app_desktop.rs`
- Modify: `src/app/onboarding/onboarding_view.rs` (`size_full` → `min_h(relative(1.0))` on page roots)
- Modify: `src/app/home/mod.rs` (same `min_h` change so the scroller can grow)

**Interfaces:**
- Consumes: `fluid_page`, `reset_scroll`, `window_min_size` from Task 2 / Task 1
- Produces:
  - `OpenCoreApp.page_scroll: ScrollHandle`
  - `OpenCoreApp.scrolled_screen: ActiveScreen`
  - `OpenCoreApp::sync_page_scroll(&mut self)`
  - `WindowOptions.window_min_size: Some(window_min_size())`

Changing screen roots off `size_full` in this task is required: a `size_full` child of the scroller is locked to the viewport and **never scrolls**.

- [ ] **Step 1: Write a failing unit test for scroll sync**

Add to `src/app/layout/mod.rs` tests (this documents the offset API the app will call). If Task 2 already covers `reset_scroll_sets_offset_to_zero`, add this test in `src/app/app_desktop.rs` tests module instead — a pure helper local to the desktop file:

In `src/app/app_desktop.rs`, add next to the existing `#[cfg(test)] mod tests`:

```rust
    use crate::app::layout::reset_scroll;
    use gpui::{point, px, ScrollHandle};

    #[test]
    fn sync_page_scroll_resets_offset_when_invoked() {
        let scroll = ScrollHandle::new();
        scroll.set_offset(point(px(0.0), px(80.0)));
        reset_scroll(&scroll);
        assert_eq!(scroll.offset().y, px(0.0));
    }
```

This test should already pass if Task 2 landed. That is OK — it locks the helper the desktop layer will call. The rest of this task is wiring (no GPUI window tests in this repo).

- [ ] **Step 2: Run the desktop tests**

Run: `cargo test --lib app::app_desktop::tests -- --nocapture`

Expected: PASS, including `sync_page_scroll_resets_offset_when_invoked` and existing `WindowResizeIntent` tests (960×680 ↔ 1280×800 must still pass).

- [ ] **Step 3: Wire `OpenCoreApp` and `WindowOptions`**

In `src/app/app_desktop.rs` imports, add `ScrollHandle` to the `gpui::{...}` list and:

```rust
use super::layout::{fluid_page, reset_scroll, window_min_size};
```

Add fields to `OpenCoreApp`:

```rust
    page_scroll: ScrollHandle,
    scrolled_screen: ActiveScreen,
```

In `OpenCoreApp::new`, after reading `state.active_screen` for onboarding UI, initialize:

```rust
            page_scroll: ScrollHandle::new(),
            scrolled_screen: state.active_screen,
```

Add this method on `impl OpenCoreApp`:

```rust
    fn sync_page_scroll(&mut self) {
        if self.scrolled_screen != self.state.active_screen {
            reset_scroll(&self.page_scroll);
            self.scrolled_screen = self.state.active_screen;
        }
    }
```

At the top of `Render::render`, immediately after `self.apply_resize_intent(window, cx);`:

```rust
        self.sync_page_scroll();
        let page_scroll = self.page_scroll.clone();
```

Replace the onboarding / home `content` assignment so the extra `div().size_full()` wrappers are gone (the shell owns the viewport). Both match arms wrap in `div().child(...)` so they unify as `Div`:

```rust
        let content = match self.state.active_screen {
            ActiveScreen::Onboarding => {
                let _ = self
                    .onboarding_ui
                    .get_or_insert_with(OnboardingUiState::new);
                if let Some(ui) = self.onboarding_ui.as_mut() {
                    ui.tick(now);
                }
                let ui = self.onboarding_ui.as_ref().expect("inserted");
                let callbacks = OnboardingCallbacks::from_app(cx.entity().downgrade());
                let persistence_error = self.persistence_error.as_deref();
                let on_enter = callbacks.on_enter.clone();

                div().child(onboarding_interactive_root(
                    &self.focus_handle,
                    on_enter,
                    onboarding_screen(theme, ui, callbacks, persistence_error),
                ))
            }
            ActiveScreen::Home => div().child(home_screen(theme)),
        };
        let page = fluid_page(&page_scroll, content);
```

In the debug overlay, change `.child(content)` to `.child(page)`. In the release branch, `page` instead of `content`:

```rust
        #[cfg(not(debug_assertions))]
        {
            page
        }
```

In `run_desktop`, set the min size:

```rust
                let options = WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(window_min_size()),
                    ..Default::default()
                };
```

Leave FAB bounds on `initial_window_size()` until Task 6.

Then change page roots so they can grow past the viewport.

`onboarding_interactive_root` in `src/app/onboarding/onboarding_view.rs`:

```rust
    div()
        .w_full()
        .min_h(relative(1.0))
        .tab_index(0)
        .track_focus(focus_handle)
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if is_enter_keystroke(event) {
                on_enter(window, cx);
            }
        })
        .child(content)
```

`onboarding_screen`:

```rust
    div()
        .w_full()
        .min_h(relative(1.0))
        .bg(background)
        .child(main_column(theme, ui, callbacks, persistence_error))
```

`main_column` opening chain: replace `.size_full()` with `.w_full().min_h(relative(1.0))`. Keep `.p(px(EDGE_INSET_V)).px(px(EDGE_INSET_H))` for now.

`home_screen` in `src/app/home/mod.rs`: add `relative` to the gpui import and replace `.size_full()` with `.w_full().min_h(relative(1.0))`. Keep `justify_center` / `items_center`.

- [ ] **Step 4: Run tests**

Run: `cargo test --all-targets`

Expected: PASS, including layout tests, `sync_page_scroll_resets_offset_when_invoked`, `take_pending_window_resize_clears_intent`, `completing_onboarding_records_window_resize_intent`, `home_screen_builds_for_both_themes`, `ascii_hero_layout_constants`.

- [ ] **Step 5: Commit**

```bash
git add src/app/app_desktop.rs src/app/onboarding/onboarding_view.rs src/app/home/mod.rs
git commit -m "feat(app): wrap screens in fluid_page and set window min size"
```

---

### Task 4: Onboarding readable column and ASCII `max_w_full`

**Files:**
- Modify: `src/app/onboarding/onboarding_view.rs`

**Interfaces:**
- Consumes: `content_column`, `CONTENT_MAX_WIDTH`, `PAGE_INSET_H`, `PAGE_INSET_V` from Task 1–2
- Produces: onboarding hero/copy in `content_column(CONTENT_MAX_WIDTH)`; ASCII box `w(px(320)).max_w_full()`; insets equal layout constants

- [ ] **Step 1: Update the layout-constant test (fails until insets/hero width source change)**

Replace `ascii_hero_layout_constants` in `src/app/onboarding/onboarding_view.rs` with:

```rust
    #[test]
    fn ascii_hero_layout_constants() {
        assert_eq!(CONTENT_MAX_WIDTH, 680.0);
        assert_eq!(PAGE_INSET_H, 16.0);
        assert_eq!(PAGE_INSET_V, 20.0);
        assert_eq!(ASCII_HERO_HEIGHT, 360.0);
        assert_eq!(HERO_GLOW_INSET_H, 44.0);
        assert_eq!(HERO_GLOW_INSET_TOP, 46.0);
        assert_eq!(HERO_GLOW_INSET_BOTTOM, 34.0);
        assert_eq!(ASCII_TEXT_SIZE, 9.0);
        assert_eq!(ASCII_BOX_SIZE, 320.0);
        assert_eq!(ENTER_BUTTON_HEIGHT, 48.0);
        assert_eq!(COLS, 74);
        assert_eq!(ROWS, 44);
    }
```

Add to the test module imports:

```rust
    use crate::app::layout::{CONTENT_MAX_WIDTH, PAGE_INSET_H, PAGE_INSET_V};
```

Remove `assert_eq!(HERO_MAX_WIDTH, 680.0);` — `HERO_MAX_WIDTH` will be deleted.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib app::onboarding::onboarding_view::tests::ascii_hero_layout_constants -- --nocapture`

Expected: compile error `cannot find value 'CONTENT_MAX_WIDTH' in this scope` if the `use` was not added, or PASS on constants already if Step 1 included the `use`. If Step 1 included the `use`, the test may **pass immediately** (constants live in `layout`). That is OK — proceed to the view change. The behavior lock is the assertions above plus existing `COLS`/`ROWS`.

- [ ] **Step 3: Reflow the onboarding view**

At the top of `src/app/onboarding/onboarding_view.rs` add:

```rust
use crate::app::layout::{content_column, CONTENT_MAX_WIDTH, PAGE_INSET_H, PAGE_INSET_V};
```

Delete `const HERO_MAX_WIDTH`, `EDGE_INSET_H`, and `EDGE_INSET_V`.

In `main_column`, use layout insets:

```rust
        .p(px(PAGE_INSET_V))
        .px(px(PAGE_INSET_H))
```

Replace `hero_block` so the 680 cap goes through `content_column`. Full new function:

```rust
fn hero_block(theme: OpenCoreTheme, ui: &OnboardingUiState) -> impl IntoElement {
    let primary = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let ascii_color = theme.foreground(ForegroundToken::Primary);
    let grotesk = SharedString::from("Space Grotesk");
    let spacing = theme.spacing;

    let hero_ascii = div()
        .relative()
        .w_full()
        .h(px(ASCII_HERO_HEIGHT))
        .flex()
        .items_center()
        .justify_center()
        .child(hero_glow(theme))
        .child(ascii_box(theme, ui.last_frame(), ascii_color));

    content_column(
        CONTENT_MAX_WIDTH,
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .child(hero_ascii)
            .child(div().h(px(spacing.lg as f32)))
            .child(
                div()
                    .w_full()
                    .text_center()
                    .text_size(px(TypeRole::DisplayMd.size()))
                    .font_family(grotesk.clone())
                    .text_color(primary)
                    .child("Your local AI command workspace"),
            )
            .child(div().h(px(spacing.sm as f32)))
            .child(
                div()
                    .w_full()
                    .text_center()
                    .text_size(px(TypeRole::MonoSm.size()))
                    .line_height(relative(TypeRole::MonoSm.line_height()))
                    .font_family(grotesk)
                    .text_color(secondary)
                    .child("OpenCore combines chat, terminal, editing, and Rust-native performance in one permissioned desktop environment. To leave the crowded cloud, polluted by leaks and unconsciousness, to return to a workspace that stays on your machine."),
            ),
    )
}
```

In `ascii_box`, change the outer box from `.w(px(ASCII_BOX_SIZE)).h(px(ASCII_BOX_SIZE))` to:

```rust
        .w(px(ASCII_BOX_SIZE))
        .max_w_full()
        .h(px(ASCII_BOX_SIZE))
```

Keep existing `overflow_hidden` so glyphs clip on a very narrow window instead of forcing horizontal page scroll.

Header and Enter row stay full width of the padded page (do not wrap them in `content_column`).

- [ ] **Step 4: Run tests**

Run: `cargo test --lib app::onboarding -- --nocapture`

Expected: PASS, including `ascii_hero_layout_constants`.

- [ ] **Step 5: Commit**

```bash
git add src/app/onboarding/onboarding_view.rs
git commit -m "feat(onboarding): reflow hero into a max-width column"
```

---

### Task 5: Home readable column

**Files:**
- Modify: `src/app/home/mod.rs`

**Interfaces:**
- Consumes: `content_column`, `CONTENT_MAX_WIDTH`, `PAGE_INSET_H`, `PAGE_INSET_V`
- Produces: Hello World stack inside `content_column(CONTENT_MAX_WIDTH)`, vertically centered when the window is taller than the stack

- [ ] **Step 1: Extend home tests**

Add to `src/app/home/mod.rs` tests:

```rust
    use crate::app::layout::{CONTENT_MAX_WIDTH, PAGE_INSET_H, PAGE_INSET_V};

    #[test]
    fn home_uses_spec_column_metrics() {
        assert_eq!(CONTENT_MAX_WIDTH, 680.0);
        assert_eq!(PAGE_INSET_H, 16.0);
        assert_eq!(PAGE_INSET_V, 20.0);
    }
```

- [ ] **Step 2: Run the new test**

Run: `cargo test --lib app::home::tests::home_uses_spec_column_metrics -- --nocapture`

Expected: PASS (constants already exist). Proceed to the view change.

- [ ] **Step 3: Wrap the Hello World stack**

Add imports:

```rust
use gpui::{IntoElement, ParentElement, SharedString, Styled, div, px, relative};

use crate::app::layout::{content_column, CONTENT_MAX_WIDTH, PAGE_INSET_H, PAGE_INSET_V};
```

Replace `home_screen` with:

```rust
/// Full-screen Nothing-styled Hello World home screen.
pub fn home_screen(theme: OpenCoreTheme) -> impl IntoElement {
    let page = theme.surface(BackgroundToken::Primary);
    let display = theme.foreground(ForegroundToken::Primary);
    let secondary = theme.foreground(ForegroundToken::Secondary);
    let label = theme.foreground(ForegroundToken::Muted);
    let mono = SharedString::from("Space Mono");

    let stack = div()
        .w_full()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(theme.spacing.md as f32))
        .child(
            div()
                .text_size(px(48.))
                .font_family(SharedString::from("Space Grotesk"))
                .font_weight(gpui::FontWeight::LIGHT)
                .text_color(display)
                .child("Hello, World!"),
        )
        .child(
            div()
                .text_size(px(TypeRole::MonoSm.size()))
                .font_family(mono.clone())
                .text_color(secondary)
                .child("OpenCore · GPUI"),
        )
        .child(swatch_row(theme))
        .child(
            div()
                .mt(px(theme.spacing.xl as f32))
                .text_size(px(11.))
                .font_family(mono)
                .text_color(label)
                .child("HOME"),
        );

    div()
        .w_full()
        .min_h(relative(1.0))
        .bg(page)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .p(px(PAGE_INSET_V))
        .px(px(PAGE_INSET_H))
        .child(content_column(CONTENT_MAX_WIDTH, stack))
}
```

48px “Hello, World!” and 32px swatches stay token-sized (no compression).

- [ ] **Step 4: Run tests**

Run: `cargo test --lib app::home -- --nocapture`

Expected: PASS (`home_screen_builds_for_both_themes`, `home_swatch_radius_follows_control_radius`, `home_uses_spec_column_metrics`).

- [ ] **Step 5: Commit**

```bash
git add src/app/home/mod.rs
git commit -m "feat(home): reflow Hello World into a max-width column"
```

---

### Task 6: Debug FAB live viewport clamp

**Files:**
- Modify: `src/app/dev_reset/mod.rs`
- Modify: `src/app/app_desktop.rs` (debug-only FAB bounds + clamp on render)

**Interfaces:**
- Consumes: `window.viewport_size() -> Size<Pixels>`
- Produces:
  - `pub(crate) fn clamp_fab_origin(origin: Point<Pixels>, viewport: Size<Pixels>, fab_width: f32, fab_height: f32) -> Point<Pixels>`
  - Drag callbacks take live viewport each frame
  - On render, origin outside the viewport is moved back inside

- [ ] **Step 1: Write failing clamp tests**

Append to `src/app/dev_reset/mod.rs` tests:

```rust
    use gpui::size;

    #[test]
    fn clamp_fab_origin_moves_outside_point_inside() {
        let origin = Point {
            x: px(900.0),
            y: px(600.0),
        };
        let viewport = size(px(400.0), px(300.0));
        let clamped = clamp_fab_origin(origin, viewport, FAB_WIDTH, FAB_HEIGHT);
        assert_eq!(clamped.x, px(400.0 - FAB_WIDTH));
        assert_eq!(clamped.y, px(300.0 - FAB_HEIGHT));
    }

    #[test]
    fn clamp_fab_origin_leaves_inside_point_unchanged() {
        let origin = Point {
            x: px(10.0),
            y: px(12.0),
        };
        let viewport = size(px(400.0), px(300.0));
        let clamped = clamp_fab_origin(origin, viewport, FAB_WIDTH, FAB_HEIGHT);
        assert_eq!(clamped, origin);
    }

    #[test]
    fn clamp_fab_origin_zero_viewport_pins_to_origin() {
        let origin = Point {
            x: px(80.0),
            y: px(28.0),
        };
        let viewport = size(px(0.0), px(0.0));
        let clamped = clamp_fab_origin(origin, viewport, FAB_WIDTH, FAB_HEIGHT);
        assert_eq!(clamped.x, px(0.0));
        assert_eq!(clamped.y, px(0.0));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib app::dev_reset::tests -- --nocapture`

Expected: compile error `cannot find function 'clamp_fab_origin'`.

- [ ] **Step 3: Implement clamp and wire live viewport**

Add `Size` to the gpui import in `src/app/dev_reset/mod.rs`. Add:

```rust
/// Keep the FAB fully inside `viewport`. Origin is top-left of the FAB.
pub(crate) fn clamp_fab_origin(
    origin: Point<Pixels>,
    viewport: Size<Pixels>,
    fab_width: f32,
    fab_height: f32,
) -> Point<Pixels> {
    let max_x = (viewport.width.as_f32() - fab_width).max(0.0);
    let max_y = (viewport.height.as_f32() - fab_height).max(0.0);
    Point {
        x: px(origin.x.as_f32().clamp(0.0, max_x)),
        y: px(origin.y.as_f32().clamp(0.0, max_y)),
    }
}
```

In `src/app/app_desktop.rs` debug `render` overlay, replace the `initial_window_size()` bounds with live viewport, and clamp before snapshotting:

```rust
        #[cfg(debug_assertions)]
        {
            let viewport = window.viewport_size();
            self.dev_reset_state.origin = super::dev_reset::clamp_fab_origin(
                self.dev_reset_state.origin,
                viewport,
                super::dev_reset::FAB_WIDTH,
                super::dev_reset::FAB_HEIGHT,
            );
            let bounds = (viewport.width.as_f32(), viewport.height.as_f32());
            let callbacks = DevResetCallbacks::from_app(cx.entity().downgrade(), bounds);
            let state_snapshot = self.dev_reset_state.clone();
            let on_drag_move = callbacks.on_drag_move.clone();
            let on_drag_end = callbacks.on_drag_end.clone();

            div()
                .size_full()
                .relative()
                .child(page)
                .child(dev_reset_fab(theme, &state_snapshot, &callbacks))
                .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                    (on_drag_move)(event, window, cx);
                })
                .on_mouse_up(
                    MouseButton::Left,
                    move |event: &MouseUpEvent, window, cx| {
                        (on_drag_end)(event, window, cx);
                    },
                )
        }
```

Leave `DevResetCallbacks::from_app` drag math as it is: it already clamps with `damp_translation` against the `bounds` captured that frame. Because `bounds` is now the live viewport, drag uses the live size.

- [ ] **Step 4: Run tests and CI-equivalent checks**

Run:

```bash
cargo test --all-targets
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

Expected: all tests PASS; fmt check clean; clippy clean.

Manual (`cargo run`), debug build:

- Drag onboarding narrower than 680: copy wraps, 16px side inset stays, ASCII box shrinks below 320 if needed, vertical scroll if the stack is taller than the window.
- Drag wider: hero stays 680, side margins grow.
- Enter → home still resizes to 1280×800 and re-centers; Hello World stays centered in a 680 column.
- Drag home short: stack scrolls instead of clipping.
- FAB stays fully on-screen after a live shrink.

- [ ] **Step 5: Commit**

```bash
git add src/app/dev_reset/mod.rs src/app/app_desktop.rs
git commit -m "fix(dev-reset): clamp FAB to the live viewport on resize"
```

---

## Spec coverage

| Spec requirement | Task |
|------------------|------|
| Shared `fluid_page` at composition root | 2, 3 |
| Vertical scrollbar / `ScrollHandle` | 2, 3 |
| Inner min-height 100% | 2, 3 |
| `content_column` max 680 | 2, 4, 5 |
| Header full padded width | 4 (not wrapped in column) |
| Page padding 16×20 | 4, 5 |
| ASCII 320 + `max_w_full` + clip | 4 |
| Type/spacing tokens unchanged | 4, 5 |
| `window_min_size` 360×240 | 1, 3 |
| Launch sizes + resize intent unchanged | 3 (existing tests) |
| Scroll reset on screen change | 2, 3 |
| No window-size persistence | all (no prefs changes) |
| `column_width` tests | 1 |
| FAB live clamp | 6 |
| Manual resize checklist | 6 |

## Type consistency

- `column_width(viewport: f32, inset: f32, max: f32) -> f32`
- `window_min_size() -> Size<Pixels>`
- `reset_scroll(scroll: &ScrollHandle)`
- `fluid_page(scroll: &ScrollHandle, child: impl IntoElement) -> impl IntoElement`
- `content_column(max_width: f32, child: impl IntoElement) -> impl IntoElement`
- `clamp_fab_origin(origin: Point<Pixels>, viewport: Size<Pixels>, fab_width: f32, fab_height: f32) -> Point<Pixels>`
- `OpenCoreApp.page_scroll: ScrollHandle`
- `OpenCoreApp.scrolled_screen: ActiveScreen`
- `OpenCoreApp::sync_page_scroll(&mut self)`
