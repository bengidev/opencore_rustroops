# Adaptive GUI scaling — Design

**Date:** 2026-08-13  
**Status:** Spec drafted; awaiting user review before the implementation plan  
**Path note:** Canonical copy lives under `docs/design/` (`docs/superpowers/` is gitignored in this repo).  
**Stack:** Rust + GPUI + gpui-component  
**Reference (behavior, not copy):** [OpenCode docs](https://opencode.ai/docs/) desktop layout — readable column, vertical scroll, side margins grow on a wide window.

## Goal

When the user drags a window edge, onboarding and home **reflow** instead of clipping or zooming. Content uses a centered max-width column; overflow **scrolls vertically**. Type, spacing, and ASCII glyph size stay on current tokens.

## Decisions (locked)

| Topic | Choice |
|--------|--------|
| Layout model | Fluid reflow (not uniform zoom, not breakpoint chrome) |
| Screens | All current screens (onboarding and home) |
| Too-small window | Vertical scroll; no type/spacing compression |
| Wide window | Readable column; side margins grow |
| Approach | Shared page shell at the composition root |
| Window size persistence | Not in v1 |
| Launch / transition sizes | Unchanged: 960×680 onboarding, 1280×800 home, including the Enter / reset jump |
| Type / spacing / ASCII px | Unchanged tokens |

## Non-goals (v1)

- Uniform scale / `rem` zoom of the whole UI
- Sidebar / breakpoint desktop chrome
- Stepping type, spacing, or hero density at widths
- Persisting window bounds across launches
- Removing `WindowResizeIntent` or `center_window`
- Horizontal page scroll as the primary overflow (ASCII may clip inside its box)

---

## Section 1 — Architecture

A shared page shell wraps whichever screen is active. Screens do not each implement scroll + max-width.

`OpenCoreApp::render` tree:

```
div.size_full.relative
  fluid_page (vertical scroll, min-height 100% inner column)
    Onboarding | Home
  debug FAB (overlay, not inside the scroller)
```

- **Outer `fluid_page`:** fills the window; gpui-component vertical scrollbar when content is taller than the viewport.
- **Inner column:** `min-height: 100%` flex column so short pages keep today’s pinning (Enter at the bottom on onboarding; Hello World vertically centered on home).
- **Readable column:** `width: 100%`, centered, `max-width` 680 for onboarding hero/copy and the home stack. Page padding stays **16px horizontal / 20px vertical** (today’s `EDGE_INSET_H` / `EDGE_INSET_V`).
- **Header:** full width of the padded page (logo + theme toggle), not squeezed into 680px.
- **Tokens:** type, spacing, and ASCII box pixel sizes do not scale. Copy wraps. The ASCII box uses `max_w_full` and existing `overflow_hidden` (glyphs clip on a very narrow window).
- **Window:** launch sizes and the programmatic jump stay. `WindowOptions.window_min_size` is **360×240** so the frame cannot vanish. v1 does not save window size.
- **Scroll:** `ScrollHandle` on `OpenCoreApp`; reset to top on `ActiveScreen` change. User scroll is left alone while they stay on a screen.
- **Debug FAB:** clamp against **live** `window.viewport_size()`, not the launch size.

---

## Section 2 — Components

New module `src/app/layout/`:

| Unit | Responsibility |
|------|----------------|
| `fluid_page(scroll, child)` | Full-window vertical scroll; inner `min-height: 100%` column |
| `content_column(max_width, child)` | `width: 100%`, centered, capped at `max_width` |
| `column_width(viewport, inset, max)` | Pure: `min(max(0, viewport - 2×inset), max)` |
| Constants | `CONTENT_MAX_WIDTH = 680`, `PAGE_INSET_H = 16`, `PAGE_INSET_V = 20`, `WINDOW_MIN_SIZE = 360×240` |

`OpenCoreApp` owns the `ScrollHandle`, wraps both screens in `fluid_page`, sets `window_min_size`, and resets scroll on onboarding ↔ home.

**Onboarding.** `main_column` is the min-height page body (not the native window). Header stays full padded width. Hero + subtitle go through `content_column(680)`. ASCII box stays 320px with `max_w_full`. Enter row stays at the bottom of the min-height column. Focus/Enter key handling on `onboarding_interactive_root` is unchanged; that root fills the page inside the scroller.

**Home.** Same shell. Hello World stack in `content_column(680)`, vertically centered when the window is taller than the stack.

**Debug FAB.** Extract a pure origin clamp used on render and during drag, with live viewport size. After a shrink, an origin outside the viewport is moved back inside on the next paint.

---

## Section 3 — Data flow

Nothing new is persisted. Preferences remain `theme_mode` + `onboarding_completed`.

1. Native resize updates the GPUI viewport.
2. Next `render`: `fluid_page` fills that viewport; flex applies `w_full` + `max_w`; `column_width` is for tests and any explicit numeric caller.
3. FAB is the only widget that reads `window.viewport_size()` to clamp.
4. `ScrollHandle` lives on `OpenCoreApp`, not `AppState`. On `ActiveScreen` change (Enter, debug reset), reset to top before the next paint. If the named scroll element is not mounted yet (first frame), skip — the next paint starts at the top.
5. `WindowResizeIntent` still fires on onboarding complete / reset; `apply_resize_intent` still resizes and re-centers. Dragging the edge after that only changes the native window; layout follows on the next render.

---

## Section 4 — Error handling

No new persistence or network paths. Onboarding save failures stay as today’s inline `[ERROR: …]`.

- `column_width`: viewport `0` or missing → `0`; never negative; no panic.
- FAB origin outside the new viewport → clamp on next render (same damping math, live size).
- OS min size is a floor only. If a resize intent is larger than the display, keep `center_window` as it is — no new failure UI.
- Scroll reset is best-effort on the first frame.

---

## Section 5 — Testing

### Automated

- `column_width`: wide viewport caps at 680; narrow equals `viewport - 32`; `0` viewport returns `0`; never negative.
- Constants: max width 680, horizontal inset 16, vertical inset 20, OS min 360×240.
- Existing onboarding ASCII constant test and home “builds for both themes” tests still pass.
- Existing `WindowResizeIntent` tests still assert 960×680 ↔ 1280×800.
- FAB clamp helper: origin outside a smaller viewport is moved back inside.

### Manual (`cargo run`)

- Drag onboarding narrower than 680: copy wraps, side inset stays, ASCII box shrinks below 320 if needed, vertical scroll if the stack is taller than the window.
- Drag wider: hero stays 680, side margins grow.
- Enter → home still resizes to 1280×800 and re-centers; Hello World stays centered in a 680 column.
- Drag home short: stack scrolls instead of clipping.
- Debug: FAB stays on-screen after a live resize.

### Done when

Automated tests above pass; manual resize checks pass on onboarding and home; spec reviewed; implementation plan written only after that review.

---

## Open points for the implementation plan (not blockers)

- Exact gpui-component scrollbar API (`overflow_y_scrollbar` vs `Scrollable` + `ScrollHandle`) at the pinned crate rev.
- Whether home’s vertical centering uses `justify_center` on the min-height inner column or a flex spacer pair — visual result is the same.
