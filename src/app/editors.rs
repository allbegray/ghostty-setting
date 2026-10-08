//! Retained editor state for option rows.
//!
//! A row's editing widget stays alive between frames, keyed by option key, so
//! that a slider keeps its drag and a text field keeps its caret. That state
//! used to live in four maps on the view, which meant every value change had to
//! know which maps held its key: the eviction rule was copied into sixteen call
//! sites and drifted — a slider dropped its paired text field while the field
//! dropped the slider, written as two one-way rules 330 lines apart.
//!
//! The cache owns the maps, the one eviction rule, and the write-back. A
//! caller says which editor it wants for a key and gets the entity; whether
//! this is the first frame, which subscription reports the change, and which
//! slot to leave standing are the cache's facts, not things a call site
//! computes. An entry pairs the editor with its subscription, so evicting one
//! drops the other: the listener list does not grow, and a widget that would
//! show a stale value cannot keep writing.

use std::collections::HashMap;

use gpui_kit::component::{
    IndexPath,
    color_picker::{ColorPickerEvent, ColorPickerState},
    input::{InputEvent, InputState},
    searchable_list::SearchableVec,
    select::{SelectEvent, SelectState},
    slider::{SliderEvent, SliderState},
};
use gpui_kit::{AppContext as _, Context, Entity, SharedString, Subscription, Window};

use super::SettingsView;
use crate::app::value;

/// One of the four editor kinds the cache retains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Input,
    Select,
    Color,
    Slider,
}

/// The editor a change came from.
///
/// Dropping the widget that produced a change destroys state the user is still
/// using — a slider mid-drag, an open select, a colour picker's swatch — so the
/// cache is told which one to leave alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kept {
    /// A stateless control changed the value: every editor for the key is stale.
    Nothing,
    Input,
    Select,
    Color,
    Slider,
}

impl Kept {
    /// Whether a change from this editor leaves `slot` standing.
    ///
    /// The whole eviction rule: the editor that produced a change keeps its
    /// state, and every other editor for that key is rebuilt from the file.
    /// Written as a predicate so it can be tested without a window.
    fn keeps(self, slot: Slot) -> bool {
        matches!(
            (self, slot),
            (Kept::Input, Slot::Input)
                | (Kept::Select, Slot::Select)
                | (Kept::Color, Slot::Color)
                | (Kept::Slider, Slot::Slider)
        )
    }
}

/// Every slot the cache retains. The tests walk this list, so a slot added
/// without the rule answering for it fails a test.
#[cfg(test)]
const ALL_SLOTS: [Slot; 4] = [Slot::Input, Slot::Select, Slot::Color, Slot::Slider];

/// A retained editor and its write-back subscription.
///
/// They are one entry because they are one thing: the editor is alive exactly
/// as long as its subscription is attached. Removing the entry drops the
/// subscription, so a widget that would show a stale value stops reporting.
struct Retained<T> {
    editor: Entity<T>,
    _subscription: Subscription,
}

/// Editors retained for the option rows, keyed by option key.
#[derive(Default)]
pub struct EditorCache {
    inputs: HashMap<&'static str, Retained<InputState>>,
    selects: HashMap<&'static str, Retained<SelectState<SearchableVec<SharedString>>>>,
    colors: HashMap<&'static str, Retained<ColorPickerState>>,
    sliders: HashMap<&'static str, Retained<SliderState>>,
}

impl EditorCache {
    /// The text field for `key`, created on first use, with its write-back.
    ///
    /// A field reports its value trimmed, and an empty field means the key is
    /// removed — that distinction is the cache's to make, once.
    pub fn input(
        &mut self,
        key: &'static str,
        seed: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<SettingsView>,
    ) -> Entity<InputState> {
        if let Some(entry) = self.inputs.get(key) {
            return entry.editor.clone();
        }
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(seed.to_string())
        });
        let subscription = cx.subscribe_in(&editor, window, move |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                let value = state.read(cx).value().trim().to_string();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(value.as_str())
                };
                this.commit(key, value, Kept::Input, cx);
            }
        });
        self.inputs.insert(
            key,
            Retained {
                editor: editor.clone(),
                _subscription: subscription,
            },
        );
        editor
    }

    /// The numeric field for `key`, created on first use.
    ///
    /// It lives in the same map as the plain text field — both are text inputs
    /// and a value change invalidates them together — but it carries the bounds
    /// and step its control needs. Its write-back is the same rule.
    pub fn number_input(
        &mut self,
        key: &'static str,
        seed: &str,
        min: f64,
        max: f64,
        step: f64,
        window: &mut Window,
        cx: &mut Context<SettingsView>,
    ) -> Entity<InputState> {
        if let Some(entry) = self.inputs.get(key) {
            return entry.editor.clone();
        }
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(seed.to_string())
                .min(min)
                .max(max)
                .step(step)
        });
        let subscription = cx.subscribe_in(&editor, window, move |this, state, event, _, cx| {
            if matches!(event, InputEvent::Change) {
                let value = state.read(cx).value().trim().to_string();
                let value = if value.is_empty() {
                    None
                } else {
                    Some(value.as_str())
                };
                this.commit(key, value, Kept::Input, cx);
            }
        });
        self.inputs.insert(
            key,
            Retained {
                editor: editor.clone(),
                _subscription: subscription,
            },
        );
        editor
    }

    /// The dropdown for `key`, created on first use.
    ///
    /// A dropdown reports the value the user confirmed.
    pub fn select(
        &mut self,
        key: &'static str,
        items: Vec<SharedString>,
        selected: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<SettingsView>,
    ) -> Entity<SelectState<SearchableVec<SharedString>>> {
        if let Some(entry) = self.selects.get(key) {
            return entry.editor.clone();
        }
        let editor = cx.new(|cx| SelectState::new(SearchableVec::new(items), selected, window, cx));
        let subscription = cx.subscribe(&editor, move |this, _, event, cx| {
            let SelectEvent::Confirm(value) = event;
            this.commit(key, value.as_ref().map(|v| v.as_ref()), Kept::Select, cx);
        });
        self.selects.insert(
            key,
            Retained {
                editor: editor.clone(),
                _subscription: subscription,
            },
        );
        editor
    }

    /// The colour picker for `key`, created on first use.
    ///
    /// A picker reports the colour the user settled on, as the hex the file
    /// holds.
    pub fn color(
        &mut self,
        key: &'static str,
        value: Option<gpui_kit::Hsla>,
        window: &mut Window,
        cx: &mut Context<SettingsView>,
    ) -> Entity<ColorPickerState> {
        if let Some(entry) = self.colors.get(key) {
            return entry.editor.clone();
        }
        let editor = cx.new(|cx| {
            let mut state = ColorPickerState::new(window, cx);
            if let Some(value) = value {
                state.set_value(value, window, cx);
            }
            state
        });
        let subscription = cx.subscribe(&editor, move |this, _, event, cx| {
            if let ColorPickerEvent::Change(Some(color)) = event {
                this.commit(key, Some(&value::hex(*color)), Kept::Color, cx);
            }
        });
        self.colors.insert(
            key,
            Retained {
                editor: editor.clone(),
                _subscription: subscription,
            },
        );
        editor
    }

    /// The slider for `key`, created on first use.
    ///
    /// `decimal` is whether a change writes its fractional part: an opacity
    /// keeps `0.85`, a cell width keeps whole numbers.
    pub fn slider(
        &mut self,
        key: &'static str,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        decimal: bool,
        cx: &mut Context<SettingsView>,
    ) -> Entity<SliderState> {
        if let Some(entry) = self.sliders.get(key) {
            return entry.editor.clone();
        }
        let editor = cx.new(|_| {
            SliderState::new()
                .min(min)
                .max(max)
                .step(step)
                .default_value(value)
        });
        let subscription = cx.subscribe(&editor, move |this, _, event, cx| {
            let (SliderEvent::Change(val) | SliderEvent::Release(val)) = event;
            let new_val = if decimal {
                format!("{:.2}", val.start())
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            } else {
                format!("{}", val.start().round() as i64)
            };
            this.commit(key, Some(&new_val), Kept::Slider, cx);
        });
        self.sliders.insert(
            key,
            Retained {
                editor: editor.clone(),
                _subscription: subscription,
            },
        );
        editor
    }

    /// Discard the editors for `key` that would show a stale value.
    ///
    /// `kept` is the editor that produced the change; every other editor for
    /// the key is rebuilt from the file on the next frame.
    pub fn invalidate(&mut self, key: &str, kept: Kept) {
        if !kept.keeps(Slot::Input) {
            self.inputs.remove(key);
        }
        if !kept.keeps(Slot::Select) {
            self.selects.remove(key);
        }
        if !kept.keeps(Slot::Color) {
            self.colors.remove(key);
        }
        if !kept.keeps(Slot::Slider) {
            self.sliders.remove(key);
        }
    }

    /// Discard every editor. Used when the whole file is replaced.
    pub fn forget_all(&mut self) {
        self.inputs.clear();
        self.selects.clear();
        self.colors.clear();
        self.sliders.clear();
    }

    /// Discard only the text fields, whose placeholders carry localized copy.
    pub fn forget_inputs(&mut self) {
        self.inputs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The invariant the whole module exists for: a change drops every editor
    /// for its key except the one that produced it.
    #[test]
    fn a_change_keeps_only_the_editor_that_made_it() {
        for kept in [Kept::Input, Kept::Select, Kept::Color, Kept::Slider] {
            let surviving: Vec<Slot> = ALL_SLOTS
                .into_iter()
                .filter(|slot| kept.keeps(*slot))
                .collect();
            assert_eq!(
                surviving.len(),
                1,
                "{kept:?} must keep exactly its own kind, kept {surviving:?}"
            );
        }
    }

    /// A chip, a switch or a reset changed the value, so nothing it wrote is
    /// still on screen: every editor for the key is stale.
    #[test]
    fn a_stateless_control_keeps_nothing() {
        for slot in ALL_SLOTS {
            assert!(!Kept::Nothing.keeps(slot), "Nothing must drop {slot:?}");
        }
    }
}
