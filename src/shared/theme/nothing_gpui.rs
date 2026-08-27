use std::rc::Rc;

use gpui::App;
use gpui_component::theme::ThemeMode as ComponentThemeMode;
use gpui_component::{Theme, ThemeSet};

use super::ThemeMode;

const NOTHING_THEME_JSON: &str = include_str!("../../../themes/nothing.json");

pub fn nothing_theme_set() -> ThemeSet {
    serde_json::from_str(NOTHING_THEME_JSON).expect("themes/nothing.json must parse as ThemeSet")
}

pub fn apply_nothing_theme(mode: ThemeMode, cx: &mut App) {
    let set = nothing_theme_set();
    let light = set
        .themes
        .iter()
        .find(|t| !t.mode.is_dark())
        .cloned()
        .expect("Nothing Light theme");
    let dark = set
        .themes
        .iter()
        .find(|t| t.mode.is_dark())
        .cloned()
        .expect("Nothing Dark theme");

    {
        let theme = Theme::global_mut(cx);
        theme.light_theme = Rc::new(light);
        theme.dark_theme = Rc::new(dark);
    }

    let component_mode = match mode {
        ThemeMode::Light => ComponentThemeMode::Light,
        ThemeMode::Dark => ComponentThemeMode::Dark,
    };
    Theme::change(component_mode, None, cx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_component::ThemeMode as ComponentThemeMode;

    #[test]
    fn nothing_json_parses_with_light_and_dark() {
        let set = nothing_theme_set();
        assert_eq!(set.name.as_ref(), "Nothing");
        assert_eq!(set.themes.len(), 2);
        let light = set
            .themes
            .iter()
            .find(|t| !t.mode.is_dark())
            .expect("light");
        let dark = set.themes.iter().find(|t| t.mode.is_dark()).expect("dark");
        assert_eq!(light.radius, Some(0));
        assert_eq!(dark.radius, Some(0));
        assert_eq!(light.shadow, Some(false));
        assert_eq!(dark.shadow, Some(false));
        assert_eq!(light.colors.background.as_deref(), Some("#F5F5F5"));
        assert_eq!(dark.colors.background.as_deref(), Some("#000000"));
        assert_eq!(light.colors.danger.as_deref(), Some("#D71921"));
        assert_eq!(dark.colors.danger.as_deref(), Some("#D71921"));
        let _ = ComponentThemeMode::Dark;
    }

    #[test]
    fn primary_is_inverted_not_sky_blue() {
        let set = nothing_theme_set();
        let dark = set.themes.iter().find(|t| t.mode.is_dark()).unwrap();
        assert_eq!(dark.colors.primary.as_deref(), Some("#FFFFFF"));
        let light = set.themes.iter().find(|t| !t.mode.is_dark()).unwrap();
        assert_eq!(light.colors.primary.as_deref(), Some("#1A1A1A"));
    }
}
