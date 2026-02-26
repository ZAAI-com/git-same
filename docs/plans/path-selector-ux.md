# Setup Path Selector UX Ideas

**Status:** Proposed
**Scope:** Setup wizard (`SelectPath` screen)

## Goal

Reduce friction when choosing a base path during setup, especially for users who do not remember exact directory names.

## Current Friction

- Path entry depends on free typing + tab completion.
- Suggestions are helpful, but users cannot visually browse real folders.
- New users can get stuck on path syntax (`~`, trailing `/`, nested folders).

## Option A (Recommended): Inline Folder Navigator Mode

Add a toggleable browse mode inside the existing `SelectPath` screen.

### Interaction

- `b` opens navigator mode
- `Up`/`Down` selects folder
- `Right` enters selected folder
- `Left` goes to parent folder
- `Enter` selects current folder as base path
- `Esc` exits navigator mode back to typed path mode

### Mockup

```text
  Where should repositories be cloned?
  Repos will be organized as: <path>/<org>/<repo>

  Base Path: ~/Developer

  Browse Folders (Navigator)
  Current: ~/Developer

    > projects/
      clients/
      playground/
      archives/
      .. (parent)

  [Enter] Use Folder  [Left/Right] Open/Back  [Esc] Close
```

### Why this fits now

- Reuses current key model (arrow navigation already standard).
- Keeps existing typed mode and tab completion for power users.
- Minimal architecture impact: can live inside `setup/screens/path.rs` + `setup/handler.rs`.

## Option B: Two-Pane Explorer

Split path screen into left tree (folders) + right preview/details.

### Mockup

```text
  Base Path Picker

  ~/Developer                   Preview
  > projects/                   Final path:
    clients/                    ~/Developer/projects
    playground/                 Clone layout:
    archives/                   ~/Developer/<org>/<repo>
```

### Trade-off

Clearer context, but more rendering complexity and harder to support narrow terminals.

## Option C: Guided Presets + "Browse from here"

Keep suggestions first, but add one action: "Browse from selected suggestion".

### Mockup

```text
  Suggestions:
    > ~/Git-Same/GitHub  (current directory)
      ~/Developer
      ~/Projects
      ~

  [Enter] Use Suggestion  [b] Browse From Suggestion  [Tab] Edit
```

### Trade-off

Very small change, but less flexible than full navigator mode.

## Recommended Rollout

1. Ship Option C first (fast, low risk).
2. Add Option A navigator in next iteration.
3. Keep typed + completion mode permanently for advanced users.

## Implementation Notes

- New `PathInputMode` enum (e.g., `Suggestions | Typing | Browsing`).
- Navigator state fields:
  - `browse_current_dir: String`
  - `browse_entries: Vec<String>`
  - `browse_index: usize`
- Hide dot-folders by default; allow toggle later.
- Always show resulting normalized path in a preview line.
