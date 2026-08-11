use std::borrow::Cow;

use gpui::{App, AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/fonts"]
struct FontAssets;

pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(rest) = path.strip_prefix("fonts/") {
            return Ok(FontAssets::get(rest).map(|f| f.data));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path == "fonts" || path == "fonts/" {
            return Ok(FontAssets::iter()
                .map(|p| format!("fonts/{p}").into())
                .collect());
        }
        gpui_component_assets::Assets.list(path)
    }
}

impl AppAssets {
    pub fn load_fonts(&self, cx: &App) -> anyhow::Result<()> {
        let mut fonts = Vec::new();
        for path in FontAssets::iter() {
            if path.ends_with(".ttf") {
                let file = FontAssets::get(&path).expect("listed font exists");
                fonts.push(file.data);
            }
        }
        cx.text_system().add_fonts(fonts)?;
        Ok(())
    }
}
