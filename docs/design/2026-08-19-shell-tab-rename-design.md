# Shell tab rename (inline)

Date: 2026-08-19  
Branch: `feat/shell-holy-grail`  
Status: approved for planning

## Goal

Let the user rename a shell tab by double-clicking it. The chip should size to its title, and the close control should stay out of the way until hover.

## Interaction

- Double-click a tab chip (not the close button) enters inline rename.
- If the tab was inactive, select it first, then enter rename.
- While renaming:
  - **Enter** commits
  - **Escape** cancels (restore previous title)
  - **Blur / click away** commits
- On commit: trim whitespace. If the result is empty, keep the previous title.
- Persist the new title through the existing shell-chrome save path.
- Single-click still selects; drag reorder, middle-click close, and titlebar chrome behavior stay as today.
- Double-click must not start a tab drag or trigger titlebar maximize.

## Sizing

- Remove fixed tab chip width. Chip width follows the title (plus close affordance + padding/gap).
- Soft floor so short titles still form a usable hit target (enough for padding and the close control when shown).
- Soft ceiling so extreme titles do not dominate the strip; truncate only past that ceiling.
- While renaming, the field grows with typed text using the same sizing rules.
- After commit, the chip settles to the committed title width.

## Chrome

- Close (`×`) is hidden by default.
- Close appears on tab hover for all tabs (active and inactive).
- While renaming, hide the close button so it does not compete with the text field.

## Architecture

### `TabModel`

Add `rename(id, title)`:

- no-op if `id` is unknown
- trim the incoming title
- if empty after trim, leave the existing title unchanged
- otherwise update that tab’s `title`

### `ShellView`

- Track `renaming_tab_id: Option<String>` and a draft string while editing.
- On double-click (`click_count == 2`): select if needed, set renaming state, focus the field.
- Commit: apply `TabModel::rename`, clear renaming state, `schedule_save` / notify as existing mutations do.
- Cancel: clear renaming state without mutating the title.
- Render path:
  - content-sized chip
  - title label **or** inline focused field when renaming that tab
  - close button only when hovered and not renaming

### Input surface

Prefer an inline focused text field inside the chip (approach 1). Use the project’s existing GPUI / `gpui_component` text-input patterns if available; otherwise a minimal focused editable that supports Enter / Escape / blur.

## Out of scope

- Context-menu rename
- Keyboard-only rename without a prior double-click (e.g. F2) unless added later
- Changing tab identity (`id`); only `title` is editable
- Panel resize / traffic-light / persistence debounce changes unrelated to title save

## Tests

- `TabModel::rename` trims, rejects empty, updates a known id, no-ops an unknown id.
- Commit/cancel rules covered at the model or view-helper level where practical:
  - empty/whitespace commit keeps previous title
  - cancel leaves title unchanged
- Existing tab select / close / reorder tests remain green.
- Prefer targeted unit tests over full window harness unless a small GPUI test is already the local pattern for the touched path.

## Success criteria

- Double-click → inline edit; Enter / blur save; Escape cancels.
- Titles are not clipped by a fixed chip width under normal lengths.
- Close button appears on hover and is hidden while renaming.
- Renamed titles survive app restart via shell chrome preferences.
