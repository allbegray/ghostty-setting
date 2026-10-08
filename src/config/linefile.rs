//! Line-preserving Ghostty config file.
//!
//! Ghostty's config is a flat `key = value` text file where `#` starts a
//! comment only when it appears *before* any `=` (a `#` in a value, as in
//! `background = #123abc`, is part of the value, not a comment).
//!
//! Users keep comments, grouping and ordering in this file by hand. A
//! naive parse-and-regenerate would silently destroy all of that, so this
//! module stores the exact original text of every line and rewrites only
//! the value portion of the lines it actually touches.

/// One physical line of the file, with its `key = value` parse if it has one.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The line exactly as it appeared in the file, without any trailing newline.
    pub raw: String,
    /// `Some((key, value))` for a `key = value` line.
    pub kv: Option<(String, String)>,
    /// Everything before and including the `=`, e.g. `font-size =`.
    /// Only meaningful when `kv` is `Some`.
    pub head: String,
    /// Whitespace between `=` and the value.
    pub lead: String,
    /// Whitespace after the value (usually none, or indentation of nothing).
    pub trail: String,
}

impl Line {
    /// Rewrite this line with a new value, keeping `head`/`lead`/`trail`
    /// (the user's spacing) intact, and refreshing the cached parse.
    fn set_value(&mut self, value: &str) {
        self.raw = format!(
            "{}{}{}{}",
            self.head,
            self.lead,
            encode_value(value),
            self.trail
        );
        if let Some(key) = self.kv.as_ref().map(|(k, _)| k.clone()) {
            self.kv = Some((key, value.to_string()));
        }
    }
}

/// A parsed Ghostty config file that remembers how it was written.
#[derive(Debug, Clone, PartialEq)]
pub struct LineFile {
    pub lines: Vec<Line>,
    /// Whether the source text ended with a newline.
    pub final_newline: bool,
    /// Detected line terminator, so a CRLF file is not silently rewritten as LF.
    pub line_ending: String,
}

/// Split `raw` into `(head, lead, value, trail)` if it is a `key = value` line.
fn split_kv(raw: &str) -> Option<(String, String, String, String, String)> {
    // Comments and blank lines are not key/value pairs.
    let trimmed = raw.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let eq = raw.find('=')?;
    let key = raw[..eq].trim();
    // Ghostty keys are always lowercase with hyphens; reject nonsense so we
    // never accidentally treat prose as a config key.
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return None;
    }
    let head = raw[..=eq].to_string();
    let rest = &raw[eq + 1..];
    let value_start = rest.len() - rest.trim_start().len();
    // A value that is nothing but whitespace has no span to take: `trim_end`
    // would put the end before the start. Every character is padding then.
    let value_end = rest.trim_end().len().max(value_start);
    let lead = rest[..value_start].to_string();
    let trail = rest[value_end..].to_string();
    let value = unquote(&rest[value_start..value_end]);
    Some((key.to_string(), value, head, lead, trail))
}

/// Strip one layer of double quoting, unescaping `\\`, `\"`, `\n` and `\t`
/// (the same escapes [`encode_value`] writes, and the ones Zig string
/// literals use).
fn unquote(v: &str) -> String {
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        let inner = &v[1..v.len() - 1];
        let mut out = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(other) => out.push(other),
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        out
    } else {
        v.to_string()
    }
}

/// Quote a value only when Ghostty would otherwise read it differently.
///
/// Surrounding whitespace is significant, and a bare `"` inside would
/// terminate the string early, so those force quoting.
fn encode_value(v: &str) -> String {
    // An empty value is meaningful as-is: Ghostty reads `key =` as "reset to
    // default", so it must stay unquoted.
    let needs_quotes = v != v.trim()
        || v.contains('"')
        || v.contains('\\')
        || v.contains('\n')
        || v.contains('\t');
    if !needs_quotes {
        return v.to_string();
    }
    let mut out = String::with_capacity(v.len() + 2);
    out.push('"');
    for c in v.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

impl LineFile {
    pub fn parse(text: &str) -> Self {
        let final_newline = text.ends_with('\n') || text.is_empty();
        // `lines()` drops the trailing empty segment produced by a final newline.
        let lines = text
            .lines()
            .map(|raw| {
                let parts = split_kv(raw);
                let kv = parts
                    .as_ref()
                    .map(|(k, v, _, _, _)| (k.clone(), v.clone()));
                let (head, lead, trail) = match &parts {
                    Some((_, _, h, l, t)) => (h.clone(), l.clone(), t.clone()),
                    None => (String::new(), String::new(), String::new()),
                };
                Line {
                    raw: raw.to_string(),
                    kv,
                    head,
                    lead,
                    trail,
                }
            })
            .collect();
        LineFile {
            lines,
            final_newline,
            line_ending: if text.contains("\r\n") {
                "\r\n".to_string()
            } else {
                "\n".to_string()
            },
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out.push_str(&line.raw);
            out.push_str(&self.line_ending);
        }
        if !self.final_newline && out.ends_with(&self.line_ending) {
            out.truncate(out.len() - self.line_ending.len());
        }
        out
    }

    /// All raw values for `key`, in file order.
    pub fn get_all(&self, key: &str) -> Vec<String> {
        self.lines
            .iter()
            .filter_map(|l| match &l.kv {
                Some((k, v)) if k == key => Some(v.clone()),
                _ => None,
            })
            .collect()
    }

    /// The effective value for a single-valued key (the last line wins).
    pub fn get(&self, key: &str) -> Option<String> {
        self.get_all(key).pop()
    }

    /// Indices of every line carrying `key`.
    fn indices_of(&self, key: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match &l.kv {
                Some((k, _)) if k == key => Some(i),
                _ => None,
            })
            .collect()
    }

    /// Set a single-valued key, editing the last existing line (or appending).
    pub fn set(&mut self, key: &str, value: &str) {
        match self.indices_of(key).pop() {
            Some(i) => self.lines[i].set_value(value),
            None => self.append(key, value),
        }
    }

    /// Set a repeatable key's full list of values, keeping existing lines in place.
    ///
    /// Grows by inserting after the last existing line, shrinks by deleting
    /// surplus lines from the end of the group. Lines the user grouped
    /// elsewhere in the file keep their position.
    #[allow(dead_code)]
    pub fn set_all(&mut self, key: &str, values: &[String]) {
        let idxs = self.indices_of(key);

        if idxs.is_empty() {
            for v in values {
                self.append(key, v);
            }
            return;
        }

        let common = idxs.len().min(values.len());
        for (n, &i) in idxs.iter().take(common).enumerate() {
            self.lines[i].set_value(&values[n]);
        }

        if values.len() > idxs.len() {
            let mut insert_at = idxs.last().unwrap() + 1;
            for v in &values[idxs.len()..] {
                self.lines.insert(insert_at, Self::fresh_line(key, v));
                insert_at += 1;
            }
        } else if values.len() < idxs.len() {
            // Delete from the tail of the group so earlier entries stay put.
            for &i in idxs[values.len()..].iter().rev() {
                self.lines.remove(i);
            }
        }
    }

    /// Remove every line for `key`.
    pub fn remove(&mut self, key: &str) {
        self.lines.retain(|l| match &l.kv {
            Some((k, _)) => k != key,
            None => true,
        });
    }

    fn append(&mut self, key: &str, value: &str) {
        self.lines.push(Self::fresh_line(key, value));
        self.final_newline = true;
    }

    fn fresh_line(key: &str, value: &str) -> Line {
        let raw = format!("{} = {}", key, encode_value(value));
        Line {
            raw,
            kv: Some((key.to_string(), value.to_string())),
            head: format!("{} =", key),
            lead: " ".to_string(),
            trail: String::new(),
        }
    }

    /// Every distinct key present in the file, in first-seen order.
    #[cfg(test)]
    pub fn keys(&self) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for l in &self.lines {
            if let Some((k, _)) = &l.kv {
                if !seen.iter().any(|s| s == k) {
                    seen.push(k.clone());
                }
            }
        }
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The central promise: a file that is read and written back untouched
    /// must be byte-identical, comments and all.
    fn assert_round_trip(text: &str) {
        let f = LineFile::parse(text);
        assert_eq!(f.render(), text, "round trip changed the file");
    }

    #[test]
    fn untouched_file_round_trips_byte_for_byte() {
        assert_round_trip("");
        assert_round_trip("# hello\n");
        assert_round_trip("font-size = 13\n");
        assert_round_trip("font-size = 13"); // no trailing newline
        assert_round_trip("\n\nfont-size = 13\n\n\n");
        assert_round_trip("  font-size =13  \n");
        assert_round_trip("font-size=13\ntheme=rose-pine\n");
        assert_round_trip("font-size = 13\r\ntheme = dark\r\n"); // CRLF
    }

    #[test]
    fn comments_and_blanks_survive_a_value_edit() {
        let text = "\
# My Ghostty config
# written by hand, please keep me

font-family = Iosevka

# why: 13 is the sweet spot
font-size = 13

# vim: ft=ghostty
";
        let mut f = LineFile::parse(text);
        f.set("font-size", "15");

        let out = f.render();
        assert_eq!(
            out,
            "\
# My Ghostty config
# written by hand, please keep me

font-family = Iosevka

# why: 13 is the sweet spot
font-size = 15

# vim: ft=ghostty
"
        );
    }

    #[test]
    fn editing_a_value_keeps_the_users_spacing() {
        let mut f = LineFile::parse("font-size=13\nkey   =   value\n");
        f.set("font-size", "20");
        f.set("key", "other");
        assert_eq!(f.render(), "font-size=20\nkey   =   other\n");
    }

    #[test]
    fn a_hash_in_a_value_is_not_a_comment() {
        // Ghostty documents `background = #123abc` as a color, not a comment.
        let mut f = LineFile::parse("background = #123abc\n");
        assert_eq!(f.get("background").as_deref(), Some("#123abc"));
        assert!(f.lines[0].raw.starts_with("background"));
        // Re-writing the same value must not mangle it.
        f.set("background", "#123abc");
        assert_eq!(f.render(), "background = #123abc\n");
    }

    #[test]
    fn hash_after_equals_never_creates_a_comment_line() {
        let f = LineFile::parse("background = #123abc\n# real comment\n");
        assert_eq!(f.lines.len(), 2);
        assert!(f.lines[0].kv.is_some());
        assert!(f.lines[1].kv.is_none(), "`# real comment` must stay a comment");
    }

    #[test]
    fn setting_a_new_key_appends_it() {
        let mut f = LineFile::parse("# keep me\nfont-size = 13\n");
        f.set("theme", "rose-pine");
        assert_eq!(f.render(), "# keep me\nfont-size = 13\ntheme = rose-pine\n");
        assert_eq!(f.get("theme").as_deref(), Some("rose-pine"));
    }

    #[test]
    fn get_returns_the_last_occurrence() {
        // Ghostty lets later lines override earlier ones.
        let f = LineFile::parse("font-size = 12\nfont-size = 14\n");
        assert_eq!(f.get("font-size").as_deref(), Some("14"));
    }

    #[test]
    fn set_edits_the_line_that_actually_wins() {
        let mut f = LineFile::parse("font-size = 12\nfont-size = 14\n");
        f.set("font-size", "16");
        assert_eq!(f.render(), "font-size = 12\nfont-size = 16\n");
        // The cached parse must not go stale, or the UI would read the old value.
        assert_eq!(f.get("font-size").as_deref(), Some("16"));
    }

    #[test]
    fn remove_drops_every_occurrence() {
        let mut f = LineFile::parse("font-size = 12\ntheme = x\nfont-size = 14\n");
        f.remove("font-size");
        assert_eq!(f.render(), "theme = x\n");
        assert_eq!(f.get("font-size"), None);
    }

    #[test]
    fn repeatable_values_keep_their_positions() {
        let mut f = LineFile::parse(
            "# fonts\nfont-family = A\nfont-family = B\n\ntheme = x\n",
        );
        f.set_all(
            "font-family",
            &["A".into(), "C".into(), "D".into()],
        );
        assert_eq!(
            f.render(),
            "# fonts\nfont-family = A\nfont-family = C\nfont-family = D\n\ntheme = x\n"
        );
    }

    #[test]
    fn shrinking_a_repeatable_list_deletes_from_the_tail() {
        let mut f = LineFile::parse("font-family = A\nfont-family = B\nfont-family = C\n");
        f.set_all("font-family", &["A".into(), "B".into()]);
        assert_eq!(f.render(), "font-family = A\nfont-family = B\n");
        // An empty list clears the key entirely.
        f.set_all("font-family", &[]);
        assert_eq!(f.render(), "");
    }

    #[test]
    fn repeatable_inserts_land_after_the_last_existing_line() {
        let mut f = LineFile::parse("font-family = A\ntheme = x\n");
        f.set_all("font-family", &["A".into(), "B".into()]);
        assert_eq!(f.render(), "font-family = A\nfont-family = B\ntheme = x\n");
    }

    #[test]
    fn values_with_spaces_are_quoted_and_readable_back() {
        let mut f = LineFile::parse("");
        f.set("font-family", "JetBrains Mono");
        assert_eq!(f.render(), "font-family = JetBrains Mono\n");
        assert_eq!(f.get("font-family").as_deref(), Some("JetBrains Mono"));
    }

    #[test]
    fn values_needing_quotes_round_trip() {
        for value in [
            "  leading and trailing  ",
            "has \"quotes\" inside",
            "back\\slash",
            "tab\there",
        ] {
            let mut f = LineFile::parse("");
            f.set("title", value);
            assert_eq!(
                f.get("title").as_deref(),
                Some(value),
                "value did not survive quoting: {value:?}"
            );
            // And the written file must still parse back identically.
            assert_eq!(LineFile::parse(&f.render()).get("title").as_deref(), Some(value));
        }
    }

    /// A key whose value is only whitespace used to take a byte range whose
    /// end preceded its start, which panicked on the first frame — the whole
    /// app failed to open for a file containing `font-family = `.
    #[test]
    fn a_whitespace_only_value_is_empty_and_survives_a_round_trip() {
        let text = "font-family = \nfont-size = 14\n";
        let file = LineFile::parse(text);
        assert_eq!(file.get("font-family").as_deref(), Some(""));
        assert_eq!(file.render(), text);
    }

    #[test]
    fn empty_value_means_reset_and_is_not_quoted() {
        let mut f = LineFile::parse("font-family = A\n");
        f.set("font-family", "");
        // `font-family =` is Ghostty's documented "reset to default". The
        // original space before the value is preserved rather than trimmed,
        // because untouched whitespace is this module's whole contract.
        assert_eq!(f.render(), "font-family = \n");
        assert_eq!(f.get("font-family").as_deref(), Some(""));
        assert_eq!(LineFile::parse("font-family =\n").get("font-family").as_deref(), Some(""));
    }

    #[test]
    fn prose_without_an_equals_sign_is_left_alone() {
        let text = "# comment\n\nnot a key value line\nfont-size = 13\n";
        let mut f = LineFile::parse(text);
        assert_eq!(f.render(), text);
        f.set("font-size", "14");
        assert_eq!(
            f.render(),
            "# comment\n\nnot a key value line\nfont-size = 14\n"
        );
    }

    #[test]
    fn keys_with_capitals_or_prose_are_not_mistaken_for_config_keys() {
        // Ghostty keys are lowercase; anything else is not ours to rewrite.
        let f = LineFile::parse("Font-Size = 13\nSome Note = value\n");
        assert!(f.lines.iter().all(|l| l.kv.is_none()));
    }

    #[test]
    fn keys_reports_distinct_keys_in_order() {
        let f = LineFile::parse("b = 1\na = 2\nb = 3\n");
        assert_eq!(f.keys(), vec!["b".to_string(), "a".to_string()]);
    }

    #[test]
    fn a_trailing_comment_after_a_value_is_part_of_the_value() {
        // Documented Ghostty behavior: comments cannot follow a value.
        let f = LineFile::parse("font-size = 13 # not a comment\n");
        assert_eq!(f.get("font-size").as_deref(), Some("13 # not a comment"));
    }
}
