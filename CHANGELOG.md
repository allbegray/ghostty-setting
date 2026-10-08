# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- **The view layer's sections are now modules.** The title bar, navigation
  sidebar and status bar take a snapshot of what they draw plus the actions
  they can ask for, instead of reaching into the view's fields. Each is now a
  free function; the option table keeps the view until its own seam lands.
- **The list editor's state is a named module.** Its eleven fields moved off
  the modal variant into `ListEditorModal`, so the renderer takes one
  parameter instead of eleven. The key family is derived once into `ListKind`
  and read by every arm, replacing four booleans re-tested at twenty-six sites.
- **Control policy has one home.** The chip sets, slider records and narrowed
  ranges live in a `controls` module keyed by option key, checked against the
  schema by tests, replacing five tables scattered across the view.

### Fixed

- **Chip labels no longer lose their English text.** The labels are bilingual
  again, so the app answers in the language the picker is set to.
- **Switching the language no longer discards typed values.** The retained
  fields have their placeholders re-resolved in place, rather than the fields
  being dropped and rebuilt.
- **The subscription list no longer grows on every language switch.** The
  search field's write-back has one slot that is replaced, not a vec that
  accumulates.
- **The editor cache owns its write-back.** Getters return the entity and wire
  their own subscription, so no call site guesses whether it is the first frame
  or which slot to leave standing. Eviction detaches the paired subscription.

### Added

- A `commit` module deciding what a control's report means for the file, with
  tests — clearing a field removes the key, a chosen value is written as it
  stands, and a toggle on then off restores the file byte-for-byte.
- A `row_rules` module for the row's display decisions (chip highlight, flag
  highlight, keybinding summary), testable without a window.
- A `list_items` module for the list editor's arithmetic, with tests for
  trimming, blank-skipping, deduplication and index-guarded removal.
