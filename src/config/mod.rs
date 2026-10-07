pub mod linefile;
pub mod schema;

use std::path::PathBuf;

pub use schema::{Kind, Opt};

/// A config file we found on disk, in Ghostty's precedence order (highest first).
pub struct Candidate {
    pub path: PathBuf,
    pub exists: bool,
}

fn xdg_config_home() -> PathBuf {
    if let Ok(v) = std::env::var("XDG_CONFIG_HOME") {
        if !v.is_empty() {
            return PathBuf::from(v);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".into());
    PathBuf::from(home).join(".config")
}

/// Config file locations, most significant first.
///
/// Ghostty reads `config.ghostty` (1.2.3+) in preference to `config`, and
/// loads macOS Application Support files after XDG ones — so the last file
/// loaded wins. We list them highest-precedence first because that is the
/// one whose values a user's edits most need to beat.
pub fn candidates() -> Vec<Candidate> {
    let mut out = Vec::new();

    let macos_dir = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("Library/Application Support/com.mitchellh.ghostty"));
    if let Ok(dir) = macos_dir {
        for name in ["config.ghostty", "config"] {
            let path = dir.join(name);
            out.push(Candidate {
                exists: path.is_file(),
                path,
            });
        }
    }

    let xdg = xdg_config_home().join("ghostty");
    for name in ["config.ghostty", "config"] {
        let path = xdg.join(name);
        out.push(Candidate {
            exists: path.is_file(),
            path,
        });
    }

    out
}

/// The file to open by default: the highest-precedence one that exists,
/// falling back to a new `config` under XDG.
pub fn default_path() -> PathBuf {
    let cands = candidates();
    cands
        .iter()
        .find(|c| c.exists)
        .map(|c| c.path.clone())
        .unwrap_or_else(|| xdg_config_home().join("ghostty/config"))
}
