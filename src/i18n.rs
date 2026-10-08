//! Interface language for the settings window.
//!
//! The active language is ambient application state: one window renders one
//! language at a time and every string is resolved while rendering. Keeping it
//! ambient is what lets a call site read as `Text::new("저장", "Save").s()`
//! instead of threading a `Lang` argument through every render helper.
//!
//! The state is a thread-local rather than a GPUI `Global` because several
//! call sites (static tables, free functions that return plain `&'static str`)
//! have no `App` or `Context` handle in scope. GPUI renders the window on the
//! main thread, which is also the thread that switches the language, so the
//! value the switcher writes is the value the next frame reads.

use std::cell::Cell;

/// A supported interface language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    Ko,
    En,
}

impl Lang {
    /// Every supported language, in switcher order.
    pub const ALL: &'static [Lang] = &[Lang::Ko, Lang::En];

    /// The language's name in that language, for the switcher.
    pub fn native_label(self) -> &'static str {
        match self {
            Lang::Ko => "한국어",
            Lang::En => "English",
        }
    }

    /// Parse the name the language picker displays.
    ///
    /// The picker is a `Select`, whose confirm event carries the chosen item's
    /// display text rather than an index or an id, so this is how a choice maps
    /// back to a language.
    pub fn from_native_label(label: &str) -> Option<Lang> {
        Lang::ALL
            .iter()
            .copied()
            .find(|l| l.native_label() == label)
    }
}

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::Ko) };
}

/// The language that [`Text::s`] resolves against.
pub fn current() -> Lang {
    CURRENT.with(Cell::get)
}

/// Switch the interface language.
///
/// Every surface that shows localized copy must be re-rendered afterwards;
/// [`crate::app::SettingsView`] drops its cached widget state on switch so the
/// new language reaches widgets that captured a string at construction time.
pub fn set(lang: Lang) {
    CURRENT.with(|current| current.set(lang));
}

/// One string in every supported language.
///
/// Fields are private so that a `Text` cannot be read as a plain `&str` by
/// accident: every reader goes through [`Text::get`] or [`Text::s`], which is
/// what keeps a new string from silently shipping in one language only.
#[derive(Clone, Copy, Debug)]
pub struct Text {
    ko: &'static str,
    en: &'static str,
}

impl Text {
    pub const fn new(ko: &'static str, en: &'static str) -> Self {
        Self { ko, en }
    }

    /// Resolve in an explicit language.
    pub fn get(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Ko => self.ko,
            Lang::En => self.en,
        }
    }

    /// Resolve in the active language.
    pub fn s(&self) -> &'static str {
        self.get(current())
    }

    /// Resolve and substitute the template's `{}` placeholders in order.
    ///
    /// Both languages must use the same number of `{}` placeholders; anything
    /// left over after the supplied values are consumed is kept verbatim.
    pub fn fill(&self, values: &[&str]) -> String {
        let mut rest = self.s();
        let mut out = String::with_capacity(rest.len() + 16);
        for value in values {
            match rest.split_once("{}") {
                Some((head, tail)) => {
                    out.push_str(head);
                    out.push_str(value);
                    rest = tail;
                }
                None => break,
            }
        }
        out.push_str(rest);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_resolves_per_language() {
        let text = Text::new("저장", "Save");
        assert_eq!(text.get(Lang::Ko), "저장");
        assert_eq!(text.get(Lang::En), "Save");
    }

    #[test]
    fn ambient_language_drives_s() {
        set(Lang::En);
        assert_eq!(Text::new("저장", "Save").s(), "Save");
        set(Lang::Ko);
        assert_eq!(Text::new("저장", "Save").s(), "저장");
    }

    #[test]
    fn fill_substitutes_every_placeholder() {
        set(Lang::Ko);
        assert_eq!(Text::new("편집 ({}개)", "Edit ({})").fill(&["3"]), "편집 (3개)");
        set(Lang::En);
        assert_eq!(Text::new("편집 ({}개)", "Edit ({})").fill(&["3"]), "Edit (3)");
        set(Lang::Ko);
    }

    /// The picker hands back its item's display text, so that text must map
    /// back to exactly one language.
    #[test]
    fn picker_labels_resolve_to_languages() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_native_label(lang.native_label()), Some(*lang));
        }
        assert_eq!(Lang::from_native_label("Deutsch"), None);
        assert_eq!(Lang::from_native_label("english"), None);
    }
}
