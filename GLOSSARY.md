# GLOSSARY

Domain language for ghostty-setting. One term per concept. If two names exist for
the same thing, one of them is a bug. When the code and this file disagree, the
code is wrong — fix the file or the code, never leave both standing.

## The domain

- **option** — one Ghostty configuration key (200 of them, in `config/schema.rs`),
  carrying its key, label, doc, kind and valid values. Identified by its key
  (e.g. `font-family`).
- **option row** — the row the table draws for one option: label, editing
  control, state badge, reset action.
- **repeatable key** — an option that may appear on several lines
  (`font-family`, `keybind`, `env`, `palette`); the list editor edits these.
- **line file** — the user's config file as `LineFile`: a model whose
  read/set/remove never rewrites a line it did not have to change.
- **category** — one of the nine groups the schema divides options into; the
  sidebar's destinations.
- **interface language** — the UI language (`Lang`): Ko or En.
- **capture** — a widget holding localized copy it took at construction time (a
  placeholder, say), which therefore goes stale when the language switches.

## The architecture

- **section** — one standing part of the window: title bar, nav sidebar, option
  table, status bar, preview panel. A section draws; the view owns state. Until
  its seam lands a section may still arrive as a method on the view — the
  option table and both modals do, because they read the file directly.
- **snapshot** — the plain data a section draws, handed to it as an input struct.
  A section receives a snapshot of the fields it draws, never `&SettingsView`.
- **section message** — a value a section sends back to the view to ask for a
  state change (e.g. "review the changes"). The view is the only thing that owns
  or mutates its fields.
- **editor cache** — the retained per-row editing widgets, keyed by option key
  (`EditorCache`), so a slider keeps its drag and a field keeps its caret.
- **slot** — one editor kind the cache retains: Input, Select, Color, Slider.
- **kept** — what a value change names as its source (`Kept`): the one editor
  that survives eviction, because it already shows the new value.
- **commit path** — the one path an option value changes through: `commit` →
  file set/remove → `settle` → invalidate. No widget or modal writes the file
  directly.
- **active modal** — which modal the window shows: the list editor for a
  repeatable key, the diff viewer, or `None`.
- **list editor** — the modal that edits a repeatable key's items: one modal
  shape per key family, behind one seam.
- **diff viewer** — the modal showing the line diff between the file and its
  last-read state.

## Vocabulary this project does not use

Module, not "component" or "service". Interface, not "API" or "signature".
Seam, not "boundary". A section that needs none of the view's fields takes none
of the view.
