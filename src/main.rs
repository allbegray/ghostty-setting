mod app;
mod config;

use app::{FocusSearch, Save, SettingsView};
use gpui_kit::component::TitleBar;
use gpui_kit::{AppContext as _, Bounds, KeyBinding, WindowBounds, WindowOptions, px, size};

fn main() {
    let path = std::env::args().nth(1).map(std::path::PathBuf::from);

    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            // MUST be first, before any component is used.
            gpui_kit::init(cx);

            let bounds = Bounds::centered(None, size(px(1480.0), px(880.0)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    // Physical window boundary: the minimum at which sidebar,
                    // un-wrapped settings lanes, and preview panel all fit.
                    window_min_size: Some(size(px(1180.), px(680.))),
                    ..TitleBar::window_options()
                },
                cx,
                |window, cx| cx.new(|cx| SettingsView::new(window, cx, path.clone())),
            )
            .expect("failed to open window");

            cx.bind_keys([
                KeyBinding::new("cmd-s", Save, Some("Settings")),
                KeyBinding::new("/", FocusSearch, Some("Settings")),
            ]);
        });
}
