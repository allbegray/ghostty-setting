use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::config::linefile::LineFile;
use crate::config::schema::{lookup, CATEGORIES, OPTS, Kind, Opt};
use crate::config::default_path;
use crate::ui::color;
use crate::ui::fields;

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Form,
    Raw,
}

struct Notice {
    text: String,
    error: bool,
    at: Instant,
}

impl Notice {
    fn ok(text: impl Into<String>) -> Self {
        Self { text: text.into(), error: false, at: Instant::now() }
    }
    fn error(text: impl Into<String>) -> Self {
        Self { text: text.into(), error: true, at: Instant::now() }
    }
}

fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "linux") {
        "GTK"
    } else {
        ""
    }
}

fn is_foreign(platform: &str) -> bool {
    !platform.is_empty() && platform != current_platform()
}

pub struct GhosttyApp {
    path: PathBuf,
    file: LineFile,
    /// Bytes as loaded from disk (or last saved); drives the dirty flag.
    original: String,
    /// Text bound to the raw editor.
    raw_text: String,
    /// Working copies of repeatable keys, kept across frames so a row the
    /// user just added but has not typed into yet survives.
    lists: HashMap<String, Vec<String>>,

    tab: Tab,
    category: usize,
    search: String,
    hide_foreign: bool,
    notice: Option<Notice>,
    revert_armed: Option<Instant>,
}

impl GhosttyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, path: Option<PathBuf>) -> Self {
        // Must happen before the first frame: egui rebuilds its font atlas at
        // the start of the next pass, so Korean labels render instead of boxes.
        crate::ui::fonts::install_fallback(&cc.egui_ctx);

        let mut app = Self {
            path: path.unwrap_or_else(default_path),
            file: LineFile::parse(""),
            original: String::new(),
            raw_text: String::new(),
            lists: HashMap::new(),
            tab: Tab::Form,
            category: 0,
            search: String::new(),
            hide_foreign: current_platform() == "macOS",
            notice: None,
            revert_armed: None,
        };
        app.load_from_disk();
        app
    }

    fn load_from_disk(&mut self) {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => {
                self.notice = Some(Notice::error(format!("읽기 실패: {e}")));
                String::new()
            }
        };
        self.file = LineFile::parse(&text);
        self.original = text.clone();
        self.raw_text = text;
        self.resync_lists();
    }

    fn resync_lists(&mut self) {
        self.lists.clear();
        for opt in OPTS.iter().filter(|o| o.repeatable()) {
            let vals = self.file.get_all(opt.key);
            if !vals.is_empty() {
                self.lists.insert(opt.key.to_string(), vals);
            }
        }
    }

    fn dirty(&self) -> bool {
        self.file.render() != self.original
    }

    /// Problems worth blocking a save over: a value that Ghostty will reject
    /// because the form was left mid-edit.
    fn validation_errors(&self) -> Vec<String> {
        let mut out = Vec::new();
        for opt in OPTS {
            let Some(raw) = self.file.get(opt.key) else {
                continue;
            };
            let v = raw.trim();
            match opt.kind {
                Kind::Int { .. } => {
                    if v.parse::<i64>().is_err() {
                        out.push(format!("`{}` = {v} — 정수가 아닙니다", opt.key));
                    }
                }
                Kind::Float { .. } => {
                    if v.parse::<f64>().is_err() {
                        out.push(format!("`{}` = {v} — 숫자가 아닙니다", opt.key));
                    }
                }
                Kind::Color { special } => {
                    if color::is_partial_hex(v) && !special.contains(&v) {
                        out.push(format!("`{}` = {v} — 색상값이 완성되지 않았습니다", opt.key));
                    }
                }
                _ => {}
            }
        }
        for entry in self.file.get_all("palette") {
            if let Some((idx, val)) = entry.split_once('=') {
                if color::is_partial_hex(val) {
                    out.push(format!("`palette` = {idx}={val} — 색상값이 완성되지 않았습니다"));
                }
            }
        }
        out
    }

    fn save(&mut self) {
        let errs = self.validation_errors();
        if !errs.is_empty() {
            self.notice = Some(Notice::error(format!(
                "저장하지 않았습니다 · {}",
                errs.join(" / ")
            )));
            return;
        }
        if let Some(parent) = self.path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.notice = Some(Notice::error(format!("디렉터리 생성 실패: {e}")));
                return;
            }
        }
        let rendered = self.file.render();
        match std::fs::write(&self.path, &rendered) {
            Ok(()) => {
                self.original = rendered.clone();
                self.raw_text = rendered;
                self.notice = Some(Notice::ok(format!("저장됨 · {}", self.path.display())));
            }
            Err(e) => self.notice = Some(Notice::error(format!("저장 실패: {e}"))),
        }
    }

    fn revert(&mut self) {
        self.load_from_disk();
        self.revert_armed = None;
        self.notice = Some(Notice::ok("파일 내용을 다시 불러왔습니다"));
    }

    fn is_set(&self, key: &str) -> bool {
        match lookup(key) {
            Some(opt) if opt.repeatable() => {
                self.file.get_all(key).iter().any(|v| !v.is_empty())
            }
            _ => self.file.get(key).is_some(),
        }
    }

    fn path_display(&self) -> String {
        let s = self.path.display().to_string();
        match std::env::var("HOME") {
            Ok(h) if !h.is_empty() && s.starts_with(&h) => format!("~{}", &s[h.len()..]),
            _ => s,
        }
    }
}

impl eframe::App for GhosttyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self
            .notice
            .as_ref()
            .is_some_and(|n| n.at.elapsed() > Duration::from_secs(6))
        {
            self.notice = None;
        }
        if self.revert_armed.is_some_and(|t| t.elapsed() > Duration::from_secs(4)) {
            self.revert_armed = None;
        }

        egui::Panel::top("header").show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::Panel::left("nav")
            .resizable(true)
            .default_size(200.0)
            .min_size(170.0)
            .show(ui, |ui| self.left_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| self.central_panel(ui));
    }
}

impl GhosttyApp {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("Ghostty 설정");
            ui.separator();

            if self.dirty() {
                ui.label(
                    egui::RichText::new("● 저장되지 않은 변경")
                        .color(egui::Color32::from_rgb(230, 170, 50)),
                );
            } else {
                ui.label(egui::RichText::new("○ 변경 없음").weak());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let cmd_s = ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S));
                let dirty = self.dirty();

                if ui
                    .add_enabled(dirty, egui::Button::new("저장 (⌘S)"))
                    .clicked()
                    || (dirty && cmd_s)
                {
                    self.save();
                }

                let armed = self.revert_armed.is_some();
                let mut btn =
                    egui::Button::new(if armed { "정말 되돌릴까요?" } else { "되돌리기" });
                if armed {
                    btn = btn.fill(egui::Color32::from_rgb(140, 55, 55));
                }
                if ui.add_enabled(dirty, btn).clicked() {
                    if armed {
                        self.revert();
                    } else {
                        self.revert_armed = Some(Instant::now());
                    }
                }

                if ui
                    .button("새로고침")
                    .on_hover_text("디스크에서 다시 읽어옵니다")
                    .clicked()
                {
                    self.load_from_disk();
                    self.notice = Some(Notice::ok("다시 불러왔습니다"));
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("파일:").weak());
            let current = self.path.display().to_string();
            let mut chosen = current.clone();
            egui::ComboBox::from_id_salt("file-picker")
                .width(560.0)
                .selected_text(&current)
                .show_ui(ui, |ui| {
                    for c in crate::config::candidates() {
                        let suffix = if c.exists { "" } else { "  (없음 · 새로 생성)" };
                        ui.selectable_value(
                            &mut chosen,
                            c.path.display().to_string(),
                            format!("{}{suffix}", c.path.display()),
                        );
                    }
                });
            if chosen != current {
                if self.dirty() {
                    self.notice =
                        Some(Notice::error("저장하지 않은 변경이 있어 파일을 바꿀 수 없습니다"));
                } else {
                    self.path = PathBuf::from(chosen);
                    self.load_from_disk();
                }
            }
        });
        ui.add_space(6.0);
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("옵션 검색")
                .desired_width(f32::INFINITY),
        );
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, cat) in CATEGORIES.iter().enumerate() {
                let set = cat.keys.iter().filter(|k| self.is_set(k)).count();
                let selected = self.search.is_empty() && self.category == i;
                let text = if set > 0 {
                    format!("{}  ({set})", cat.label)
                } else {
                    cat.label.to_string()
                };
                if ui.selectable_label(selected, text).clicked() {
                    self.category = i;
                    self.search.clear();
                }
            }
            ui.separator();
            ui.checkbox(&mut self.hide_foreign, "다른 플랫폼 옵션 숨기기")
                .on_hover_text(format!("현재 {} 전용 옵션을 가립니다", current_platform()));
        });
        ui.add_space(8.0);
    }

    fn central_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let form = ui.selectable_label(self.tab == Tab::Form, "옵션");
            let raw = ui.selectable_label(self.tab == Tab::Raw, "원본 편집");
            if form.clicked() {
                self.tab = Tab::Form;
            }
            if raw.clicked() {
                self.tab = Tab::Raw;
            }
            ui.separator();
            ui.label(egui::RichText::new(self.path_display()).weak().monospace());
        });
        ui.separator();

        match self.tab {
            Tab::Raw => self.raw_editor(ui),
            Tab::Form => self.form_editor(ui),
        }
    }

    fn raw_editor(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new(
                "Ghostty가 지원하는 모든 옵션을 직접 씁니다. 주석과 순서는 그대로 유지됩니다.",
            )
            .weak(),
        );
        ui.add_space(4.0);
        let resp = ui.add(
            egui::TextEdit::multiline(&mut self.raw_text)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(24),
        );
        if resp.changed() {
            self.file = LineFile::parse(&self.raw_text);
            self.resync_lists();
        }
    }

    fn form_editor(&mut self, ui: &mut egui::Ui) {
        let query = self.search.trim().to_lowercase();
        let category = self.category;
        let hide_foreign = self.hide_foreign;

        let opts: Vec<&'static Opt> = if query.is_empty() {
            CATEGORIES[category]
                .keys
                .iter()
                .filter_map(|k| lookup(k))
                .collect::<Vec<_>>()
        } else {
            OPTS.iter()
                .filter(|o| {
                    o.key.contains(&query)
                        || o.label.to_lowercase().contains(&query)
                        || o.doc.to_lowercase().contains(&query)
                })
                .collect::<Vec<_>>()
        }
        .into_iter()
        .filter(|o| !hide_foreign || !is_foreign(o.platform))
        .collect();

        ui.add_space(4.0);
        if query.is_empty() {
            let cat = &CATEGORIES[category];
            ui.heading(cat.label);
            ui.label(egui::RichText::new(cat.desc).weak());
        } else {
            ui.heading(format!("검색 결과 {}개", opts.len()));
        }
        ui.add_space(6.0);

        if opts.is_empty() {
            ui.label("일치하는 옵션이 없습니다.");
            return;
        }

        // Split the struct so the table can hold &mut file / lists / raw_text
        // at the same time.
        let Self {
            file,
            lists,
            raw_text,
            ..
        } = self;
        let mut changed = false;

        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::exact(212.0))
            .column(Column::remainder().at_least(250.0))
            .column(Column::exact(110.0))
            .header(22.0, |mut header| {
                header.col(|ui| {
                    ui.strong("옵션");
                });
                header.col(|ui| {
                    ui.strong("값");
                });
                header.col(|ui| {
                    ui.strong("상태");
                });
            })
            .body(|mut body| {
                for opt in opts {
                    let h = if opt.repeatable() {
                        let n = lists.get(opt.key).map(|v| v.len()).unwrap_or(0);
                        (n as f32 + 1.0) * 24.0 + 14.0
                    } else {
                        38.0
                    };
                    body.row(h, |mut row| {
                        row.col(|ui| name_cell(ui, opt, hide_foreign));
                        row.col(|ui| {
                            if value_cell(ui, opt, file, lists) {
                                changed = true;
                            }
                        });
                        row.col(|ui| {
                            if status_cell(ui, opt, file) {
                                changed = true;
                            }
                        });
                    });
                }
            });

        if changed {
            *raw_text = file.render();
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match &self.notice {
                Some(n) => {
                    let color = if n.error {
                        egui::Color32::from_rgb(230, 90, 90)
                    } else {
                        egui::Color32::from_rgb(110, 190, 120)
                    };
                    ui.label(egui::RichText::new(&n.text).color(color));
                }
                None => {
                    ui.label(
                        egui::RichText::new(
                            "설정을 저장하면 Ghostty가 자동으로 다시 로드합니다 · 검증: ghostty +validate-config",
                        )
                        .weak(),
                    );
                }
            }
        });
    }
}

fn name_cell(ui: &mut egui::Ui, opt: &Opt, hide_foreign: bool) {
    let mut tooltip = opt.doc.to_string();
    if !opt.platform.is_empty() {
        tooltip.push_str(&format!("\n\n전용 플랫폼: {}", opt.platform));
    }
    ui.add(egui::Label::new(egui::RichText::new(opt.label).strong()).sense(egui::Sense::hover()))
        .on_hover_text(tooltip);
    ui.label(
        egui::RichText::new(format!("`{}`", opt.key))
            .monospace()
            .weak()
            .small(),
    );
    if !opt.platform.is_empty() && !hide_foreign {
        ui.label(
            egui::RichText::new(opt.platform)
                .small()
                .color(egui::Color32::from_rgb(130, 160, 205)),
        );
    }
}

/// Renders the editor for one option and writes the result back to `file`.
/// Returns whether the file changed.
fn value_cell(
    ui: &mut egui::Ui,
    opt: &'static Opt,
    file: &mut LineFile,
    lists: &mut HashMap<String, Vec<String>>,
) -> bool {
    match opt.kind {
        Kind::List => {
            let vals = lists.entry(opt.key.to_string()).or_default();
            if !fields::edit_list(ui, opt, vals) {
                return false;
            }
            let clean: Vec<String> = vals
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if clean.is_empty() {
                file.remove(opt.key);
            } else {
                file.set_all(opt.key, &clean);
            }
            true
        }
        Kind::Flags(_) => {
            let mut cur = file.get(opt.key);
            if !fields::edit_flags(ui, opt, &mut cur) {
                return false;
            }
            match cur {
                Some(v) => file.set(opt.key, &v),
                None => file.remove(opt.key),
            }
            true
        }
        _ => {
            let mut cur = file.get(opt.key);
            if !fields::edit(ui, opt, &mut cur) {
                return false;
            }
            match cur {
                Some(v) => file.set(opt.key, &v),
                None => file.remove(opt.key),
            }
            true
        }
    }
}

/// Badge plus a reset affordance. Returns whether the file changed.
fn status_cell(ui: &mut egui::Ui, opt: &Opt, file: &mut LineFile) -> bool {
    let set = if opt.repeatable() {
        file.get_all(opt.key).iter().any(|v| !v.is_empty())
    } else {
        file.get(opt.key).is_some()
    };

    if set {
        ui.label(
            egui::RichText::new("설정됨")
                .small()
                .color(egui::Color32::from_rgb(110, 190, 120)),
        );
        if ui.small_button("⟲").on_hover_text("설정 파일에서 제거").clicked() {
            file.remove(opt.key);
            return true;
        }
    } else {
        ui.label(egui::RichText::new("기본값").small().weak());
    }
    false
}
