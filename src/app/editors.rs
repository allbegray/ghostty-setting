//! Retained editor state for option rows.
//!
//! A row's editing widget stays alive between frames, keyed by option key, so
//! that a slider keeps its drag and a text field keeps its caret. That state
//! used to live in four maps on the view, which meant every value change had to
//! know which maps held its key: the eviction rule was copied into sixteen call
//! sites and drifted — a slider dropped its paired text field while the field
//! dropped the slider, written as two one-way rules 330 lines apart.
//!
//! The cache owns the maps and the one rule. A caller says what it is, and
//! [`EditorCache::invalidate`] keeps that editor standing while dropping the
//! rest of the key's editors, because the widget that produced a change is
//! already showing the new value.

use std::collections::HashMap;

use gpui_kit::component::{
    IndexPath,
    color_picker::ColorPickerState,
    input::InputState,
    searchable_list::SearchableVec,
    select::SelectState,
    slider::SliderState,
};
use gpui_kit::{App, AppContext as _, Entity, Hsla, SharedString, Window};

/// One of the four editor kinds the cache retains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
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
    pub fn keeps(self, slot: Slot) -> bool {
        matches!(
            (self, slot),
            (Kept::Input, Slot::Input)
                | (Kept::Select, Slot::Select)
                | (Kept::Color, Slot::Color)
                | (Kept::Slider, Slot::Slider)
        )
    }
}

/// Editors retained for the option rows, keyed by option key.
#[derive(Default)]
pub struct EditorCache {
    inputs: HashMap<&'static str, Entity<InputState>>,
    selects: HashMap<&'static str, Entity<SelectState<SearchableVec<SharedString>>>>,
    colors: HashMap<&'static str, Entity<ColorPickerState>>,
    sliders: HashMap<&'static str, Entity<SliderState>>,
}

impl EditorCache {
    /// The text field for `key`, created on first use.
    ///
    /// The flag says whether this call created it, so the caller can attach the
    /// write-back subscription exactly once.
    pub fn input(
        &mut self,
        key: &'static str,
        seed: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> (Entity<InputState>, bool) {
        let created = !self.inputs.contains_key(key);
        let state = self.inputs.entry(key).or_insert_with(|| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(placeholder)
                    .default_value(seed.to_string())
            })
        });
        (state.clone(), created)
    }

    /// The numeric field for `key`, created on first use.
    ///
    /// It lives in the same map as the plain text field — both are text inputs
    /// and a value change invalidates them together — but it carries the bounds
    /// and step its control needs.
    pub fn number_input(
        &mut self,
        key: &'static str,
        seed: &str,
        min: f64,
        max: f64,
        step: f64,
        window: &mut Window,
        cx: &mut App,
    ) -> (Entity<InputState>, bool) {
        let created = !self.inputs.contains_key(key);
        let state = self.inputs.entry(key).or_insert_with(|| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(seed.to_string())
                    .min(min)
                    .max(max)
                    .step(step)
            })
        });
        (state.clone(), created)
    }

    /// The dropdown for `key`, created on first use.
    pub fn select(
        &mut self,
        key: &'static str,
        items: Vec<SharedString>,
        selected: Option<IndexPath>,
        window: &mut Window,
        cx: &mut App,
    ) -> (Entity<SelectState<SearchableVec<SharedString>>>, bool) {
        let created = !self.selects.contains_key(key);
        let state = self.selects.entry(key).or_insert_with(|| {
            cx.new(|cx| SelectState::new(SearchableVec::new(items), selected, window, cx))
        });
        (state.clone(), created)
    }

    /// The colour picker for `key`, created on first use.
    pub fn color(
        &mut self,
        key: &'static str,
        value: Option<Hsla>,
        window: &mut Window,
        cx: &mut App,
    ) -> (Entity<ColorPickerState>, bool) {
        let created = !self.colors.contains_key(key);
        let state = self.colors.entry(key).or_insert_with(|| {
            cx.new(|cx| {
                let mut state = ColorPickerState::new(window, cx);
                if let Some(value) = value {
                    state.set_value(value, window, cx);
                }
                state
            })
        });
        (state.clone(), created)
    }

    /// The slider for `key`, created on first use.
    pub fn slider(
        &mut self,
        key: &'static str,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        cx: &mut App,
    ) -> (Entity<SliderState>, bool) {
        let created = !self.sliders.contains_key(key);
        let state = self.sliders.entry(key).or_insert_with(|| {
            cx.new(|_| {
                SliderState::new()
                    .min(min)
                    .max(max)
                    .step(step)
                    .default_value(value)
            })
        });
        (state.clone(), created)
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

    const ALL_SLOTS: [Slot; 4] = [Slot::Input, Slot::Select, Slot::Color, Slot::Slider];

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
