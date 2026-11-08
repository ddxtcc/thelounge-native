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
            "icons/download.svg" => include_bytes!("../assets/icons/download.svg").as_slice(),
            "icons/expand.svg" => include_bytes!("../assets/icons/expand.svg").as_slice(),
            "icons/maximize.svg" => include_bytes!("../assets/icons/maximize.svg").as_slice(),
            "icons/music.svg" => include_bytes!("../assets/icons/music.svg").as_slice(),
            "icons/pause.svg" => include_bytes!("../assets/icons/pause.svg").as_slice(),
            "icons/play.svg" => include_bytes!("../assets/icons/play.svg").as_slice(),
            "icons/x.svg" => include_bytes!("../assets/icons/x.svg").as_slice(),
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
    #[cfg(feature = "media")]
    if let Some(result) = poster_command() {
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    Application::new()
        .with_assets(Assets)
        .with_http_client(Arc::new(http_client::BlockingHttpClient))
        .run(|cx: &mut App| {
            text_input::bind_keys(cx);
            let config = config::AppConfig::load();
            let centered = || WindowBounds::centered(size(px(1150.), px(780.)), cx);
            let bounds = match (
                config.window_x,
                config.window_y,
                config.window_width,
                config.window_height,
            ) {
                (Some(x), Some(y), Some(width), Some(height))
                    if x.is_finite()
                        && y.is_finite()
                        && width.is_finite()
                        && height.is_finite()
                        && width > 0.
                        && height > 0. =>
                {
                    let windowed = Bounds::new(point(px(x), px(y)), size(px(width), px(height)));
                    if cx
                        .displays()
                        .iter()
                        .any(|display| display.bounds().contains(&windowed.center()))
                    {
                        WindowBounds::Windowed(windowed)
                    } else {
                        centered()
                    }
                }
                _ => centered(),
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

#[cfg(feature = "media")]
fn poster_command() -> Option<Result<(), String>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("--poster") {
        return None;
    }
    let url = args.next()?;
    let destination = args.next()?;
    Some(app::extract_poster(&url, &destination))
}
