mod app;
mod config;

use app::SettingsView;
use gpui_kit::AppContext as _;
use gpui_kit::component::TitleBar;
use gpui_kit::{Bounds, WindowBounds, WindowOptions, px, size};

fn main() {
    let path = std::env::args().nth(1).map(std::path::PathBuf::from);

    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            // MUST be first, before any component is used.
            gpui_kit::init(cx);

            let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..TitleBar::window_options()
                },
                cx,
                |window, cx| cx.new(|cx| SettingsView::new(window, cx, path.clone())),
            )
            .expect("failed to open window");
        });
}
