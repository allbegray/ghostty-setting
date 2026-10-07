//! System-font fallback so Korean text renders instead of missing-glyph boxes.
//!
//! egui's bundled fonts only cover Latin and Cyrillic. Rather than shipping a
//! Hangul font (several MB plus licensing questions), we register a
//! system font with [`FontPriority::Lowest`]: it is appended *after* egui's
//! own faces, so Latin keeps coming from the default font and only characters
//! it cannot draw are looked up in ours.

use std::path::PathBuf;

use egui::epaint::text::{FontData, FontInsert, FontPriority, InsertFontFamily};
use egui::FontFamily;

/// Explicit override for machines whose Hangul font lives somewhere we do not
/// probe (e.g. a user-installed Noto Sans KR).
const FONT_ENV: &str = "GHOSTTY_SETTING_FONT";

/// Name used as the key in `FontDefinitions::font_data`; must be unique.
const FONT_NAME: &str = "system-hangul";

/// Candidate paths in preference order.
///
/// Only `.ttf`/`.otf` are listed: egui reads a single face per file, and
/// picking the right face of a `.ttc` collection would mean parsing it first.
fn candidates() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    const PATHS: &[&str] = &[
        // Full Hangul coverage; present on every macOS release.
        "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
        // Wider script coverage (CJK, Thai, …) if we get that far.
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
    ];

    #[cfg(target_os = "linux")]
    const PATHS: &[&str] = &[
        "/usr/share/fonts/truetype/nanum/NanumBarunGothic.ttf",
        "/usr/share/fonts/truetype/nanum/NanumGothic.ttf",
        "/usr/share/fonts/truetype/unfonts/UnBatang.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansKR-Regular.ttf",
        "/usr/share/fonts/opentype/noto/NotoSansCJKkr-Regular.otf",
        "/usr/share/fonts/noto-cjk/NotoSansCJKkr-Regular.otf",
    ];

    #[cfg(target_os = "windows")]
    const PATHS: &[&str] = &[
        // Malgun Gothic.
        r"C:\Windows\Fonts\malgun.ttf",
        r"C:\Windows\Fonts\gulim.ttf",
    ];

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    const PATHS: &[&str] = &[];

    PATHS
}

/// First candidate that actually reads, environment override taking precedence.
fn load_font_bytes() -> Option<Vec<u8>> {
    let mut paths: Vec<PathBuf> = std::env::var_os(FONT_ENV)
        .map(PathBuf::from)
        .into_iter()
        .collect();
    paths.extend(candidates().iter().map(PathBuf::from));

    for path in paths {
        match std::fs::read(&path) {
            Ok(bytes) if !bytes.is_empty() => return Some(bytes),
            // Missing files are expected — the candidate list is platform-wide,
            // so only complain about the explicit override and real I/O errors.
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                eprintln!("ghostty-setting: {} 읽기 실패: {e}", path.display());
            }
            _ => {}
        }
    }
    None
}

/// Register a Hangul-capable system font as the last-resort fallback for both
/// text families. Safe to call once at startup; egui rebuilds its font atlas
/// at the start of the next frame.
pub fn install_fallback(ctx: &egui::Context) {
    let Some(bytes) = load_font_bytes() else {
        eprintln!(
            "ghostty-setting: 한글 폰트를 찾지 못했습니다 — 한국어가 네모로 표시됩니다. \
             {FONT_ENV} 환경변수로 폰트 경로를 지정할 수 있습니다."
        );
        return;
    };

    ctx.add_font(FontInsert::new(
        FONT_NAME,
        FontData::from_owned(bytes),
        [
            FontFamily::Proportional,
            FontFamily::Monospace, // option keys stay monospace; only Hangul falls back
        ]
        .into_iter()
        .map(|family| InsertFontFamily {
            family,
            priority: FontPriority::Lowest,
        })
        .collect(),
    ));
}
