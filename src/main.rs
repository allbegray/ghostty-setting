mod app;
mod config;
mod ui;

fn main() -> eframe::Result {
    // An optional path overrides config auto-detection, e.g. for editing a
    // config that lives in a dotfiles repo.
    let path = std::env::args().nth(1).map(std::path::PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 780.0])
            .with_min_inner_size([840.0, 560.0])
            .with_title("Ghostty 설정"),
        ..Default::default()
    };
    eframe::run_native(
        "Ghostty 설정",
        options,
        Box::new(move |cc| Ok(Box::new(app::GhosttyApp::new(cc, path.clone())))),
    )
}
