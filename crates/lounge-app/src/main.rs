mod app;
mod config;
mod emoji;
mod http_client;
mod text_input;
mod theme;

use std::borrow::Cow;
use std::sync::Arc;

use gpui::{
    App, AppContext, Application, AssetSource, Bounds, Result, SharedString, TitlebarOptions,
    WindowBounds, WindowOptions, point, px, size,
};

use app::LoungeApp;

struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes = match path {
            "icons/at-sign.svg" => include_bytes!("../assets/icons/at-sign.svg").as_slice(),
            "icons/chevron-left.svg" => {
                include_bytes!("../assets/icons/chevron-left.svg").as_slice()
            }
            "icons/log-out.svg" => include_bytes!("../assets/icons/log-out.svg").as_slice(),
            "icons/menu.svg" => include_bytes!("../assets/icons/menu.svg").as_slice(),
            "icons/search.svg" => include_bytes!("../assets/icons/search.svg").as_slice(),
            "icons/users.svg" => include_bytes!("../assets/icons/users.svg").as_slice(),
            "icons/volume-2.svg" => include_bytes!("../assets/icons/volume-2.svg").as_slice(),
            "icons/volume-x.svg" => include_bytes!("../assets/icons/volume-x.svg").as_slice(),
            _ => return Ok(None),
        };
        Ok(Some(Cow::Borrowed(bytes)))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

fn main() {
    Application::new()
        .with_assets(Assets)
        .with_http_client(Arc::new(http_client::BlockingHttpClient))
        .run(|cx: &mut App| {
            text_input::bind_keys(cx);
            let config = config::AppConfig::load();
            let bounds = match (
                config.window_x,
                config.window_y,
                config.window_width,
                config.window_height,
            ) {
                (Some(x), Some(y), Some(width), Some(height)) => WindowBounds::Windowed(
                    Bounds::new(point(px(x), px(y)), size(px(width), px(height))),
                ),
                _ => WindowBounds::centered(size(px(1150.), px(780.)), cx),
            };
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(bounds),
                        titlebar: Some(TitlebarOptions {
                            title: Some("Lounge Native".into()),
                            ..Default::default()
                        }),
                        app_id: Some("lounge-native".into()),
                        window_min_size: Some(size(px(760.), px(480.))),
                        ..Default::default()
                    },
                    |_, cx| cx.new(LoungeApp::new),
                )
                .expect("failed to open window");
            cx.activate(true);
            cx.on_window_closed(|cx| cx.quit()).detach();
            let _ = window;
        });
}
