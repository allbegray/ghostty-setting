# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-10-09

The view layer was split into files, but the split cut file boundaries and zero
module interfaces — every section still received the whole view, so each one
could reach state it had nothing to do with, and no section could be replaced
on its own. This release gives those sections real seams, and the same
treatment to four other shallow clusters. The config file format and what any
value writes are unchanged.

### Changed

- **The view layer's sections are modules.** The title bar, navigation sidebar
  and status bar take a snapshot of what they draw plus the actions they can
  ask for, instead of reaching into the view's fields. The render body no
  longer carries a borrow-checker workaround around its own modal state.
- **The list editor's state is a named module.** Its eleven fields moved off
  the modal variant into `ListEditorModal`, so the renderer takes one
  parameter instead of eleven. The key family is derived once and read by
  every arm, replacing four booleans re-tested at twenty-six sites.
- **Control policy has one home.** Chip sets, slider records and narrowed
  ranges live in a controls module keyed by option key, checked against the
  schema by tests, replacing five tables scattered across three files.
- **The editor cache owns its write-back.** Getters return the entity and wire
  their own subscription, so no call site guesses whether it is the first
  frame or which slot to leave standing. Eviction detaches the paired
  subscription.
- **The row's editor takes the file and the cache, not the view.** A
  control's code can no longer reach the view's other state.
- **What a control's change means is one tested decision.** The rules that
  decide a file edit live in a commit module, rather than as a ternary per
  call site.

### Fixed

- **Chip labels no longer lose their English text.** A control-policy move had
  turned them into plain strings, which would have shipped Korean-only copy to
  a user whose picker said English.
- **Switching the language no longer discards typed values.** The retained
  fields have their placeholders re-resolved in place, where they were
  previously dropped and rebuilt.
- **The list modal's input follows a language switch.** Its placeholder is
  re-resolved like the row fields, instead of staying in the previous
  language.
- **The subscription list no longer grows on every language switch.** The
  search field's write-back has one slot that is replaced, not a list that
  accumulates one per switch.

### Added

- Tests for rules that were previously expressions inside window-bound
  closures: trimming and blank-clearing, deduplication, index-guarded removal,
  the chip and flag highlights, the keybinding summary, the keystroke-to-trigger
  spelling, and the toggle round trip that must restore a file byte-for-byte.
- A `GLOSSARY.md` naming the domain and the architecture vocabulary this work
  speaks.

## [0.1.0] - 2026-10-08

First release. A desktop settings editor for Ghostty's config: all 200
documented options in nine categories, edits that preserve the file's lines,
a live terminal preview, and a bilingual Korean/English interface. Ships as a
prebuilt Mac binary via the installer, or builds from source with cargo.

[0.2.0]: https://github.com/allbegray/ghostty-setting/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/allbegray/ghostty-setting/releases/tag/v0.1.0
