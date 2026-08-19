# Shell Holy Grail Layout Design

## Goal

Replace the post-onboarding Hello World home stub with a new `shell` module that provides a holy-grail desktop chrome: overlay titlebar, left/right collapsible panels, bottom drawer, and a tabbed **center main surface**. Focus on panel and tab interaction behavior; panel bodies and tab pages stay stubs.

## Decisions locked

- Classic holy grail this slice: left | center | right + bottom under center. No right-pane surface tabs, no right takeover mode.
- Persist full chrome across restarts: widths/heights and all open/collapsed flags, plus main tab list/order/active id.
- Fuller main tabs: open/switch/close, drag reorder, overflow scroll with edge fade; stub page content only.
- Overlay titlebar in-app (custom drag region, tabs and toggles in the top band). Content columns pad below titlebar height.
- Click/toggle buttons only; no keyboard shortcuts this slice.
- When left/right collapse or bottom closes, the **center main surface expands into the freed space** (flex). Sides never leave empty gutters; center is the primary surface, not the side panels.

## Architecture

### Routing

- Keep `ActiveScreen::Home` as the post-onboarding route.
- Enter OpenCore still completes onboarding, resizes to 1280×800, and renders shell instead of `home_screen`.
- Boot with `onboarding_completed` opens shell directly at home window size.

### Module

New `src/app/shell/` owning an `Entity<Shell>`:

| Piece | Responsibility |
| --- | --- |
| `Shell` | Live chrome state, tweens, tab model, panel open/size, render tree |
| Nested prefs chrome | Persisted sizes, open flags, main tabs |
| Overlay titlebar | Drag region, panel toggles, main tab strip |
| Regions | Left \| center \| right; bottom drawer inside the center column |

`OpenCoreApp` holds `Option<Entity<Shell>>`, created when Home is active (including boot-to-Home). Theme continues to come from existing `OpenCoreTheme` / preferences.

Remove or retire `src/app/home/` Hello World as the Home render target. Optional: one default stub tab page can reuse a tiny placeholder view inside shell.

### Region roles

| Region | Role |
| --- | --- |
| Center | Primary main surface + tab content; `flex_1` absorbs space when sides/bottom collapse |
| Left / right | Collapsible secondary panels |
| Bottom | Height drawer inside the center column only |
| Overlay titlebar | Drag + toggles + main tab strip |

### Out of scope

- Keyboard shortcuts for panel toggles
- Right-pane surface tabs / takeover
- Real editor, chat, or terminal content
- Packaging / signing changes

## Interaction behaviors

### Panels

- Toggle open/close via titlebar buttons.
- Open/close: ~200ms ease-out width/height tween; clip inner content during tween so mid-animation reflow does not distort content.
- Drag resize: live tracking (no tween); clears in-flight tween; clamps to min/max (bottom also capped by viewport height fraction).
- Double-click resize handle resets that dimension to default.
- Collapsed side target width = `0`; collapsed bottom target height = `0`; center expands into freed space.
- Reduced motion: snap to target (no tween travel).
- Press feedback on toggles: brief scale-down feel (~0.97) with ease-out under ~200ms where the element API allows.

### Main tabs (center only)

- Strip lives in the overlay titlebar over the center band.
- Open / switch / close; drag reorder; horizontal overflow with scroll + edge fade.
- Closing the active tab selects a neighbor.
- Keep at least one tab (never empty the strip to zero tabs).
- Tab list + active id persist with chrome.
- Chip press feedback matches toggle polish; visible focus on interactive chrome.

### Focus

- Interactive chrome is keyboard-focusable where practical.
- Panels do not steal center focus unless the user clicks into them.
- Titlebar controls must not break window drag (occlude / prevent_default on mousedown as needed).

## State, persistence, errors

### Source of truth

- Live interaction state on `Entity<Shell>` (sizes, open flags, tweens, tab order/active).
- Durable copy in `AppPreferences` as a nested `shell` chrome blob with serde `default` so older preference files still load.

### Persisted fields

- Left/right widths and open/collapsed
- Bottom height and open
- Main tabs: ordered ids/titles (stubs), active tab id

### Save path

- Debounced write (~400ms) through existing `PreferencesStore` on `OpenCoreApp` (atomic tmp + rename).
- Shell asks the app to merge chrome into preferences and save; theme and onboarding fields stay untouched.

### Boot

- Home path constructs `Shell` from persisted chrome, clamped to mins/maxes and current window bounds.
- Dev reset clears shell chrome with the rest of preferences.

### Errors

- Save failures keep in-memory chrome; report via existing persistence-error path on `OpenCoreApp` when available, otherwise log. UI stays interactive.
- Corrupt preferences: existing reset-to-defaults applies; shell uses chrome defaults.

## Default chrome (first run after onboarding)

- Left open at default width; right closed; bottom closed.
- One stub main tab active (e.g. “Welcome” or “Untitled”).
- Exact pixel defaults chosen at implementation time within sensible min/max clamps.

## Testing & verification

### Automated

- Clamp helpers for left/right width and bottom height.
- Tab model: switch, close (neighbor selection), reorder, at-least-one-tab policy.
- Preferences: `shell` chrome JSON round-trip and missing-field defaults.
- Home route renders `Shell` after onboarding complete (or boot with `onboarding_completed`).
- Toggle left/right/bottom updates open flags and schedules persist (in-memory store).

### Manual QA before done

- Launch → Enter OpenCore → shell replaces Hello World.
- Collapse/expand each panel: center expands into freed space; tween + reduced-motion snap.
- Drag and double-click-reset handles.
- Main tabs: open/switch/close/reorder + overflow scroll/fade.
- Restart: chrome + tabs restore.
- Light/dark and window resize.

## Success criteria

- Post-onboarding Home is the shell, not Hello World.
- Center is visibly the main surface and reclaim space when chrome collapses.
- Panel and tab interactions feel responsive (press feedback, ease-out toggles, live drag).
- Full chrome state survives restart.
- Stub content only; no claim of real product surfaces yet.
